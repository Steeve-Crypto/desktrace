from __future__ import annotations

import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from fastapi import FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field, field_validator

from app.capture import list_apps, read_clipboard, snapshot_files
from app.store import Store

ROOT = Path(__file__).resolve().parent.parent
STATIC = ROOT / "static"
MAX_TABS = 80
TAB_TTL_SECONDS = 120

store = Store()
app = FastAPI(title="DeskTrace", version="0.2.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=[],
    allow_origin_regex=r"^chrome-extension://[a-z0-9]+$|^http://127\.0\.0\.1:8741$|^http://localhost:8741$",
    allow_methods=["GET", "POST", "DELETE", "OPTIONS"],
    allow_headers=["Content-Type"],
)


class CaptureIn(BaseModel):
    note: str | None = Field(default=None, max_length=500)
    include_clipboard: bool = True
    tabs: list[dict[str, Any]] | None = None


class CallToolIn(BaseModel):
    name: str
    arguments: dict[str, Any] = Field(default_factory=dict)


class TabIn(BaseModel):
    title: str = Field(default="", max_length=300)
    url: str = Field(default="", max_length=2000)
    active: bool = False
    pinned: bool = False
    window_id: int | None = None

    @field_validator("url")
    @classmethod
    def url_must_be_httpish(cls, value: str) -> str:
        value = value.strip()
        if not value:
            return ""
        parsed = urlparse(value)
        if parsed.scheme in {"http", "https", "chrome", "edge", "about", "file"}:
            return value
        raise ValueError("unsupported url scheme")


class TabsIn(BaseModel):
    source: str = Field(default="desktrace-extension", max_length=64)
    browser: str | None = Field(default=None, max_length=32)
    tabs: list[TabIn] = Field(default_factory=list)


def _sanitize_tabs(raw: list[Any] | None) -> list[dict[str, Any]]:
    cleaned: list[dict[str, Any]] = []
    for item in raw or []:
        if not isinstance(item, dict):
            continue
        url = str(item.get("url") or "").strip()
        title = str(item.get("title") or "").strip()[:300]
        if not url and not title:
            continue
        parsed = urlparse(url)
        if url and parsed.scheme not in {"http", "https", "chrome", "edge", "about", "file"}:
            continue
        cleaned.append(
            {
                "title": title or url,
                "url": url,
                "active": bool(item.get("active")),
                "pinned": bool(item.get("pinned")),
                "window_id": item.get("window_id") or item.get("windowId"),
            }
        )
        if len(cleaned) >= MAX_TABS:
            break
    return cleaned


