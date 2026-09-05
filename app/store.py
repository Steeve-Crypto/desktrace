from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def default_data_dir() -> Path:
    return Path.home() / ".desktrace"


class Store:
    def __init__(self, data_dir: Path | None = None) -> None:
        self.data_dir = data_dir or default_data_dir()
        self.shots_dir = self.data_dir / "shots"
        self.db_path = self.data_dir / "snapshots.db"
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.shots_dir.mkdir(parents=True, exist_ok=True)
        self._init()

    def _connect(self) -> sqlite3.Connection:
        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        return conn

    def _init(self) -> None:
        with self._connect() as conn:
            conn.execute(
                """
                CREATE TABLE IF NOT EXISTS snapshots (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    created_at TEXT NOT NULL,
                    note TEXT,
                    focused TEXT,
                    apps_json TEXT NOT NULL,
                    clipboard TEXT,
                    screenshot_path TEXT,
                    placeholder INTEGER NOT NULL DEFAULT 0,
                    tabs_json TEXT
                )
                """
            )
            cols = {
                row[1]
                for row in conn.execute("PRAGMA table_info(snapshots)").fetchall()
            }
            if "tabs_json" not in cols:
                conn.execute("ALTER TABLE snapshots ADD COLUMN tabs_json TEXT")

    def insert(
        self,
        *,
        note: str | None,
        focused: str | None,
        apps: list[dict[str, Any]],
        clipboard: str | None,
        screenshot_path: str | None,
        placeholder: bool,
        tabs: list[dict[str, Any]] | None = None,
    ) -> int:
        created = datetime.now(timezone.utc).isoformat()
        with self._connect() as conn:
            cur = conn.execute(
                """
                INSERT INTO snapshots
                    (created_at, note, focused, apps_json, clipboard, screenshot_path, placeholder, tabs_json)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    created,
                    note,
                    focused,
                    json.dumps(apps),
                    clipboard,
                    screenshot_path,
                    int(placeholder),
                    json.dumps(tabs or []),
                ),
            )
            return int(cur.lastrowid)

    def list(self, query: str | None = None, limit: int = 200) -> list[dict[str, Any]]:
        sql = "SELECT * FROM snapshots"
        params: list[Any] = []
        if query:
            sql += " WHERE note LIKE ? OR focused LIKE ? OR apps_json LIKE ? OR tabs_json LIKE ?"
            like = f"%{query}%"
            params.extend([like, like, like, like])
        sql += " ORDER BY id DESC LIMIT ?"
        params.append(limit)
        with self._connect() as conn:
            rows = conn.execute(sql, params).fetchall()
        return [self._row(r) for r in rows]

    def get(self, snapshot_id: int) -> dict[str, Any] | None:
        with self._connect() as conn:
            row = conn.execute(
                "SELECT * FROM snapshots WHERE id = ?", (snapshot_id,)
            ).fetchone()
        return self._row(row) if row else None

    def delete(self, snapshot_id: int) -> bool:
        row = self.get(snapshot_id)
        if not row:
            return False
        path = row.get("screenshot_path")
        if path:
            p = Path(path)
            if p.exists():
                p.unlink()
        with self._connect() as conn:
            conn.execute("DELETE FROM snapshots WHERE id = ?", (snapshot_id,))
        return True

    def save_latest_tabs(self, payload: dict[str, Any]) -> None:
        path = self.data_dir / "latest_tabs.json"
        path.write_text(json.dumps(payload), encoding="utf-8")

    def load_latest_tabs(self, max_age_seconds: int = 120) -> list[dict[str, Any]]:
        path = self.data_dir / "latest_tabs.json"
        if not path.exists():
            return []
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            return []
        received = payload.get("received_at")
        if received:
            try:
                ts = datetime.fromisoformat(received)
                if ts.tzinfo is None:
                    ts = ts.replace(tzinfo=timezone.utc)
                age = (datetime.now(timezone.utc) - ts).total_seconds()
                if age > max_age_seconds:
                    return []
            except ValueError:
                return []
        tabs = payload.get("tabs")
        return tabs if isinstance(tabs, list) else []

    def stats(self) -> dict[str, Any]:
        with self._connect() as conn:
            count = conn.execute("SELECT COUNT(*) FROM snapshots").fetchone()[0]
        shot_bytes = sum(p.stat().st_size for p in self.shots_dir.glob("*") if p.is_file())
        return {
            "count": count,
            "shots_bytes": shot_bytes,
            "data_dir": str(self.data_dir),
        }

    @staticmethod
    def _row(row: sqlite3.Row) -> dict[str, Any]:
        return {
            "id": row["id"],
            "created_at": row["created_at"],
            "note": row["note"],
            "focused": row["focused"],
            "apps": json.loads(row["apps_json"] or "[]"),
            "clipboard": row["clipboard"],
            "screenshot_path": row["screenshot_path"],
            "placeholder": bool(row["placeholder"]),
            "tabs": json.loads(row["tabs_json"] or "[]") if "tabs_json" in row.keys() else [],
        }
