const { invoke } = window.__TAURI__.core;
const { open }     = window.__TAURI__.dialog;
const { listen }   = window.__TAURI__.event;

// ── Screen management ─────────────────────────────────────────────────

const screens = {
  "first-launch": document.getElementById("screen-first-launch"),
  onboarding: document.getElementById("screen-onboarding"),
  error: document.getElementById("screen-error"),
  popover: document.getElementById("screen-popover"),
};

function showScreen(name) {
  Object.values(screens).forEach((s) => s.classList.add("hidden"));
  screens[name].classList.remove("hidden");
}

// ── State ─────────────────────────────────────────────────────────────

let destinations = [];
let selectedDestination = null;
let isFirstLaunch = true;

// ── Init ──────────────────────────────────────────────────────────────

async function init() {
  // Check if already configured — go straight to popover if so
  try {
    const config = await invoke("get_config");
    if (config && config.watched_folder && config.designer_name) {
      isFirstLaunch = false;
      populatePopover(config);
      showScreen("popover");
      await invoke("start_watching");
      return;
    }
    // Config exists but is stale from an older version — clear it
    if (config) {
      try { await invoke("clear_config"); } catch (_) {}
    }
  } catch (e) {
    console.error("Config read failed:", e);
  }

  // First launch — show Mac setup instructions
  showScreen("first-launch");
}

// ── Destinations ──────────────────────────────────────────────────────

async function loadDestinations() {
  const select = document.getElementById("destination-select");
  select.innerHTML = '<option value="">Loading…</option>';

  try {
    const data = await invoke("fetch_destinations");
    destinations = data.destinations;

    select.innerHTML = '<option value="">Choose destination…</option>';
    destinations.forEach((d, i) => {
      const opt = document.createElement("option");
      opt.value = i;
      opt.textContent = d.label;
      select.appendChild(opt);
    });
  } catch (e) {
    showScreen("error");
  }
}

document.getElementById("destination-select").addEventListener("change", async (e) => {
  const idx = parseInt(e.target.value);
  if (isNaN(idx)) return;

  selectedDestination = destinations[idx];
  await loadDesignerNames(selectedDestination);
});

// ── Designer names ────────────────────────────────────────────────────

async function loadDesignerNames(destination) {
  const nameSelect = document.getElementById("name-select");
  nameSelect.disabled = true;
  nameSelect.innerHTML = '<option value="">Loading names…</option>';

  try {
    const names = await invoke("fetch_designer_names", {
      rcloneRemote: destination.rclone_remote,
      driveId: destination.drive_id,
    });

    nameSelect.innerHTML = '<option value="">Choose your name…</option>';
    names.forEach((name) => {
      const opt = document.createElement("option");
      opt.value = name;
      opt.textContent = name;
      nameSelect.appendChild(opt);
    });
    nameSelect.disabled = false;
  } catch (e) {
    console.error("fetch_designer_names failed:", e);
    nameSelect.innerHTML = `<option value="">⚠ ${e}</option>`;
    nameSelect.disabled = false;
  }

  updateStartButton();
}

document.getElementById("name-select").addEventListener("change", () => {
  updateStartButton();
});

// ── Folder picker ─────────────────────────────────────────────────────

document.getElementById("btn-browse").addEventListener("click", async () => {
  try {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      const input = document.getElementById("folder-path");
      input.value = selected;
      input.classList.add("has-value");
      updateStartButton();
    }
  } catch (e) {
    console.error("Folder picker error:", e);
  }
});

// ── Start button ──────────────────────────────────────────────────────

function updateStartButton() {
  const dest   = document.getElementById("destination-select").value;
  const name   = document.getElementById("name-select").value;
  const folder = document.getElementById("folder-path").value;
  document.getElementById("btn-start").disabled = !(dest && name && folder);
}

document.getElementById("btn-start").addEventListener("click", async () => {
  const nameSelect  = document.getElementById("name-select");
  const folderPath  = document.getElementById("folder-path").value;

  try {
    await invoke("save_config", {
      designerName:       nameSelect.value,
      watchedFolder:      folderPath,
      destinationLabel:   selectedDestination.label,
      destinationDriveId: selectedDestination.drive_id,
      rcloneRemote:       selectedDestination.rclone_remote,
    });

    populatePopover({
      designer_name:     nameSelect.value,
      destination_label: selectedDestination.label,
      watched_folder:    folderPath,
      last_sync:         null,
      last_synced_project: null,
    });

    showScreen("popover");
    await invoke("start_watching");
  } catch (e) {
    console.error("Save config failed:", e);
  }
});

