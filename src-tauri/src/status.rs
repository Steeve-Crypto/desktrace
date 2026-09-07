use serde::Serialize;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Default, Serialize)]
pub struct RuntimeStatus {
    pub hotkey: Option<String>,
    pub last_error: Option<String>,
    pub last_ok_id: Option<i64>,
}

fn slot() -> &'static Mutex<RuntimeStatus> {
    static SLOT: OnceLock<Mutex<RuntimeStatus>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(RuntimeStatus::default()))
}

pub fn get() -> RuntimeStatus {
    slot().lock().map(|g| g.clone()).unwrap_or_default()
}

pub fn set_hotkey(combo: Option<String>) {
    if let Ok(mut g) = slot().lock() {
        g.hotkey = combo;
    }
}

pub fn set_error(err: impl Into<String>) {
    if let Ok(mut g) = slot().lock() {
        g.last_error = Some(err.into());
    }
}

pub fn set_ok(id: i64) {
    if let Ok(mut g) = slot().lock() {
        g.last_ok_id = Some(id);
        g.last_error = None;
    }
}
