# Wild Sync

Lightweight tray app for Mac and Windows. Watches a local folder and automatically archives files to a Google Workspace Shared Drive — no Google login required.

Built for Into The Wild Design Agency.

---

## How it works

1. On first launch, pick your name and a local **watch folder**
2. Inside that folder, create a subfolder per client — e.g. `FNB Rebrand/`, `Vodacom 2026/`
3. Wild Sync watches everything. When files change inside a client folder, that folder syncs automatically to Drive under `YourName/ClientFolder/`
4. The app lives in your menu bar / system tray silently after that

```
~/Wild Sync Watch/
├── Project 1/        →  Drive: Your Name/Project 1/
├── Project 2/        →  Drive: Your Name/Project 2/
└── Project 3/        →  Drive: Your Name/Project 3/
```

No project setup needed — just create a subfolder and drop files in.

---

## Download

Go to [Releases](https://github.com/IntoTheWild-Dev/wild-sync/releases/latest) and download:

| Platform | File |
|---|---|
| Mac (Apple Silicon — M1/M2/M3/M4) | `Wild.Sync_x.x.x_aarch64.dmg` |
| Mac (Intel) | `Wild.Sync_x.x.x_x64.dmg` |
| Windows | `Wild.Sync_x.x.x_x64-setup.exe` |

---

## First time on Mac

Mac blocks apps not from the App Store. Follow these steps once:

1. Open the `.dmg` and drag Wild Sync to your Applications folder
2. Open **Terminal** (search for it in Spotlight with `Cmd + Space`)
3. Paste this command and press Enter:
   ```bash
   xattr -cr "/Applications/Wild Sync.app"
   ```
4. Now open Wild Sync from your Applications folder — it will launch normally

> This command removes the quarantine flag Apple places on downloaded apps.
> You only need to do this once. Wild Sync opens normally every time after that.

**If the app still doesn't open**, run this in Terminal to see what's happening:
```bash
/Applications/Wild\ Sync.app/Contents/MacOS/wild-sync
```
Any error will print directly — send it to your admin to get it sorted.

---

## Tech stack

- [Tauri v2](https://tauri.app) — Rust backend, ~30MB binary
- Vanilla HTML/CSS/JS frontend
- [rclone](https://rclone.org) bundled as sidecar — handles all Google Drive sync
- Service account JSON baked into the app — no user login needed

---

## For developers

### Prerequisites

- [Rust](https://rustup.rs)
- [Node.js](https://nodejs.org) 20+
- rclone binaries (see below)
- `wild-sync-service-account.json` from the admin

### Setup

```bash
npm install
```

Place the following in `src-tauri/binaries/` (download from [rclone.org/downloads](https://rclone.org/downloads)):

| File | Platform |
|---|---|
| `rclone-aarch64-apple-darwin` | Mac Apple Silicon |
| `rclone-x86_64-apple-darwin` | Mac Intel |
| `rclone-x86_64-pc-windows-msvc.exe` | Windows |

Place `wild-sync-service-account.json` in `src-tauri/resources/` — **never commit this file**.

### Run locally

```bash
npm run tauri dev
```

### Release a new version

```bash
git tag v0.x.x
git push origin v0.x.x
```

GitHub Actions builds Mac and Windows installers automatically and creates a draft release. Review it at [Releases](https://github.com/IntoTheWild-Dev/wild-sync/releases) and click **Publish**.
