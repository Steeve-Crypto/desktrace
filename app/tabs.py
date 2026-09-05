from __future__ import annotations

from typing import Any
from urllib.parse import urlparse

MAX_TABS = 80
MAX_TITLE = 300
MAX_URL = 2000
ALLOWED_SCHEMES = frozenset({"http", "https"})


def sanitize_tabs(raw: list[Any] | None) -> list[dict[str, Any]]:
    """Keep only currently-open http(s) titles+URLs. Never persist file paths."""
    cleaned: list[dict[str, Any]] = []
    seen: set[str] = set()
    for item in raw or []:
        if not isinstance(item, dict):
            continue
        url = str(item.get("url") or "").strip()[:MAX_URL]
        title = str(item.get("title") or "").strip()[:MAX_TITLE]
        if not url:
            continue
        parsed = urlparse(url)
        if parsed.scheme not in ALLOWED_SCHEMES:
            continue
        if not parsed.netloc:
            continue
        key = url
        if key in seen:
            continue
        seen.add(key)
        window_id = item.get("window_id", item.get("windowId"))
        cleaned.append(
            {
                "title": title or url,
                "url": url,
                "active": bool(item.get("active")),
                "pinned": bool(item.get("pinned")),
                "window_id": window_id if isinstance(window_id, int) else None,
            }
        )
        if len(cleaned) >= MAX_TABS:
            break
    return cleaned
