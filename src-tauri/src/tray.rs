use crate::server;
use crate::status;
use crate::store::Store;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "desktrace";
const DEFAULT_TIP: &str = "DeskTrace — waiting for hotkey";

fn data_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".desktrace")
}

pub fn capture_now(note: Option<&str>) -> Result<i64, String> {
    let store = Store::open(data_dir()).map_err(|e| e.to_string())?;
    let snap = server::do_capture(&store, note, true, None)?;
    if snap.placeholder {
        return Err(snap
            .shot_error
            .unwrap_or_else(|| "screenshot failed".into()));
    }
    Ok(snap.id)
}

pub fn mark_saved(app: &AppHandle, id: i64) {
    status::set_ok(id);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let combo = status::get().hotkey.unwrap_or_else(|| "no hotkey".into());
        let _ = tray.set_tooltip(Some(format!("DeskTrace — saved #{id} · {combo}")));
    }
}

pub fn mark_error(app: &AppHandle, err: &str) {
    status::set_error(err);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(format!("DeskTrace — capture failed: {err}")));
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

fn accel_label(combo: &str) -> String {
    combo
        .split('+')
        .map(|part| match part {
            "ctrl" => "Ctrl".to_string(),
            "alt" => "Alt".to_string(),
            "shift" => "Shift".to_string(),
            other => other.to_uppercase(),
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn menu_for(app: &AppHandle, combo: Option<&str>) -> tauri::Result<Menu<tauri::Wry>> {
    let accel = combo.map(accel_label);
    let capture = MenuItem::with_id(app, "capture", "Capture now", true, accel.as_deref())?;
    let show = MenuItem::with_id(app, "show", "Open timeline", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
    let auto_label = if crate::autostart::is_enabled() {
        "Don't start with Windows"
    } else {
        "Start with Windows"
    };
    let auto = MenuItem::with_id(app, "autostart", auto_label, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit DeskTrace", true, None::<&str>)?;
    Menu::with_items(app, &[&capture, &show, &hide, &auto, &quit])
}

pub fn apply_hotkey_ui(app: &AppHandle, combo: Option<&str>) {
    status::set_hotkey(combo.map(|s| s.to_string()));
    if let Ok(menu) = menu_for(app, combo) {
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let _ = tray.set_menu(Some(menu));
            let tip = match combo {
                Some(c) => format!("DeskTrace — {c} to capture"),
                None => "DeskTrace — no hotkey available".into(),
            };
            let _ = tray.set_tooltip(Some(tip));
        }
    }
}

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let menu = menu_for(app, None)?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(DEFAULT_TIP)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture" => match capture_now(Some("tray")) {
                Ok(id) => mark_saved(app, id),
                Err(err) => mark_error(app, &err),
            },
            "show" => show_timeline(app),
            "hide" => hide_to_tray(app),
            "autostart" => {
                crate::autostart::toggle();
                apply_hotkey_ui(app, status::get().hotkey.as_deref());
            }
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
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};

    app.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, shortcut, event| {
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
                    Err(err) => mark_error(app, &err),
                }
            })
            .build(),
    )
    .map_err(|e| e.to_string())?;

    let combos = ["ctrl+shift+s", "ctrl+alt+s", "ctrl+shift+d"];
    let mut last_err = String::from("no combo worked");
    for combo in combos {
        match app.global_shortcut().register(combo) {
            Ok(()) => {
                apply_hotkey_ui(app, Some(combo));
                return Ok(combo.to_string());
            }
            Err(err) => last_err = format!("{combo}: {err}"),
        }
    }
    apply_hotkey_ui(app, None);
    status::set_error(format!("hotkey unavailable ({last_err})"));
    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::accel_label;

    #[test]
    fn accel_pretty() {
        assert_eq!(accel_label("ctrl+alt+s"), "Ctrl+Alt+S");
    }
}
