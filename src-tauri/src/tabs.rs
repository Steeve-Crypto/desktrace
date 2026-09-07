use serde::{Deserialize, Serialize};
use url::Url;

pub const ALLOWED_SCHEMES: [&str; 2] = ["http", "https"];
pub const MAX_TABS: usize = 80;
pub const MAX_TITLE: usize = 200;
pub const MAX_URL: usize = 2048;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Tab {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub window_id: Option<i64>,
}

pub fn sanitize_tabs(raw: &[Tab]) -> Vec<Tab> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for tab in raw {
        if out.len() >= MAX_TABS {
            break;
        }
        let url = tab.url.trim();
        if url.len() > MAX_URL {
            continue;
        }
        let Ok(parsed) = Url::parse(url) else {
            continue;
        };
        if !ALLOWED_SCHEMES.contains(&parsed.scheme()) {
            continue;
        }
        if parsed.host_str().unwrap_or("").is_empty() {
            continue;
        }
        if !seen.insert(url.to_string()) {
            continue;
        }
        let mut title = tab.title.trim().to_string();
        title.truncate(MAX_TITLE);
        out.push(Tab {
            title,
            url: url.to_string(),
            active: tab.active,
            pinned: tab.pinned,
            window_id: tab.window_id,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(url: &str) -> Tab {
        Tab {
            title: "t".into(),
            url: url.into(),
            ..Default::default()
        }
    }

    #[test]
    fn drops_file_and_chrome_schemes() {
        let raw = [
            tab("https://example.com/a"),
            tab("http://localhost:3000"),
            tab("file:///tmp/secret"),
            tab("chrome://settings"),
            tab("https://example.com/a"),
        ];
        let out = sanitize_tabs(&raw);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].url, "https://example.com/a");
        assert_eq!(out[1].url, "http://localhost:3000");
    }

    #[test]
    fn caps_at_max_tabs() {
        let raw: Vec<Tab> = (0..200)
            .map(|i| tab(&format!("https://ex.com/{i}")))
            .collect();
        assert_eq!(sanitize_tabs(&raw).len(), MAX_TABS);
    }
}
