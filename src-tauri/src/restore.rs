use crate::store::{Snapshot, Store};
use crate::tabs::{sanitize_tabs, MAX_TABS};
use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct RestoreAction {
    pub kind: String,
    pub target: String,
    pub name: Option<String>,
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RestoreReport {
    pub snapshot_id: i64,
    pub launched: Vec<RestoreAction>,
    pub note: String,
}

pub fn plan(store: &Store, id: i64) -> Result<serde_json::Value, String> {
    let snap = store
        .get(id)?
        .ok_or_else(|| "snapshot not found".to_string())?;
    Ok(serde_json::json!({
        "snapshot_id": id,
        "focused": snap.focused,
        "commands": exe_targets(&snap),
        "tabs": url_targets(&snap),
        "note": "Relaunch is best-effort. Unsaved documents inside apps are not recovered."
    }))
}

pub fn execute(store: &Store, id: i64) -> Result<RestoreReport, String> {
    let snap = store
        .get(id)?
        .ok_or_else(|| "snapshot not found".to_string())?;
    let mut launched = Vec::new();

    for cmd in exe_targets(&snap) {
        let target = cmd["target"].as_str().unwrap_or("").to_string();
        let name = cmd["name"].as_str().map(|s| s.to_string());
        launched.push(launch_exe(&target, name));
    }
    for tab in url_targets(&snap) {
        let target = tab["target"].as_str().unwrap_or("").to_string();
        let name = tab["name"].as_str().map(|s| s.to_string());
        launched.push(open_url(&target, name));
    }

    Ok(RestoreReport {
        snapshot_id: id,
        launched,
        note:
            "Best-effort restore. Existing windows are not reused. Unsaved docs are not recovered."
                .into(),
    })
}

fn exe_targets(snap: &Snapshot) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for item in &snap.apps {
        if out.len() >= 25 {
            break;
        }
        let Some(exe) = item.exe.as_deref() else {
            continue;
        };
        if !Path::new(exe).exists() {
            continue;
        }
        if is_blocked_exe(exe) {
            continue;
        }
        let lower = exe.to_lowercase();
        if !seen.insert(lower) {
            continue;
        }
        out.push(serde_json::json!({
            "kind": "exe",
            "target": exe,
            "name": item.name
        }));
    }
    out
}

fn url_targets(snap: &Snapshot) -> Vec<serde_json::Value> {
    sanitize_tabs(&snap.tabs)
        .into_iter()
        .take(MAX_TABS)
        .map(|t| {
            serde_json::json!({
                "kind": "url",
                "target": t.url,
                "name": t.title
            })
        })
        .collect()
}

fn launch_exe(target: &str, name: Option<String>) -> RestoreAction {
    let mut cmd = Command::new(target);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    match cmd.spawn() {
        Ok(_) => RestoreAction {
            kind: "exe".into(),
            target: target.into(),
            name,
            ok: true,
            error: None,
        },
        Err(err) => RestoreAction {
            kind: "exe".into(),
            target: target.into(),
            name,
            ok: false,
            error: Some(err.to_string()),
        },
    }
}

fn open_url(target: &str, name: Option<String>) -> RestoreAction {
    if !(target.starts_with("http://") || target.starts_with("https://")) {
        return RestoreAction {
            kind: "url".into(),
            target: target.into(),
            name,
            ok: false,
            error: Some("rejected scheme".into()),
        };
    }
    let result = open_browser(target);
    match result {
        Ok(()) => RestoreAction {
            kind: "url".into(),
            target: target.into(),
            name,
            ok: true,
            error: None,
        },
        Err(err) => RestoreAction {
            kind: "url".into(),
            target: target.into(),
            name,
            ok: false,
            error: Some(err),
        },
    }
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

pub fn is_blocked_exe(exe: &str) -> bool {
    let lower = exe.to_lowercase().replace('/', "\\");
    lower.contains("\\windows\\") || lower.contains("\\system32\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_system_paths() {
        assert!(is_blocked_exe(r"C:\\Windows\\System32\\notepad.exe"));
        assert!(is_blocked_exe("/Windows/System32/cmd.exe"));
        assert!(!is_blocked_exe(
            r"C:\\Users\\a\\AppData\\Local\\Programs\\cursor.exe"
        ));
    }

    #[test]
    fn rejects_non_http_open() {
        let act = open_url("file:///tmp/x", None);
        assert!(!act.ok);
    }
}
