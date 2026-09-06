mod capture;
mod server;
mod store;
mod tabs;
mod tray;

use store::Store;

fn data_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".desktrace")
}

#[tauri::command]
fn capture_snapshot(note: Option<String>, include_clipboard: Option<bool>) -> Result<serde_json::Value, String> {
    let store = Store::open(data_dir()).map_err(|e| e.to_string())?;
    let snap = server::do_capture(
        &store,
        note.as_deref(),
        include_clipboard.unwrap_or(true),
        None,
    )?;
    serde_json::to_value(snap).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_snapshots(q: Option<String>) -> Result<serde_json::Value, String> {
    let store = Store::open(data_dir()).map_err(|e| e.to_string())?;
    let items = store.list(q.as_deref()).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "items": items }))
}

#[tauri::command]
fn get_stats() -> Result<serde_json::Value, String> {
    let store = Store::open(data_dir()).map_err(|e| e.to_string())?;
    Ok(store.stats())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = Store::open(data_dir()).expect("open ~/.desktrace");
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio");
        rt.block_on(server::serve(store));
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            capture_snapshot,
            list_snapshots,
            get_stats
        ])
        .setup(|app| {
            tray::install(app.handle())?;
            #[cfg(desktop)]
            {
                match tray::register_hotkeys(app.handle()) {
                    Ok(combo) => eprintln!("DeskTrace hotkey: {combo}"),
                    Err(err) => eprintln!("DeskTrace hotkey skipped: {err}"),
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running DeskTrace");
}
