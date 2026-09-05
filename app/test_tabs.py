from app.tabs import sanitize_tabs


def test_keeps_http_https_and_drops_everything_else() -> None:
    cleaned = sanitize_tabs(
        [
            {"title": "Docs", "url": "https://example.com/a", "active": True},
            {"title": "Local", "url": "http://127.0.0.1:8741"},
            {"title": "Disk", "url": "file:///C:/Users/me/secret.pdf"},
            {"title": "Internal", "url": "chrome://settings"},
            {"title": "About", "url": "about:blank"},
            {"title": "JS", "url": "javascript:alert(1)"},
            {"title": "Data", "url": "data:text/html,hi"},
            {"title": "No host", "url": "https://"},
            "not-a-dict",
            {"title": "Missing url"},
        ]
    )
    urls = [t["url"] for t in cleaned]
    assert urls == ["https://example.com/a", "http://127.0.0.1:8741"]
    assert cleaned[0]["active"] is True
    assert cleaned[0]["title"] == "Docs"


def test_dedupes_and_caps() -> None:
    raw = [{"title": "A", "url": "https://example.com"}] * 3
    raw += [{"title": str(i), "url": f"https://n{i}.example"} for i in range(90)]
    cleaned = sanitize_tabs(raw)
    assert len(cleaned) == 80
    assert cleaned[0]["url"] == "https://example.com"
    assert len({t["url"] for t in cleaned}) == 80
