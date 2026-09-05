# DeskTrace

Local-first desktop time machine.

Capture a snapshot of what you were doing — open apps, focused window, clipboard text, and a screenshot — then scroll back and restore later. Everything stays on your machine. Nothing is uploaded.

Working name. Rename anytime. Repo: https://github.com/Steeve-Crypto/desktrace

## Why this exists

Windows hibernation saves *everything* but only at shutdown. PowerToys Workspaces relaunches apps but loses the moment. DeskTrace sits in the middle: lightweight checkpoints you can take on a timer or with a hotkey, searchable, private.

## MVP (v0.1) — what ships today

- Manual + interval snapshots
- Screenshot of the primary display (when the OS allows it)
- Process / window list
- Clipboard text (optional)
- Timeline UI with search
- Local SQLite + JPEG files under `~/.desktrace`
- Restore *plan* (relaunch listed apps) — not a full memory freeze
- Local HTTP API so other tools / agents can trigger a capture
- Opt-in Chrome/Edge tab companion (`extension/`) that posts titles+URLs to localhost only

**Not in v0.1:** cloud sync, account login, selling or sharing snapshots, keylogging, browser password reads, silent background upload.

## Privacy rule

DeskTrace is a diary you keep on disk, not a product that phones home.

- Default bind: `127.0.0.1` only
- No analytics SDK
- No third-party upload path in this codebase
- Delete is real delete (row + JPEG)
- Tabs arrive only from the installed extension, http(s) only, never from profile databases

If you later add optional encrypted backup, it must be opt-in and user-owned storage (your S3 / your NAS). Do not ship a "research analytics" toggle that ships raw screenshots off-box.

## Quick start

```bash
cd desktrace
python -m venv .venv
source .venv/bin/activate   # Windows: .venv\Scripts\activate
pip install -r requirements.txt
python -m app.main
```

Open http://127.0.0.1:8741

Load the unpacked companion from `extension/` (see `extension/README.md`).

## Architecture

```
static/          timeline UI (no build step)
extension/       MV3 Chrome/Edge companion (localhost only)
app/main.py      FastAPI + /tools + /call_tool + /api/tabs
app/tabs.py      URL sanitizer (http/https only)
app/store.py     SQLite + 120s latest_tabs.json cache
app/capture.py   screenshot + process list (mss / fallback)
~/.desktrace/    snapshots.db + shots/ + latest_tabs.json
```

Stack choice: Python + FastAPI so you can run it *tonight* on the Windows laptop that keeps dying. Tauri/Rust native shell is the v0.2 wrap when the capture loop is proven.

## Capture reality check

True "freeze RAM and reopen Excel mid-cell" is what **hibernation** already does. DeskTrace records *enough context to reconstruct the session*.

Browser tabs: opt-in Chrome/Edge extension in `extension/`. It POSTs open titles+URLs to `http://127.0.0.1:8741/api/tabs` only. No profile-database scrape.

## Monetization (honest)

ICP: knowledge workers who lose context after crashes, dock/undock, or "where was that tab."

v1 money is **not** selling user screens.

- Free: local snapshots, 7-day retention
- Pro ($8–12/mo): unlimited local history, encrypted folder export, multi-monitor, restore playbooks
- Team later: shared *workspace templates* (app layouts), never raw personal screens

## Agent interface

```
GET  /tools
POST /call_tool   { "name": "capture_snapshot", "arguments": { "note": "before reboot" } }
```

## Next milestones

1. Windows installer + tray icon + global hotkey (issue #1)
2. Chrome/Edge companion extension for tab URLs — shipped in `extension/` (issue #2)
3. Tauri shell so it is a real `.msi` / `.dmg`
4. Encrypted zip export the user copies to a USB

## License

MIT. You own the name change.
