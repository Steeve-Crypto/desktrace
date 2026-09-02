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
    )
    items = store.list(query="reboot")
    assert len(items) == 1
    assert items[0]["id"] == sid
    assert store.delete(sid) is True
    assert store.list() == []
