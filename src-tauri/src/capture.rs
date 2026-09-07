use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRow {
    pub name: String,
    pub exe: Option<String>,
    pub pid: u32,
}

pub fn list_apps() -> (Vec<AppRow>, Option<String>) {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut apps = Vec::new();
    let mut focused = None;
    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy().to_string();
        if name.is_empty() {
            continue;
        }
        let lower = name.to_lowercase();
        if matches!(
            lower.as_str(),
            "system" | "idle" | "registry" | "smss.exe" | "csrss.exe" | "svchost.exe"
        ) {
            continue;
        }
        let exe = proc.exe().map(|p| p.to_string_lossy().to_string());
        if focused.is_none() {
            focused = Some(name.clone());
        }
        apps.push(AppRow {
            name,
            exe,
            pid: pid.as_u32(),
        });
        if apps.len() >= 80 {
            break;
        }
    }
    (apps, focused)
}

pub fn read_clipboard() -> Option<String> {
    let mut clip = arboard::Clipboard::new().ok()?;
    let text = clip.get_text().ok()?;
    if text.trim().is_empty() {
        return None;
    }
    Some(text.chars().take(20_000).collect())
}

pub fn take_screenshot(dir: &Path) -> (PathBuf, bool) {
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join(format!("{}.jpg", uuid::Uuid::new_v4()));
    match screenshots::Screen::all() {
        Ok(screens) => {
            if let Some(screen) = screens.first() {
                if let Ok(image) = screen.capture() {
                    let buffer = image::RgbaImage::from_raw(
                        image.width(),
                        image.height(),
                        image.rgba().to_vec(),
                    );
                    if let Some(buf) = buffer {
                        let rgb = image::DynamicImage::ImageRgba8(buf).to_rgb8();
                        if rgb.save_with_format(&path, image::ImageFormat::Jpeg).is_ok() {
                            return (path, true);
                        }
                    }
                }
            }
        }
        Err(_) => {}
    }
    let placeholder = image::RgbImage::from_pixel(16, 16, image::Rgb([12, 13, 16]));
    let _ = placeholder.save_with_format(&path, image::ImageFormat::Jpeg);
    (path, false)
}
