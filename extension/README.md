# DeskTrace Tabs (Chrome / Edge)

Implements https://github.com/Steeve-Crypto/desktrace/issues/2

The extension reads **currently open** tab titles and URLs through `chrome.tabs` / `chrome.windows` and POSTs that list to `http://127.0.0.1:8741` only.

It does **not**:

- open the Chrome/Edge profile folder
- query History, Cookies, Bookmarks, or Sync
- inject content scripts
- talk to any host other than loopback port 8741
- run in Incognito (`incognito: not_allowed`)
- send `file://`, `chrome://`, `edge://`, `about:`, `data:`, or `javascript:` URLs

Installing the unpacked extension is the opt-in. Nothing is sent until the user clicks **Push open tabs** or **Capture snapshot now**.

## Permissions

| Permission | Why |
| --- | --- |
| `tabs` | Title + URL of windows the user already has open |
| `host_permissions: http://127.0.0.1:8741/*` and `http://localhost:8741/*` | Talk to the local DeskTrace process |

No `history`, `cookies`, `webRequest`, `debugger`, `<all_urls>`, downloads, or nativeMessaging.

## Install (unpacked)

1. Start DeskTrace: `python -m app.main`
2. Chrome → `chrome://extensions` or Edge → `edge://extensions`
3. Enable Developer mode
4. Load unpacked → select this `extension/` folder
5. Pin the icon. Click **Push open tabs**, then capture in the DeskTrace UI within two minutes — or use **Capture snapshot now**

## Local API contract

```
POST http://127.0.0.1:8741/api/tabs
{
  "source": "desktrace-extension",
  "browser": "chrome" | "edge",
  "tabs": [
    { "title": "...", "url": "https://...", "active": true, "pinned": false, "window_id": 1 }
  ]
}
```

Response:

```
{ "ok": true, "stored": 12, "ttl_seconds": 120 }
```

DeskTrace keeps that list for 120 seconds in `~/.desktrace/latest_tabs.json` and attaches it to the next snapshot if the snapshot body does not include `tabs`. The extension can also POST the same list inline on `POST /api/snapshots`.

```
GET  /api/tabs     inspect the cache
DELETE /api/tabs   wipe the cache
GET  /api/health   { tabs_fresh, tab_count, bind }
```

Server-side sanitizer drops unknown schemes, caps at 80 tabs, title ≤ 300 chars, URL ≤ 2000 chars.
