const ENDPOINT = "http://127.0.0.1:8741/api/tabs";
const CAPTURE = "http://127.0.0.1:8741/api/snapshots";

function collectTabs(windows) {
  const tabs = [];
  for (const win of windows) {
    for (const tab of win.tabs || []) {
      const url = tab.url || "";
      const title = tab.title || "";
      if (!url && !title) continue;
      tabs.push({
        title,
        url,
        active: Boolean(tab.active),
        pinned: Boolean(tab.pinned),
        window_id: win.id,
      });
    }
  }
  return tabs;
}

async function listOpenTabs() {
  const windows = await chrome.windows.getAll({ populate: true, windowTypes: ["normal"] });
  return collectTabs(windows);
}

async function pushTabs() {
  const tabs = await listOpenTabs();
  const res = await fetch(ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      source: "desktrace-extension",
      browser: navigator.userAgent.includes("Edg/") ? "edge" : "chrome",
      tabs,
    }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
  return res.json();
}

async function captureNow(note) {
  const tabs = await listOpenTabs();
  const res = await fetch(CAPTURE, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      note: note || null,
      include_clipboard: false,
      tabs,
    }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}`);
  }
  return res.json();
}

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  const run = async () => {
    if (msg?.type === "push") return pushTabs();
    if (msg?.type === "capture") return captureNow(msg.note);
    throw new Error("unknown command");
  };
  run().then((data) => sendResponse({ ok: true, data })).catch((err) => {
    sendResponse({ ok: false, error: String(err.message || err) });
  });
  return true;
});
