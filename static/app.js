const timeline = document.getElementById("timeline");
const statsEl = document.getElementById("stats");
const detail = document.getElementById("detail");
let currentId = null;
const API = "http://127.0.0.1:8741";

async function j(url, opts) {
  const res = await fetch(url.startsWith("http") ? url : API + url, opts);
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

function fmt(iso) {
  try {
    return new Date(iso).toLocaleString();
  } catch {
    return iso;
  }
}

async function loadStats() {
  const s = await j("/api/stats");
  const mb = (s.shots_bytes / (1024 * 1024)).toFixed(2);
  const tabs = s.tabs_fresh ? `${s.tab_count} tabs cached` : "no fresh tabs";
  const hk = s.hotkey ? s.hotkey : "no hotkey";
  statsEl.textContent = `${s.count} snapshots · ${mb} MB shots · ${tabs} · ${hk} · ${s.data_dir}`;
  const banner = document.getElementById("error-banner");
  if (banner) {
    if (s.last_error) {
      banner.textContent = s.last_error;
      banner.hidden = false;
    } else {
      banner.hidden = true;
    }
  }
}

async function loadList() {
  const q = document.getElementById("search").value.trim();
  const url = q ? `/api/snapshots?q=${encodeURIComponent(q)}` : "/api/snapshots";
  const data = await j(url);
  timeline.innerHTML = "";
  if (!data.items.length) {
    timeline.innerHTML = `<p class="empty">No snapshots yet. Hit Capture now.</p>`;
    return;
  }
  for (const item of data.items) {
    const card = document.createElement("article");
    card.className = "card";
    card.innerHTML = `
      <img src="${API}/api/snapshots/${item.id}/shot" alt="" />
      <div class="body">
        <h3>${item.note || item.focused || "Snapshot #" + item.id}</h3>
        <p>${fmt(item.created_at)} · ${item.apps.length} apps · ${(item.tabs || []).length} tabs${item.placeholder ? " · SCREENSHOT FAILED" : ""}${item.shot_error ? " · " + item.shot_error : ""}</p>
      </div>`;
    card.onclick = () => openDetail(item.id);
    timeline.appendChild(card);
  }
}

async function openDetail(id) {
  currentId = id;
  const item = await j(`/api/snapshots/${id}`);
  detail.classList.remove("hidden");
  document.getElementById("shot").src = `${API}/api/snapshots/${id}/shot?t=${Date.now()}`;
  document.getElementById("detail-title").textContent = item.note || `Snapshot #${item.id}`;
  document.getElementById("detail-time").textContent = fmt(item.created_at);
  document.getElementById("detail-note").textContent = item.note || "";
  document.getElementById("detail-focus").textContent = item.focused
    ? `Foreground: ${item.focused}${item.monitors ? " · " + item.monitors + " monitor(s)" : ""}`
    : "";
  const err = [item.shot_error, item.clipboard_error].filter(Boolean).join(" · ");
  const errEl = document.getElementById("detail-error");
  if (errEl) {
    errEl.textContent = err;
    errEl.hidden = !err;
  }
  document.getElementById("app-list").innerHTML = item.apps
    .map((a) => `<li>${a.name}</li>`)
    .join("");
  const tabs = item.tabs || [];
  document.getElementById("tab-list").innerHTML = tabs.length
    ? tabs.map((t) => `<li>${t.active ? "● " : ""}${t.title || t.url}<br /><span class="url">${t.url || ""}</span></li>`).join("")
    : "<li>No tabs. Load the extension and push first.</li>";
  document.getElementById("plan-out").textContent = "";
}

document.getElementById("close").onclick = () => detail.classList.add("hidden");

document.getElementById("capture").onclick = async () => {
  const note = document.getElementById("note").value.trim();
  await j("/api/snapshots", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ note: note || null, include_clipboard: true }),
  });
  document.getElementById("note").value = "";
  await loadStats();
  await loadList();
};

document.getElementById("search").addEventListener("input", () => {
  clearTimeout(window._t);
  window._t = setTimeout(loadList, 200);
});

document.getElementById("del").onclick = async () => {
  if (!currentId) return;
  if (!confirm("Delete this snapshot from disk?")) return;
  await fetch(`${API}/api/snapshots/${currentId}`, { method: "DELETE" });
  detail.classList.add("hidden");
  await loadStats();
  await loadList();
};

document.getElementById("plan").onclick = async () => {
  if (!currentId) return;
  const plan = await j(`/api/snapshots/${currentId}/restore-plan`, { method: "POST" });
  document.getElementById("plan-out").textContent = JSON.stringify(plan, null, 2);
};

document.getElementById("restore").onclick = async () => {
  if (!currentId) return;
  if (!confirm("Relaunch saved apps and open saved http(s) tabs? Unsaved work in those apps is not recovered.")) return;
  const report = await j(`/api/snapshots/${currentId}/restore`, { method: "POST" });
  document.getElementById("plan-out").textContent = JSON.stringify(report, null, 2);
};

loadStats().catch((e) => (statsEl.textContent = e.message));
loadList().catch((e) => (timeline.textContent = e.message));