// ── Retry button ──────────────────────────────────────────────────────

document.getElementById("btn-retry").addEventListener("click", async () => {
  showScreen("onboarding");
  await loadDestinations();
});

// ── Popover ───────────────────────────────────────────────────────────

function populatePopover(config) {
  document.getElementById("info-name").textContent        = config.designer_name     || "—";
  document.getElementById("info-destination").textContent = config.destination_label || "—";
  document.getElementById("info-folder").textContent      =
    config.watched_folder?.replace(/^.*[\\/]([^\\/]+)$/, "~/$1") || "—";

  const lastSyncTime  = config.last_sync        ? new Date(config.last_sync)        : null;
  const lastErrorTime = config.last_error_time  ? new Date(config.last_error_time)  : null;
  const hasError = lastErrorTime && (!lastSyncTime || lastErrorTime > lastSyncTime);

  const syncEl  = document.getElementById("info-last-sync");
  const badge   = document.getElementById("status-badge");

  if (hasError) {
    const project = config.last_error_project || "unknown project";
    syncEl.textContent = `Failed — ${project}`;
    syncEl.classList.add("info-value-error");
    badge.className   = "status-badge status-error";
    badge.textContent = "● Error";
    document.getElementById("info-last-project").textContent = project;
  } else {
    syncEl.textContent = lastSyncTime ? formatRelativeTime(config.last_sync) : "Never";
    syncEl.classList.remove("info-value-error");
    document.getElementById("info-last-project").textContent =
      config.last_synced_project || "Waiting for changes…";
  }
}

document.getElementById("btn-open-folder").addEventListener("click", () => {
  invoke("open_watched_folder");
});

document.getElementById("btn-open-drive").addEventListener("click", () => {
  invoke("open_drive");
});

// ── First launch screen ───────────────────────────────────────────────

document.getElementById("btn-copy").addEventListener("click", () => {
  const cmd = 'xattr -cr "/Applications/Wild Sync.app"';
  navigator.clipboard.writeText(cmd).then(() => {
    const btn = document.getElementById("btn-copy");
    btn.textContent = "Copied";
    btn.classList.add("copied");
    setTimeout(() => {
      btn.textContent = "Copy";
      btn.classList.remove("copied");
    }, 2000);
  });
});

document.getElementById("btn-continue-setup").addEventListener("click", async () => {
  showScreen("onboarding");
  await loadDestinations();
});

// ── Sync status updates from Rust ─────────────────────────────────────

listen("sync-status", (event) => {
  const badge  = document.getElementById("status-badge");
  const syncEl = document.getElementById("info-last-sync");
  const { status, last_sync, project } = event.payload;

  badge.className = "";
  if (status === "syncing") {
    badge.classList.add("status-sync");
    badge.textContent = project ? `● Syncing ${project}…` : "● Syncing…";
    syncEl.classList.remove("info-value-error");
  } else if (status === "error") {
    badge.classList.add("status-error");
    badge.textContent = "● Error";
    syncEl.textContent = project ? `Failed — ${project}` : "Failed";
    syncEl.classList.add("info-value-error");
    if (project) {
      document.getElementById("info-last-project").textContent = project;
    }
  } else {
    badge.classList.add("status-idle");
    badge.textContent = "● Synced";
    syncEl.classList.remove("info-value-error");
    if (project) {
      document.getElementById("info-last-project").textContent = project;
    }
    if (last_sync) {
      syncEl.textContent = formatRelativeTime(last_sync);
    }
  }
});

// ── Helpers ───────────────────────────────────────────────────────────

function formatRelativeTime(isoString) {
  const diff = Math.floor((Date.now() - new Date(isoString).getTime()) / 1000);
  if (diff < 60) return "Just now";
  if (diff < 3600) return `${Math.floor(diff / 60)} mins ago`;
  return `${Math.floor(diff / 3600)} hrs ago`;
}

// ── Boot ──────────────────────────────────────────────────────────────

init();