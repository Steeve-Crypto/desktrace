# Privacy

DeskTrace stores snapshots only under `~/.desktrace` on the machine that captured them.

- Bound to `127.0.0.1` by default
- No accounts
- No telemetry
- No upload client
- Delete removes the SQLite row and the JPEG
- Browser tabs arrive only from the installed companion extension, via POST to 127.0.0.1:8741
- The extension never reads History, Cookies, Bookmarks, Sync, or profile SQLite files
- Only currently open `http`/`https` titles and URLs are accepted; `file://` and internal pages are dropped
- The latest-tab cache lives 120 seconds in `~/.desktrace/latest_tabs.json` and is not uploaded

Do not add a silent cloud path. Optional backup, if ever added, must be user-owned storage and off by default.
