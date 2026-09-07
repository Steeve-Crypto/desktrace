use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppRow {
    pub name: String,
    pub exe: Option<String>,
    pub pid: u32,
}

#[derive(Debug, Clone)]
pub struct ClipResult {
    pub text: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ShotResult {
    pub path: PathBuf,
    pub placeholder: bool,
    pub error: Option<String>,
    pub monitors: u32,
}

#[derive(Debug, Clone)]
pub struct MonitorFrame {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn list_apps() -> (Vec<AppRow>, Option<String>) {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let fg = foreground_pid();
    let mut apps = Vec::new();
    let mut focused = None;
    for (pid, proc) in sys.processes() {
        let name = proc.name().to_string_lossy().to_string();
        if name.is_empty() || is_noise(&name) {
            continue;
        }
        let exe = proc.exe().map(|p| p.to_string_lossy().to_string());
        let pid_u = pid.as_u32();
        if fg == Some(pid_u) {
            focused = Some(name.clone());
        }
        apps.push(AppRow {
            name,
            exe,
            pid: pid_u,
        });
        if apps.len() >= 80 {
            break;
        }
    }
    if focused.is_none() {
        if let Some(pid) = fg {
            focused = Some(format!("pid:{pid}"));
        }
    }
    (apps, focused)
}

pub fn is_noise(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "system"
            | "idle"
            | "registry"
            | "smss.exe"
            | "csrss.exe"
            | "svchost.exe"
            | "wininit.exe"
            | "services.exe"
    )
}

pub fn read_clipboard() -> ClipResult {
    match arboard::Clipboard::new() {
        Ok(mut clip) => match clip.get_text() {
            Ok(text) if text.trim().is_empty() => ClipResult {
                text: None,
                error: None,
            },
            Ok(text) => ClipResult {
                text: Some(text.chars().take(20_000).collect()),
                error: None,
            },
            Err(err) => ClipResult {
                text: None,
                error: Some(format!("clipboard read failed: {err}")),
            },
        },
        Err(err) => ClipResult {
            text: None,
            error: Some(format!("clipboard unavailable: {err}")),
        },
    }
}

pub fn take_screenshot(dir: &Path) -> ShotResult {
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join(format!("{}.jpg", uuid::Uuid::new_v4()));
    match capture_all_monitors() {
        Ok(frames) if !frames.is_empty() => {
            let monitors = frames.len() as u32;
            match stitch_frames(&frames) {
                Ok(rgb) => match rgb.save_with_format(&path, image::ImageFormat::Jpeg) {
                    Ok(()) => ShotResult {
                        path,
                        placeholder: false,
                        error: None,
                        monitors,
                    },
                    Err(err) => write_placeholder(path, Some(format!("jpeg save failed: {err}"))),
                },
                Err(err) => write_placeholder(path, Some(err)),
            }
        }
        Ok(_) => write_placeholder(path, Some("no monitors reported".into())),
        Err(err) => write_placeholder(path, Some(err)),
    }
}

fn capture_all_monitors() -> Result<Vec<MonitorFrame>, String> {
    let screens = screenshots::Screen::all().map_err(|e| format!("screen enum failed: {e}"))?;
    if screens.is_empty() {
        return Err("screen enum returned zero displays".into());
    }
    let mut frames = Vec::new();
    let mut errors = Vec::new();
    for screen in screens {
        match screen.capture() {
            Ok(image) => {
                let width = image.width();
                let height = image.height();
                let rgba = image.into_raw();
                if rgba.len() != (width as usize) * (height as usize) * 4 {
                    errors.push(format!("monitor {width}x{height} buffer size mismatch"));
                    continue;
                }
                frames.push(MonitorFrame {
                    x: screen.display_info.x,
                    y: screen.display_info.y,
                    width,
                    height,
                    rgba,
                });
            }
            Err(err) => errors.push(format!("monitor capture failed: {err}")),
        }
    }
    if frames.is_empty() {
        return Err(if errors.is_empty() {
            "all monitor captures empty".into()
        } else {
            errors.join("; ")
        });
    }
    Ok(frames)
}

pub fn stitch_frames(frames: &[MonitorFrame]) -> Result<image::RgbImage, String> {
    if frames.is_empty() {
        return Err("no frames to stitch".into());
    }
    let min_x = frames.iter().map(|f| f.x).min().unwrap();
    let min_y = frames.iter().map(|f| f.y).min().unwrap();
    let max_x = frames
        .iter()
        .map(|f| f.x.saturating_add_unsigned(f.width))
        .max()
        .unwrap();
    let max_y = frames
        .iter()
        .map(|f| f.y.saturating_add_unsigned(f.height))
        .max()
        .unwrap();
    let w = (max_x - min_x) as u32;
    let h = (max_y - min_y) as u32;
    if w == 0 || h == 0 || w > 16_384 || h > 16_384 {
        return Err(format!("invalid virtual desktop {w}x{h}"));
    }
    let mut canvas = image::RgbImage::from_pixel(w, h, image::Rgb([12, 13, 16]));
    for frame in frames {
        let ox = (frame.x - min_x) as u32;
        let oy = (frame.y - min_y) as u32;
        let src = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba.clone())
            .ok_or_else(|| "rgba buffer rejected".to_string())?;
        for (x, y, px) in src.enumerate_pixels() {
            let dx = ox + x;
            let dy = oy + y;
            if dx < w && dy < h {
                canvas.put_pixel(dx, dy, image::Rgb([px[0], px[1], px[2]]));
            }
        }
    }
    Ok(canvas)
}

fn write_placeholder(path: PathBuf, error: Option<String>) -> ShotResult {
    let placeholder = image::RgbImage::from_pixel(16, 16, image::Rgb([12, 13, 16]));
    let _ = placeholder.save_with_format(&path, image::ImageFormat::Jpeg);
    ShotResult {
        path,
        placeholder: true,
        error,
        monitors: 0,
    }
}

fn foreground_pid() -> Option<u32> {
    #[cfg(windows)]
    {
        windows_foreground_pid()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn windows_foreground_pid() -> Option<u32> {
    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> isize;
        fn GetWindowThreadProcessId(hwnd: isize, lpdwprocessid: *mut u32) -> u32;
    }
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            None
        } else {
            Some(pid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_filter() {
        assert!(is_noise("svchost.exe"));
        assert!(!is_noise("Code.exe"));
    }

    #[test]
    fn stitch_two_monitors_side_by_side() {
        let left = MonitorFrame {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
            ],
        };
        let right = MonitorFrame {
            x: 2,
            y: 0,
            width: 2,
            height: 2,
            rgba: vec![
                0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255,
            ],
        };
        let img = stitch_frames(&[left, right]).unwrap();
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 2);
        assert_eq!(img.get_pixel(0, 0), &image::Rgb([255, 0, 0]));
        assert_eq!(img.get_pixel(3, 1), &image::Rgb([0, 255, 0]));
    }
}
