#[cfg(target_os = "windows")]
use std::fs;
#[cfg(target_os = "windows")]
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "windows")]
fn startup_bat() -> Option<PathBuf> {
    let appdata = dirs::data_dir()?;
    let roaming = appdata.parent()?.join("Roaming");
    Some(
        roaming
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Startup")
            .join("DeskTrace.bat"),
    )
}

pub fn is_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        startup_bat().map(|p| p.exists()).unwrap_or(false)
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

/// Best-effort Windows login start: Startup folder bat + HKCU Run.
pub fn enable_windows_startup() {
    #[cfg(target_os = "windows")]
    {
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        if let Some(bat) = startup_bat() {
            if let Some(dir) = bat.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let body = format!("start \"\" \"{}\"\r\n", exe.display());
            let _ = fs::write(bat, body);
        }
        let _ = Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "DeskTrace",
                "/t",
                "REG_SZ",
                "/d",
                &exe.display().to_string(),
                "/f",
            ])
            .output();
    }
}

pub fn disable_windows_startup() {
    #[cfg(target_os = "windows")]
    {
        if let Some(bat) = startup_bat() {
            let _ = fs::remove_file(bat);
        }
        let _ = Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "DeskTrace",
                "/f",
            ])
            .output();
    }
}

pub fn toggle() -> bool {
    if is_enabled() {
        disable_windows_startup();
        false
    } else {
        enable_windows_startup();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::is_enabled;

    #[test]
    fn non_windows_reports_disabled() {
        if cfg!(not(target_os = "windows")) {
            assert!(!is_enabled());
        }
    }
}
