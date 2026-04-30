import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

// ── Screen management ─────────────────────────────────────────────────

const screens = {
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

// ── Init ──────────────────────────────────────────────────────────────

async function init() {
  // Check if already configured — go straight to popover if so
  try {
    const config = await invoke("get_config");
    if (config) {
      populatePopover(config);
      showScreen("popover");
      await invoke("start_watching");
      return;
    }
  } catch (e) {
    console.error("Config read failed:", e);
  }

  // First launch — show onboarding
  showScreen("onboarding");
  await loadDestinations();
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
    nameSelect.innerHTML = '<option value="">Could not load names</option>';
  }

  updateStartButton();
}

// ── Project names ─────────────────────────────────────────────────────

async function loadProjectNames(designerName) {
  const projectSelect = document.getElementById("project-select");
  const projectNew = document.getElementById("project-new");
  projectSelect.disabled = true;
  projectSelect.innerHTML = '<option value="">Loading projects…</option>';
  projectNew.classList.add("hidden");
  projectNew.value = "";
  updateStartButton();

  try {
    const projects = await invoke("fetch_project_names", {
      rcloneRemote: selectedDestination.rclone_remote,
      driveId: selectedDestination.drive_id,
      designerName,
    });

    projectSelect.innerHTML = '<option value="">Choose a project…</option>';
    const newOpt = document.createElement("option");
    newOpt.value = "__new__";
    newOpt.textContent = "+ New project…";
    projectSelect.appendChild(newOpt);

    projects.forEach((name) => {
      const opt = document.createElement("option");
      opt.value = name;
      opt.textContent = name;
      projectSelect.appendChild(opt);
    });
    projectSelect.disabled = false;
  } catch (e) {
    projectSelect.innerHTML = '<option value="">Could not load projects</option>';
  }
  updateStartButton();
}

document.getElementById("project-select").addEventListener("change", (e) => {
  const projectNew = document.getElementById("project-new");
  if (e.target.value === "__new__") {
    projectNew.classList.remove("hidden");
    projectNew.focus();
  } else {
    projectNew.classList.add("hidden");
    projectNew.value = "";
  }
  updateStartButton();
});

document.getElementById("project-new").addEventListener("input", updateStartButton);

// ── Folder picker ─────────────────────────────────────────────────────

document.getElementById("btn-browse").addEventListener("click", async () => {
  try {
    // Opens native folder picker dialog
    const selected = await openDialog({ directory: true, multiple: false });
    if (selected) {
      document.getElementById("folder-path").value = selected;
      updateStartButton();
    }
  } catch (e) {
    console.error("Folder picker error:", e);
  }
});

// ── Start button ──────────────────────────────────────────────────────

function updateStartButton() {
  const dest = document.getElementById("destination-select").value;
  const name = document.getElementById("name-select").value;
  const projectVal = document.getElementById("project-select").value;
  const projectNew = document.getElementById("project-new");
  const project = projectVal === "__new__" ? projectNew.value.trim() : projectVal;
  const folder = document.getElementById("folder-path").value;
  document.getElementById("btn-start").disabled = !(dest && name && project && folder);
}

document.getElementById("name-select").addEventListener("change", async (e) => {
  const name = e.target.value;
  const projectSelect = document.getElementById("project-select");
  projectSelect.innerHTML = '<option value="">Select your name first</option>';
  projectSelect.disabled = true;
  document.getElementById("project-new").classList.add("hidden");
  document.getElementById("project-new").value = "";
  if (!name) { updateStartButton(); return; }
  await loadProjectNames(name);
});

document.getElementById("btn-start").addEventListener("click", async () => {
  const nameSelect = document.getElementById("name-select");
  const folderPath = document.getElementById("folder-path").value;

  try {
    const projectSelect = document.getElementById("project-select");
    const projectNewInput = document.getElementById("project-new");
    const projectName = projectSelect.value === "__new__"
      ? projectNewInput.value.trim()
      : projectSelect.value;

    await invoke("save_config", {
      designerName: nameSelect.value,
      projectName,
      watchedFolder: folderPath,
      destinationLabel: selectedDestination.label,
      destinationDriveId: selectedDestination.drive_id,
      rcloneRemote: selectedDestination.rclone_remote,
    });

    populatePopover({
      designer_name: nameSelect.value,
      project_name: projectName,
      destination_label: selectedDestination.label,
      watched_folder: folderPath,
      last_sync: null,
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
  document.getElementById("info-name").textContent = config.designer_name || "—";
  document.getElementById("info-project").textContent = config.project_name || "—";
  document.getElementById("info-destination").textContent = config.destination_label || "—";
  document.getElementById("info-folder").textContent =
    config.watched_folder?.replace(/.*[\\/]/, "~/") || "—";
  document.getElementById("info-last-sync").textContent = config.last_sync
    ? formatRelativeTime(config.last_sync)
    : "Never";
}

document.getElementById("btn-open-folder").addEventListener("click", () => {
  invoke("open_watched_folder");
});

document.getElementById("btn-open-drive").addEventListener("click", () => {
  invoke("open_drive");
});

// ── Sync status updates from Rust ─────────────────────────────────────

import { listen } from "@tauri-apps/api/event";

listen("sync-status", (event) => {
  const badge = document.getElementById("status-badge");
  const { status, last_sync } = event.payload;

  badge.className = "";
  if (status === "syncing") {
    badge.classList.add("status-sync");
    badge.textContent = "● Syncing…";
  } else if (status === "error") {
    badge.classList.add("status-error");
    badge.textContent = "● Error";
  } else {
    badge.classList.add("status-idle");
    badge.textContent = "● Synced";
    if (last_sync) {
      document.getElementById("info-last-sync").textContent =
        formatRelativeTime(last_sync);
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
