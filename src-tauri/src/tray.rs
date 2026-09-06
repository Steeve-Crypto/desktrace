use crate::server;
use crate::store::Store;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "desktrace";
const DEFAULT_TIP: &str = "DeskTrace — Ctrl+Shift+S to capture";

fn data_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".desktrace")
}

pub fn capture_now(note: Option<&str>) -> Result<i64, String> {
    let store = Store::open(data_dir()).map_err(|e| e.to_string())?;
    let snap = server::do_capture(&store, note, true, None)?;
    Ok(snap.id)
}

pub fn mark_saved(app: &AppHandle, id: i64) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(format!("DeskTrace — saved #{id}")));
    }
}

pub fn show_timeline(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_skip_taskbar(false);
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

pub fn hide_to_tray(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
        let _ = win.set_skip_taskbar(true);
    }
}

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let capture = MenuItem::with_id(app, "capture", "Capture now", true, Some("Ctrl+Shift+S"))?;
    let show = MenuItem::with_id(app, "show", "Open timeline", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit DeskTrace", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&capture, &show, &hide, &quit])?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(DEFAULT_TIP)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture" => match capture_now(Some("tray")) {
                Ok(id) => mark_saved(app, id),
                Err(err) => eprintln!("tray capture failed: {err}"),
            },
            "show" => show_timeline(app),
            "hide" => hide_to_tray(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        hide_to_tray(app);
                    } else {
                        show_timeline(app);
                    }
                }
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;

    if let Some(win) = app.get_webview_window("main") {
        let handle = app.clone();
        win.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide_to_tray(&handle);
            }
        });
    }

    Ok(())
}

#[cfg(desktop)]
pub fn register_hotkeys(app: &AppHandle) -> Result<String, String> {
    use tauri_plugin_global_shortcut::{Code, Modifiers, ShortcutState};

    let combos = ["ctrl+shift+s", "ctrl+alt+s", "ctrl+shift+d"];
    let mut last_err = String::from("no combo worked");
    for combo in combos {
        let built = tauri_plugin_global_shortcut::Builder::new()
            .with_shortcuts([combo])
            .and_then(|b| {
                Ok(b.with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let hit = shortcut.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyS)
                        || shortcut.matches(Modifiers::CONTROL | Modifiers::ALT, Code::KeyS)
                        || shortcut.matches(Modifiers::CONTROL | Modifiers::SHIFT, Code::KeyD);
                    if !hit {
                        return;
                    }
                    match capture_now(Some("hotkey")) {
                        Ok(id) => mark_saved(app, id),
                        Err(err) => eprintln!("hotkey capture failed: {err}"),
                    }
                })
                .build())
            });
        match built {
            Ok(plugin) => {
                if let Err(err) = app.plugin(plugin) {
                    last_err = format!("{combo}: {err}");
                    continue;
                }
                if let Some(tray) = app.tray_by_id(TRAY_ID) {
                    let _ = tray.set_tooltip(Some(format!("DeskTrace — {combo} to capture")));
                }
                return Ok(combo.to_string());
            }
            Err(err) => last_err = format!("{combo}: {err}"),
        }
    }
    Err(last_err)
}
