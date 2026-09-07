use std::fs;
use std::path::PathBuf;

/// Best-effort Windows login start via a .bat in the user Startup folder.
/// Does nothing on other platforms. Safe to call every launch.
pub fn enable_windows_startup() {
    #[cfg(target_os = "windows")]
    {
        let Some(appdata) = dirs::data_dir() else {
            return;
        };
        // %APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
        let startup = appdata
            .parent()
            .map(|p| {
                p.join("Roaming")
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs")
                    .join("Startup")
            })
            .unwrap_or_else(|| PathBuf::from("."));
        let _ = fs::create_dir_all(&startup);
        let bat = startup.join("DeskTrace.bat");
        if let Ok(exe) = std::env::current_exe() {
            let body = format!("start \"\" \"{}\"\r\n", exe.display());
            let _ = fs::write(bat, body);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = PathBuf::new();
        let _ = fs::metadata(".");
    }
}
