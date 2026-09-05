const statusEl = document.getElementById("status");

function say(text) {
  statusEl.textContent = text;
}

function send(type) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage({ type }, resolve);
  });
}

document.getElementById("push").onclick = async () => {
  say("Talking to localhost…");
  const res = await send("push");
  if (!res?.ok) {
    say(res?.error || "DeskTrace is not running on :8741.");
    return;
  }
  say(`Stored ${res.data.stored} tabs for 120s.`);
};

document.getElementById("capture").onclick = async () => {
  say("Capturing…");
  const res = await send("capture");
  if (!res?.ok) {
    say(res?.error || "DeskTrace is not running on :8741.");
    return;
  }
  const n = (res.data.tabs || []).length;
  say(`Snapshot #${res.data.id} saved with ${n} tabs.`);
};