def do_capture(
    note: str | None = None,
    include_clipboard: bool = True,
    tabs: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    apps, focused = list_apps()
    clip = read_clipboard() if include_clipboard else None
    shot_path, real = snapshot_files(store.shots_dir)
    attached = _sanitize_tabs(tabs) if tabs else store.load_latest_tabs(TAB_TTL_SECONDS)
    snap_id = store.insert(
        note=note,
        focused=focused,
        apps=apps,
        clipboard=clip,
        screenshot_path=str(shot_path),
        placeholder=not real,
        tabs=attached,
    )
    row = store.get(snap_id)
    assert row is not None
    return row


@app.middleware("http")
async def localhost_only(request: Request, call_next):
    host = (request.headers.get("host") or "").split(":")[0]
    if host not in {"127.0.0.1", "localhost"}:
        raise HTTPException(403, "DeskTrace only listens on localhost")
    return await call_next(request)


@app.get("/api/health")
def health() -> dict[str, Any]:
    latest = store.load_latest_tabs(TAB_TTL_SECONDS)
    return {
        "ok": True,
        "product": "DeskTrace",
        "bind": "127.0.0.1",
        "tabs_fresh": bool(latest),
        "tab_count": len(latest),
    }


@app.get("/api/stats")
def stats() -> dict[str, Any]:
    data = store.stats()
    latest = store.load_latest_tabs(TAB_TTL_SECONDS)
    data["tabs_fresh"] = bool(latest)
    data["tab_count"] = len(latest)
    return data


@app.get("/api/tabs")
def get_tabs() -> dict[str, Any]:
    path = store.data_dir / "latest_tabs.json"
    if not path.exists():
        return {"tabs": [], "fresh": False}
    try:
        payload = path.read_text(encoding="utf-8")
        import json

        data = json.loads(payload)
    except Exception:
        return {"tabs": [], "fresh": False}
    tabs = store.load_latest_tabs(TAB_TTL_SECONDS)
    return {
        "tabs": tabs,
        "fresh": bool(tabs),
        "received_at": data.get("received_at"),
        "browser": data.get("browser"),
        "source": data.get("source"),
    }


@app.post("/api/tabs")
def put_tabs(body: TabsIn) -> dict[str, Any]:
    tabs = _sanitize_tabs([t.model_dump() for t in body.tabs])
    payload = {
        "source": body.source,
        "browser": body.browser,
        "received_at": datetime.now(timezone.utc).isoformat(),
        "tabs": tabs,
    }
    store.save_latest_tabs(payload)
    return {"ok": True, "stored": len(tabs), "ttl_seconds": TAB_TTL_SECONDS}


@app.get("/api/snapshots")
def snapshots(q: str | None = None) -> dict[str, Any]:
    return {"items": store.list(query=q)}


@app.get("/api/snapshots/{snapshot_id}")
def snapshot_one(snapshot_id: int) -> dict[str, Any]:
    row = store.get(snapshot_id)
    if not row:
        raise HTTPException(404, "snapshot not found")
    return row


@app.get("/api/snapshots/{snapshot_id}/shot")
def snapshot_shot(snapshot_id: int) -> FileResponse:
    row = store.get(snapshot_id)
    if not row or not row.get("screenshot_path"):
        raise HTTPException(404, "no screenshot")
    path = Path(row["screenshot_path"])
    if not path.exists():
        raise HTTPException(404, "file missing")
    return FileResponse(path, media_type="image/jpeg")


@app.post("/api/snapshots")
def capture(body: CaptureIn | None = None) -> dict[str, Any]:
    body = body or CaptureIn()
    return do_capture(
        note=body.note,
        include_clipboard=body.include_clipboard,
        tabs=body.tabs,
    )


@app.delete("/api/snapshots/{snapshot_id}")
def delete_snapshot(snapshot_id: int) -> dict[str, Any]:
    if not store.delete(snapshot_id):
        raise HTTPException(404, "snapshot not found")
    return {"deleted": snapshot_id}


@app.post("/api/snapshots/{snapshot_id}/restore-plan")
def restore_plan(snapshot_id: int) -> dict[str, Any]:
    row = store.get(snapshot_id)
    if not row:
        raise HTTPException(404, "snapshot not found")
    commands = []
    for item in row["apps"]:
        exe = item.get("exe")
        name = item.get("name")
        if exe and Path(str(exe)).exists():
            commands.append({"kind": "exe", "target": exe, "name": name})
        elif name:
            commands.append({"kind": "name", "target": name, "name": name})
    tab_urls = [
        {"kind": "url", "target": tab.get("url"), "name": tab.get("title")}
        for tab in row.get("tabs") or []
        if tab.get("url", "").startswith(("http://", "https://"))
    ]
    return {
        "snapshot_id": snapshot_id,
        "focused": row.get("focused"),
        "commands": commands[:25],
        "tabs": tab_urls[:MAX_TABS],
        "note": "Relaunch is best-effort. Unsaved documents inside apps are not recovered.",
    }


TOOLS = [
    {
        "name": "capture_snapshot",
        "description": "Take a local DeskTrace snapshot of apps + screenshot. Data stays on disk.",
        "parameters": {
            "type": "object",
            "properties": {
                "note": {"type": "string"},
                "include_clipboard": {"type": "boolean", "default": True},
            },
        },
    },
    {
        "name": "list_snapshots",
        "description": "List recent local snapshots, optionally filtered.",
        "parameters": {
            "type": "object",
            "properties": {"q": {"type": "string"}},
        },
    },
    {
        "name": "get_stats",
        "description": "Local store stats for DeskTrace.",
        "parameters": {"type": "object", "properties": {}},
    },
]


@app.get("/tools")
def tools() -> dict[str, Any]:
    return {"tools": TOOLS}


@app.post("/call_tool")
def call_tool(body: CallToolIn) -> dict[str, Any]:
    if body.name == "capture_snapshot":
        return do_capture(
            note=body.arguments.get("note"),
            include_clipboard=bool(body.arguments.get("include_clipboard", True)),
        )
    if body.name == "list_snapshots":
        return {"items": store.list(query=body.arguments.get("q"))}
    if body.name == "get_stats":
        return store.stats()
    raise HTTPException(400, f"unknown tool: {body.name}")


if STATIC.exists():
    app.mount("/", StaticFiles(directory=STATIC, html=True), name="static")


def run() -> None:
    import uvicorn

    host = os.environ.get("DESKTRACE_HOST", "127.0.0.1")
    port = int(os.environ.get("DESKTRACE_PORT", "8741"))
    uvicorn.run("app.main:app", host=host, port=port, reload=False)


if __name__ == "__main__":
    run()
