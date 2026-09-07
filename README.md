# DeskTrace

<p>
  <img src="brand/wordmark.svg" alt="DeskTrace" height="72" />
</p>

Local-first desktop time machine.

**Stack: Tauri 2 + Rust.** Capture apps, screenshot, clipboard, and opt-in browser tabs. Restore later. Nothing leaves the machine.

Python under `app/` is leftover prototype. The running product is `src-tauri/`.

Repo: https://github.com/Steeve-Crypto/desktrace

## Product stack

```
src-tauri/
  src/capture.rs    processes, clipboard, screenshot
  src/store.rs      SQLite under ~/.desktrace
  src/tabs.rs       http(s) only sanitizer
  src/server.rs     127.0.0.1:8741 for the Chrome/Edge extension
  src/restore.rs    launch existing exes + open http(s) tabs
  src/autostart.rs  writes DeskTrace.bat to Windows Startup
  src/tray.rs       system tray + hide-on-close
static/             timeline UI
extension/          MV3 companion — localhost only
brand/              logo + wordmark
```

## Quick start

```bash
rustup default stable
cargo install tauri-cli --version "^2" --locked
cargo tauri dev
```

## Restore

Timeline detail → Restore now.

- Opens saved http/https tabs via the default browser
- Starts executables whose paths still exist and are not under `\\Windows\\`
- Best-effort. Does not recover unsaved documents. Does not reuse existing windows.

## Autostart

On first Windows launch the app writes `%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\DeskTrace.bat`.

## Tray + hotkey

One tray icon, built in Rust. Config does not declare a second tray.

- Close hides to tray and drops the taskbar / Alt+Tab entry
- Left-click toggles the timeline
- Hotkey tries `ctrl+shift+s`, then `ctrl+alt+s`, then `ctrl+shift+d`
- Menu accelerator and tooltip show the combo that actually registered
- Capture or hotkey failure is written to the tray tooltip and the timeline banner — never a silent 16×16 “success”

Timeline UI talks to `http://127.0.0.1:8741` explicitly.

## CI

`.github/workflows/ci.yml` runs `cargo fmt`, `clippy -D warnings`, and `cargo test --all-targets` on Ubuntu and Windows.

Windows installer still needs `cargo tauri build` on the laptop after those jobs are green.

## Privacy

Bind `127.0.0.1` only. No upload path. Tabs only from the extension.

MIT.
