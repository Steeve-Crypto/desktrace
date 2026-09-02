from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from app.capture import list_apps, read_clipboard, snapshot_files
from app.store import Store

ROOT = Path(__file__).resolve().parent.parent
STATIC = ROOT / "static"

store = Store()
app = FastAPI(title="DeskTrace", version="0.1.0")


class CaptureIn(BaseModel):
    note: str | None = Field(default=None, max_length=500)
    include_clipboard: bool = True


class CallToolIn(BaseModel):
    name: str
    arguments: dict[str, Any] = Field(default_factory=dict)


def do_capture(note: str | None = None, include_clipboard: bool = True) -> dict[str, Any]:
    apps, focused = list_apps()
    clip = read_clipboard() if include_clipboard else None
    shot_path, real = snapshot_files(store.shots_dir)
    snap_id = store.insert(
        note=note,
        focused=focused,
        apps=apps,
        clipboard=clip,
        screenshot_path=str(shot_path),
        placeholder=not real,
    )
    row = store.get(snap_id)
    assert row is not None
    return row


@app.get("/api/health")
def health() -> dict[str, Any]:
    return {"ok": True, "product": "DeskTrace", "bind": "127.0.0.1"}


@app.get("/api/stats")
def stats() -> dict[str, Any]:
    return store.stats()


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
    return do_capture(note=body.note, include_clipboard=body.include_clipboard)


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
    return {
        "snapshot_id": snapshot_id,
        "focused": row.get("focused"),
        "commands": commands[:25],
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
