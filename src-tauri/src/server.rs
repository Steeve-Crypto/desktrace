use crate::capture;
use crate::restore;
use crate::status;
use crate::store::{NewSnapshot, Snapshot, Store};
use crate::tabs::{sanitize_tabs, Tab, MAX_TABS};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use tower_http::cors::{AllowOrigin, CorsLayer};

#[derive(Clone)]
struct AppState {
    store: Store,
}

#[derive(Deserialize)]
struct CaptureIn {
    note: Option<String>,
    include_clipboard: Option<bool>,
    tabs: Option<Vec<Tab>>,
}

#[derive(Deserialize)]
struct TabsIn {
    source: Option<String>,
    browser: Option<String>,
    tabs: Option<Vec<Tab>>,
}

#[derive(Deserialize)]
struct ListQ {
    q: Option<String>,
}

pub fn do_capture(
    store: &Store,
    note: Option<&str>,
    include_clipboard: bool,
    tabs: Option<Vec<Tab>>,
) -> Result<Snapshot, String> {
    let (apps, focused) = capture::list_apps();
    let clip = if include_clipboard {
        capture::read_clipboard()
    } else {
        capture::ClipResult {
            text: None,
            error: None,
        }
    };
    let shot = capture::take_screenshot(&store.shots_dir);
    let attached = if let Some(tabs) = tabs {
        sanitize_tabs(&tabs)
    } else {
        store.load_latest_tabs()
    };
    if let Some(err) = shot.error.as_deref() {
        status::set_error(err);
    }
    if let Some(err) = clip.error.as_deref() {
        status::set_error(err);
    }
    let shot_path = shot.path.to_string_lossy().to_string();
    let id = store.insert(NewSnapshot {
        note,
        focused: focused.as_deref(),
        apps: &apps,
        clipboard: clip.text.as_deref(),
        clipboard_error: clip.error.as_deref(),
        screenshot_path: Some(&shot_path),
        placeholder: shot.placeholder,
        shot_error: shot.error.as_deref(),
        monitors: shot.monitors,
        tabs: &attached,
    })?;
    if shot.placeholder {
        status::set_error(
            shot.error
                .clone()
                .unwrap_or_else(|| "screenshot failed".into()),
        );
    } else if clip.error.is_none() {
        status::set_ok(id);
    }
    store.get(id)?.ok_or_else(|| "insert vanished".into())
}

pub async fn serve(store: Store) {
    let cors = CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            let s = origin.as_bytes();
            s.starts_with(b"chrome-extension://")
                || s == b"http://127.0.0.1:8741"
                || s == b"http://localhost:8741"
        }));

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/stats", get(stats))
        .route("/api/tabs", get(get_tabs).post(put_tabs).delete(clear_tabs))
        .route("/api/snapshots", get(list_snaps).post(capture_http))
        .route("/api/snapshots/:id", get(one).delete(del))
        .route("/api/snapshots/:id/shot", get(shot))
        .route("/api/snapshots/:id/restore-plan", post(restore_plan))
        .route("/api/snapshots/:id/restore", post(restore_run))
        .layer(cors)
        .layer(axum::middleware::from_fn(localhost_only))
        .with_state(AppState { store });

    let addr = SocketAddr::from(([127, 0, 0, 1], 8741));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(err) => {
            eprintln!("DeskTrace loopback already bound or failed: {err}");
            return;
        }
    };
    eprintln!("DeskTrace loopback http://127.0.0.1:8741");
    let _ = axum::serve(listener, app).await;
}

async fn localhost_only(
    headers: HeaderMap,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    if host != "127.0.0.1" && host != "localhost" {
        return (StatusCode::FORBIDDEN, "DeskTrace only listens on localhost").into_response();
    }
    next.run(req).await
}

async fn health(State(st): State<AppState>) -> Json<Value> {
    let tabs = st.store.load_latest_tabs();
    Json(json!({
        "ok": true,
        "product": "DeskTrace",
        "bind": "127.0.0.1",
        "tabs_fresh": !tabs.is_empty(),
        "tab_count": tabs.len()
    }))
}

async fn stats(State(st): State<AppState>) -> Json<Value> {
    Json(st.store.stats())
}

async fn get_tabs(State(st): State<AppState>) -> Json<Value> {
    let tabs = st.store.load_latest_tabs();
    Json(json!({ "tabs": tabs, "fresh": !tabs.is_empty() }))
}

async fn put_tabs(State(st): State<AppState>, Json(body): Json<TabsIn>) -> Json<Value> {
    let tabs = sanitize_tabs(&body.tabs.unwrap_or_default());
    let payload = json!({
        "source": body.source.unwrap_or_else(|| "desktrace-extension".into()),
        "browser": body.browser,
        "received_at": chrono::Utc::now().to_rfc3339(),
        "tabs": tabs,
    });
    let _ = st.store.save_latest_tabs(&payload);
    Json(
        json!({ "ok": true, "stored": payload["tabs"].as_array().map(|a| a.len()).unwrap_or(0), "ttl_seconds": 120 }),
    )
}

async fn clear_tabs(State(st): State<AppState>) -> Json<Value> {
    st.store.clear_latest_tabs();
    Json(json!({ "ok": true, "stored": 0 }))
}

async fn list_snaps(State(st): State<AppState>, Query(q): Query<ListQ>) -> Json<Value> {
    let items = st.store.list(q.q.as_deref()).unwrap_or_default();
    Json(json!({ "items": items }))
}

async fn one(State(st): State<AppState>, Path(id): Path<i64>) -> Response {
    match st.store.get(id) {
        Ok(Some(row)) => Json(row).into_response(),
        _ => (StatusCode::NOT_FOUND, "snapshot not found").into_response(),
    }
}

async fn shot(State(st): State<AppState>, Path(id): Path<i64>) -> Response {
    let Ok(Some(row)) = st.store.get(id) else {
        return (StatusCode::NOT_FOUND, "no screenshot").into_response();
    };
    let Some(path) = row.screenshot_path else {
        return (StatusCode::NOT_FOUND, "no screenshot").into_response();
    };
    match std::fs::read(&path) {
        Ok(bytes) => (StatusCode::OK, [("content-type", "image/jpeg")], bytes).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "file missing").into_response(),
    }
}

async fn capture_http(State(st): State<AppState>, body: Option<Json<CaptureIn>>) -> Response {
    let body = body.map(|j| j.0).unwrap_or(CaptureIn {
        note: None,
        include_clipboard: Some(true),
        tabs: None,
    });
    match do_capture(
        &st.store,
        body.note.as_deref(),
        body.include_clipboard.unwrap_or(true),
        body.tabs,
    ) {
        Ok(row) => Json(row).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err).into_response(),
    }
}

async fn del(State(st): State<AppState>, Path(id): Path<i64>) -> Response {
    match st.store.delete(id) {
        Ok(true) => Json(json!({ "deleted": id })).into_response(),
        _ => (StatusCode::NOT_FOUND, "snapshot not found").into_response(),
    }
}

async fn restore_plan(State(st): State<AppState>, Path(id): Path<i64>) -> Response {
    match restore::plan(&st.store, id) {
        Ok(v) => Json(v).into_response(),
        Err(err) => (StatusCode::NOT_FOUND, err).into_response(),
    }
}

async fn restore_run(State(st): State<AppState>, Path(id): Path<i64>) -> Response {
    match restore::execute(&st.store, id) {
        Ok(v) => Json(v).into_response(),
        Err(err) => (StatusCode::NOT_FOUND, err).into_response(),
    }
}

#[allow(dead_code)]
const _MAX: usize = MAX_TABS;
