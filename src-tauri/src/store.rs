use crate::capture::AppRow;
use crate::tabs::Tab;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const TAB_TTL_SECS: u64 = 120;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: i64,
    pub created_at: String,
    pub note: Option<String>,
    pub focused: Option<String>,
    pub apps: Vec<AppRow>,
    pub clipboard: Option<String>,
    pub screenshot_path: Option<String>,
    pub placeholder: bool,
    pub tabs: Vec<Tab>,
}

#[derive(Clone)]
pub struct Store {
    pub data_dir: PathBuf,
    pub shots_dir: PathBuf,
    db_path: PathBuf,
}

impl Store {
    pub fn open(data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        let shots_dir = data_dir.join("shots");
        std::fs::create_dir_all(&shots_dir).map_err(|e| e.to_string())?;
        let db_path = data_dir.join("desktrace.db");
        let store = Self {
            data_dir,
            shots_dir,
            db_path,
        };
        store.init()?;
        Ok(store)
    }

    fn conn(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path).map_err(|e| e.to_string())
    }

    fn init(&self) -> Result<(), String> {
        let conn = self.conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                note TEXT,
                focused TEXT,
                apps_json TEXT NOT NULL DEFAULT '[]',
                clipboard TEXT,
                screenshot_path TEXT,
                placeholder INTEGER NOT NULL DEFAULT 0,
                tabs_json TEXT NOT NULL DEFAULT '[]'
            );",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert(
        &self,
        note: Option<&str>,
        focused: Option<&str>,
        apps: &[AppRow],
        clipboard: Option<&str>,
        screenshot_path: Option<&str>,
        placeholder: bool,
        tabs: &[Tab],
    ) -> Result<i64, String> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO snapshots (created_at, note, focused, apps_json, clipboard, screenshot_path, placeholder, tabs_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                Utc::now().to_rfc3339(),
                note,
                focused,
                serde_json::to_string(apps).unwrap_or_else(|_| "[]".into()),
                clipboard,
                screenshot_path,
                if placeholder { 1 } else { 0 },
                serde_json::to_string(tabs).unwrap_or_else(|_| "[]".into()),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get(&self, id: i64) -> Result<Option<Snapshot>, String> {
        let conn = self.conn()?;
        let row = conn
            .query_row(
                "SELECT id, created_at, note, focused, apps_json, clipboard, screenshot_path, placeholder, tabs_json FROM snapshots WHERE id = ?1",
                params![id],
                map_row,
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(row)
    }

    pub fn list(&self, q: Option<&str>) -> Result<Vec<Snapshot>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, created_at, note, focused, apps_json, clipboard, screenshot_path, placeholder, tabs_json
                 FROM snapshots ORDER BY id DESC LIMIT 200",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], map_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        if let Some(q) = q.filter(|s| !s.is_empty()) {
            let needle = q.to_lowercase();
            return Ok(rows
                .into_iter()
                .filter(|s| {
                    s.note.clone().unwrap_or_default().to_lowercase().contains(&needle)
                        || s.focused.clone().unwrap_or_default().to_lowercase().contains(&needle)
                        || s.apps.iter().any(|a| a.name.to_lowercase().contains(&needle))
                })
                .collect());
        }
        Ok(rows)
    }

    pub fn delete(&self, id: i64) -> Result<bool, String> {
        if let Some(row) = self.get(id)? {
            if let Some(path) = row.screenshot_path {
                let _ = std::fs::remove_file(path);
            }
        }
        let conn = self.conn()?;
        let n = conn
            .execute("DELETE FROM snapshots WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub fn stats(&self) -> Value {
        let count = self.list(None).map(|v| v.len()).unwrap_or(0);
        let mut shots_bytes = 0u64;
        if let Ok(rd) = std::fs::read_dir(&self.shots_dir) {
            for e in rd.flatten() {
                if let Ok(m) = e.metadata() {
                    shots_bytes += m.len();
                }
            }
        }
        let latest = self.load_latest_tabs();
        serde_json::json!({
            "count": count,
            "shots_bytes": shots_bytes,
            "data_dir": self.data_dir.to_string_lossy(),
            "tabs_fresh": !latest.is_empty(),
            "tab_count": latest.len()
        })
    }

    pub fn save_latest_tabs(&self, payload: &Value) -> Result<(), String> {
        std::fs::write(
            self.data_dir.join("latest_tabs.json"),
            serde_json::to_vec_pretty(payload).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }

    pub fn load_latest_tabs(&self) -> Vec<Tab> {
        let path = self.data_dir.join("latest_tabs.json");
        let Ok(bytes) = std::fs::read(&path) else {
            return vec![];
        };
        let Ok(v) = serde_json::from_slice::<Value>(&bytes) else {
            return vec![];
        };
        if let Some(ts) = v.get("received_at").and_then(|x| x.as_str()) {
            if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                let age = Utc::now().signed_duration_since(dt.with_timezone(&Utc));
                if age.num_seconds() > TAB_TTL_SECS as i64 {
                    return vec![];
                }
            }
        }
        let tabs: Vec<Tab> = serde_json::from_value(v.get("tabs").cloned().unwrap_or(Value::Array(vec![])))
            .unwrap_or_default();
        crate::tabs::sanitize_tabs(&tabs)
    }

    pub fn clear_latest_tabs(&self) {
        let _ = std::fs::remove_file(self.data_dir.join("latest_tabs.json"));
    }
}

fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Snapshot> {
    let apps_json: String = row.get(4)?;
    let tabs_json: String = row.get(8)?;
    Ok(Snapshot {
        id: row.get(0)?,
        created_at: row.get(1)?,
        note: row.get(2)?,
        focused: row.get(3)?,
        apps: serde_json::from_str(&apps_json).unwrap_or_default(),
        clipboard: row.get(5)?,
        screenshot_path: row.get(6)?,
        placeholder: row.get::<_, i64>(7)? != 0,
        tabs: serde_json::from_str(&tabs_json).unwrap_or_default(),
    })
}

#[allow(dead_code)]
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[allow(dead_code)]
fn exists(p: &Path) -> bool {
    p.exists()
}
