# DeskTrace Tabs (Chrome / Edge)

Opt-in companion for https://github.com/Steeve-Crypto/desktrace/issues/2

The extension reads the **currently open** tab title and URL via `chrome.tabs` and POSTs that list to `http://127.0.0.1:8741` only. It does not open profile folders, History SQLite, Cookies, or any remote host.

## Permissions

| Permission | Why |
| --- | --- |
| `tabs` | Title + URL of windows the user already has open |
| `host_permissions: http://127.0.0.1:8741/*` | Talk to the local DeskTrace process |

No `history`, `cookies`, `webRequest`, `<all_urls>`, or file-system access. Incognito is `not_allowed`.

## Install (unpacked)

1. Start DeskTrace: `python -m app.main`
2. Chrome or Edge → `chrome://extensions` or `edge://extensions`
3. Enable Developer mode
4. Load unpacked → select this `extension/` folder
5. Pin the icon. Click **Push open tabs**, then capture in the DeskTrace UI within two minutes — or use **Capture snapshot now**

## API

```
POST http://127.0.0.1:8741/api/tabs
{
  "source": "desktrace-extension",
  "browser": "chrome",
  "tabs": [{ "title": "...", "url": "https://...", "active": true, "pinned": false, "window_id": 1 }]
}
```

DeskTrace keeps that list for 120 seconds and attaches it to the next snapshot. The extension can also POST the same list on `/api/snapshots`.
