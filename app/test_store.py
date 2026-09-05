from datetime import datetime, timezone
from pathlib import Path

from app.store import Store


def test_insert_list_delete(tmp_path: Path) -> None:
    store = Store(tmp_path)
    sid = store.insert(
        note="before reboot",
        focused="Code",
        apps=[{"name": "Code", "pid": 1, "exe": None}],
        clipboard="hello",
        screenshot_path=None,
        placeholder=True,
        tabs=[{"title": "Docs", "url": "https://example.com"}],
    )
    items = store.list(query="reboot")
    assert len(items) == 1
    assert items[0]["id"] == sid
    assert items[0]["tabs"][0]["url"] == "https://example.com"
    by_tab = store.list(query="example.com")
    assert by_tab[0]["id"] == sid
    assert store.delete(sid) is True
    assert store.list() == []


def test_latest_tabs_ttl_and_clear(tmp_path: Path) -> None:
    store = Store(tmp_path)
    store.save_latest_tabs(
        {
            "received_at": "2000-01-01T00:00:00+00:00",
            "tabs": [{"title": "Old", "url": "https://old.example"}],
        }
    )
    assert store.load_latest_tabs(120) == []
    store.save_latest_tabs(
        {
            "received_at": datetime.now(timezone.utc).isoformat(),
            "tabs": [
                {"title": "Now", "url": "https://now.example"},
                {"title": "Disk", "url": "file:///secret"},
            ],
        }
    )
    fresh = store.load_latest_tabs(120)
    assert [t["url"] for t in fresh] == ["https://now.example"]
    store.clear_latest_tabs()
    assert store.load_latest_tabs(120) == []
