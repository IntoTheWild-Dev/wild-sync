# Setup & Development Guide

Complete guide for initial setup and running **Wild Sync** locally.

---

## 1. Tech Stack

### Framework & Runtime
- **[Tauri v2](https://tauri.app)** — desktop app framework, Rust backend + native webview (not Electron/bundled Chromium), ~30MB binary output
- **Rust** (edition 2021) — backend language
- **Vanilla HTML/CSS/JS** — frontend, no framework (no React/Vue)
- **[Vite 6](https://vitejs.dev)** — frontend dev server & bundler

### Rust crates (`src-tauri/Cargo.toml`)
| Crate | Purpose |
|---|---|
| `tauri` (feature `tray-icon`) | Core Tauri + system tray |
| `tauri-plugin-dialog` | Native file/folder picker |
| `tauri-plugin-fs` | Filesystem access from the frontend |
| `tauri-plugin-shell` | Runs the sidecar binary (rclone) |
| `tauri-plugin-store` | Persists local state/preferences |
| `tauri-plugin-opener` | Opens files/folders/links via the OS |
| `tauri-plugin-autostart` | Launch at startup |
| `tauri-plugin-notification` | Native OS notifications |
| `serde` / `serde_json` | Data serialization (Rust ↔ JS) |
| `tokio` | Async runtime |
| `notify-debouncer-mini` | Watches file changes in the local folder (debounced) |
| `chrono` | Date/time handling |

### JS packages (`package.json`)
| Package | Purpose |
|---|---|
| `@tauri-apps/api` | JS ↔ Rust bridge (invoke commands, events) |
| `@tauri-apps/plugin-dialog`, `-fs`, `-opener`, `-shell`, `-store` | JS bindings for the Rust plugins above |
| `@tauri-apps/cli` | CLI for `tauri dev` / `tauri build` |

### External tool
- **[rclone](https://rclone.org)** — bundled as a *sidecar binary* (not a library), handles the entire Google Drive sync process
- **Google Workspace Shared Drive** — destination storage, accessed via a **service account** (JSON key), so no manual Google login is required

### CI/CD
- **GitHub Actions** (`.github/`) — automatically builds Mac (.dmg) & Windows (.exe) installers when a `v*` tag is pushed, then creates a draft release

---

## 2. Prerequisites

Install the following tools before you start:

| Tool | Version | Link |
|---|---|---|
| Rust | latest stable | [rustup.rs](https://rustup.rs) |
| Node.js | 20+ | [nodejs.org](https://nodejs.org) |
| npm | bundled with Node.js | — |

Verify the install:

```bash
rustc --version
node --version
npm --version
```

### Extra per-OS dependencies (for Tauri)

- **macOS**: Xcode Command Line Tools — `xcode-select --install`
- **Windows**: [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) + WebView2 (usually already present on Windows 10/11)

Full reference: [Tauri Prerequisites](https://tauri.app/start/prerequisites/)

---

## 3. Clone the repo & install dependencies

```bash
git clone https://github.com/IntoTheWild-Dev/wild-sync.git
cd wild-sync
npm install
```

---

## 4. Files you need to set up manually

These two file types are **not in the repo** (intentionally `.gitignore`d) and must be in place before `npm run tauri dev` will work.

### a. rclone binaries

Download from [rclone.org/downloads](https://rclone.org/downloads), then place them in `src-tauri/binaries/` with names matching the target platform:

| File | Platform |
|---|---|
| `rclone-aarch64-apple-darwin` | Mac Apple Silicon (M1/M2/M3/M4) |
| `rclone-x86_64-apple-darwin` | Mac Intel |
| `rclone-x86_64-pc-windows-msvc.exe` | Windows |

You only need the binary for the platform you're developing on. After downloading, make sure it's executable (on Mac/Linux):

```bash
chmod +x src-tauri/binaries/rclone-*
```

### b. Service account JSON

Ask an admin for the `wild-sync-service-account.json` file (this project uses a Google Workspace service account, not a regular user login).

Place it at:

```
src-tauri/resources/wild-sync-service-account.json
```

> ⚠️ **Never commit this file.** It's already `.gitignore`d, but double-check before `git add` anyway.

---

## 5. Run locally

This project does **not** auto-start the frontend dev server (`beforeDevCommand` isn't configured), so you need **two terminals**.

**Terminal 1** — start the Vite dev server first, and leave it running:
```bash
npm run dev
```

**Terminal 2** — once Vite is up on `http://localhost:1420`, start Tauri:
```bash
npm run tauri dev
```

If you run `npm run tauri dev` first, it will just print `Waiting for your frontend dev server to start on http://localhost:1420/...` in a loop until Terminal 1 is running — that's expected, not an error.

This will:
1. Serve the frontend via Vite at `http://localhost:1420`
2. Compile the Rust backend and open the Tauri window
3. Auto-reload whenever files in `src/` or `src-tauri/src/` change

On first launch, the app will ask you to pick a name and a local watch folder (see [README.md](README.md) for the usage flow).

---

## 6. Project structure at a glance

```
wild-sync/
├── src/                    # Frontend (vanilla HTML/CSS/JS)
│   ├── index.html
│   ├── main.js
│   └── style.css
├── src-tauri/              # Rust backend (Tauri v2)
│   ├── src/                # Rust source
│   ├── binaries/           # rclone sidecar (manual, gitignored)
│   ├── resources/          # service account + destinations.json (partially gitignored)
│   ├── capabilities/       # Tauri permission config
│   └── tauri.conf.json     # App config (window, tray, bundle)
└── package.json
```

---

## 7. Troubleshooting

**`npm run tauri dev` fails with rclone binary not found**
→ Make sure the binary filename in `src-tauri/binaries/` exactly matches the table above and is `chmod +x`.

**App window opens but is completely blank/white**
→ Vite's `root` isn't pointed at `src/`, so it can't find `index.html`. Check `vite.config.js` at the project root has `root: "src"`. Confirm by opening `http://localhost:1420/` directly in a browser — it should show the app HTML, not a blank page.

**Dialog: "rclone Not Opened — Apple could not verify..."**
→ macOS Gatekeeper quarantine flag on the downloaded rclone binary. Click **Done** (not "Move to Trash"), then run:
```bash
xattr -cr src-tauri/binaries/rclone-*
```
If the error persists after restarting `npm run tauri dev`, Tauri may have already copied the binary into the build output **before** you cleared the flag — that copy needs it removed too:
```bash
xattr -cr src-tauri/target/debug/rclone
```
(No app rebuild needed — just reselect the destination dropdown in the app to retry.)

**"Your Name" dropdown shows "⚠ rclone error:" with no further detail**
→ The Rust error message only surfaces rclone's stderr, which can be empty if the process was killed by Gatekeeper rather than exiting normally — see the quarantine fix above. If stderr *does* have content, check the next item.

**rclone error containing `401 Unauthorized: unauthorized_client`**
→ The service account needs **Domain-Wide Delegation** authorized in Google Workspace Admin (`admin.google.com` → Security → API Controls → Domain-wide Delegation), separate from GCP/Drive sharing. This requires a Workspace **Super Admin** (not just a GCP project Editor/Owner, and not just Shared Drive Manager). They need to add the service account's OAuth2 Client ID (found in its JSON key as `client_id`, or in GCP under the service account's Details) with scope `https://www.googleapis.com/auth/drive`. This impersonates `julia@intothewild.hamburg` per `src-tauri/src/lib.rs`'s `write_rclone_config` — that identity needs to already have access to the Shared Drive.

**Errors related to service account / Google Drive auth (other)**
→ Check that `src-tauri/resources/wild-sync-service-account.json` exists and is valid, and that the service account has access to the Shared Drive configured in `src-tauri/resources/destinations.json`.

**Rust build is slow / dependency errors**
→ Run `cd src-tauri && cargo clean`, then retry `npm run tauri dev`.

**Window doesn't appear / stuck in tray**
→ This is expected — the app is designed to live in the tray/menu bar. Click the tray icon to open the main window.

---

## 8. Build & Release

Production build (no release):

```bash
npm run tauri build
```

For an official release (triggers GitHub Actions to build Mac + Windows installers):

```bash
git tag v0.x.x
git push origin v0.x.x
```

See [README.md](README.md#release-a-new-version) for further details.
