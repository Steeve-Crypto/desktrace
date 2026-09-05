const BASE = "http://127.0.0.1:8741";
const ENDPOINT = `${BASE}/api/tabs`;
const CAPTURE = `${BASE}/api/snapshots`;
const HEALTH = `${BASE}/api/health`;
const ALLOWED = new Set(["http:", "https:"]);
const MAX_TABS = 80;

function allowedUrl(url) {
  try {
    const parsed = new URL(url);
    return ALLOWED.has(parsed.protocol);
  } catch {
    return false;
  }
}

function collectTabs(windows) {
  const tabs = [];
  for (const win of windows) {
    for (const tab of win.tabs || []) {
      const url = tab.url || "";
      const title = (tab.title || "").slice(0, 300);
      if (!allowedUrl(url)) continue;
      tabs.push({
        title: title || url,
        url,
        active: Boolean(tab.active),
        pinned: Boolean(tab.pinned),
        window_id: win.id,
      });
      if (tabs.length >= MAX_TABS) return tabs;
    }
  }
  return tabs;
}

async function listOpenTabs() {
  const windows = await chrome.windows.getAll({
    populate: true,
    windowTypes: ["normal"],
  });
  return collectTabs(windows);
}

async function ping() {
  const res = await fetch(HEALTH, { method: "GET" });
  if (!res.ok) throw new Error(`DeskTrace health ${res.status}`);
  return res.json();
}

async function postJson(url, body) {
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `HTTP ${res.status}. Is DeskTrace running on :8741?`);
  }
  return res.json();
}

async function pushTabs() {
  const tabs = await listOpenTabs();
  return postJson(ENDPOINT, {
    source: "desktrace-extension",
    browser: navigator.userAgent.includes("Edg/") ? "edge" : "chrome",
    tabs,
  });
}

async function captureNow(note) {
  const tabs = await listOpenTabs();
  return postJson(CAPTURE, {
    note: note || null,
    include_clipboard: false,
    tabs,
  });
}

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  const run = async () => {
    if (msg?.type === "health") return ping();
    if (msg?.type === "push") return pushTabs();
    if (msg?.type === "capture") return captureNow(msg.note);
    throw new Error("unknown command");
  };
  run()
    .then((data) => sendResponse({ ok: true, data }))
    .catch((err) => sendResponse({ ok: false, error: String(err.message || err) }));
  return true;
});
