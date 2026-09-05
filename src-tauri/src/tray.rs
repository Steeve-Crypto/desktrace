use crate::server;
use crate::store::Store;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

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

pub fn show_timeline(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

pub fn hide_to_tray(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let capture = MenuItem::with_id(app, "capture", "Capture now", true, Some("Ctrl+Shift+S"))?;
    let show = MenuItem::with_id(app, "show", "Open timeline", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit DeskTrace", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&capture, &show, &hide, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("bundle icon missing");

    TrayIconBuilder::with_id("desktrace")
        .tooltip("DeskTrace — Ctrl+Shift+S to capture")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture" => match capture_now(Some("tray")) {
                Ok(id) => {
                    if let Some(tray) = app.tray_by_id("desktrace") {
                        let _ = tray.set_tooltip(Some(format!("DeskTrace — saved #{id}")));
                    }
                }
                Err(err) => eprintln!("tray capture failed: {err}"),
            },
            "show" => show_timeline(app),
            "hide" => hide_to_tray(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                show_timeline(tray.app_handle());
            }
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(win) = tray.app_handle().get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        let _ = win.hide();
                    } else {
                        show_timeline(tray.app_handle());
                    }
                }
            }
        })
        .build(app)?;

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
