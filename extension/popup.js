const statusEl = document.getElementById("status");

function say(text, cls) {
  statusEl.className = cls || "";
  statusEl.textContent = text;
}

function send(type) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage({ type }, resolve);
  });
}

async function refreshHealth() {
  const res = await send("health");
  if (!res?.ok) {
    say("DeskTrace is not running on 127.0.0.1:8741. Start python -m app.main first.", "bad");
    return;
  }
  const n = res.data.tab_count || 0;
  const fresh = res.data.tabs_fresh ? `${n} tabs cached` : "no fresh tab cache";
  say(`Localhost ok. ${fresh}.`, "ok");
}

document.getElementById("push").onclick = async () => {
  say("Talking to localhost…");
  const res = await send("push");
  if (!res?.ok) {
    say(res?.error || "DeskTrace is not running on :8741.", "bad");
    return;
  }
  say(`Stored ${res.data.stored} tabs for ${res.data.ttl_seconds}s.`, "ok");
};

document.getElementById("capture").onclick = async () => {
  say("Capturing…");
  const res = await send("capture");
  if (!res?.ok) {
    say(res?.error || "DeskTrace is not running on :8741.", "bad");
    return;
  }
  const n = (res.data.tabs || []).length;
  say(`Snapshot #${res.data.id} saved with ${n} tabs.`, "ok");
};

refreshHealth();
