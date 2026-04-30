use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup_tray(app.handle())?;
            let window = app.get_webview_window("main").unwrap();
            window.show().unwrap();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            fetch_destinations,
            fetch_designer_names,
            fetch_project_names,
            save_config,
            get_config,
            start_watching,
            open_watched_folder,
            open_drive,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wild Sync");
}

// Use concrete AppHandle (= AppHandle<Wry>) to avoid generic trait-bound issues
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let status       = MenuItem::with_id(app, "status",      "Status: Starting...",   false, None::<&str>)?;
    let sep1         = PredefinedMenuItem::separator(app)?;
    let folder_item  = MenuItem::with_id(app, "open_folder", "Open Watched Folder",   true,  None::<&str>)?;
    let drive_item   = MenuItem::with_id(app, "open_drive",  "Open on Drive",         true,  None::<&str>)?;
    let sep2         = PredefinedMenuItem::separator(app)?;
    let quit         = MenuItem::with_id(app, "quit",        "Quit Wild Sync",        true,  None::<&str>)?;

    let menu = Menu::with_items(app, &[&status, &sep1, &folder_item, &drive_item, &sep2, &quit])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_folder" => { let _ = do_open_folder(app); }
            "open_drive"  => { let _ = do_open_drive_url(app); }
            "quit"        => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

// ── Sync helpers used by both tray menu and Tauri commands ────────────────────

fn do_open_folder(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;
    use tauri_plugin_opener::OpenerExt;
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    let folder = store.get("watched_folder")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or("No folder configured")?;
    app.opener().open_path(folder, None::<&str>).map_err(|e| e.to_string())
}

fn do_open_drive_url(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;
    use tauri_plugin_opener::OpenerExt;
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    let drive_id = store.get("destination_drive_id")
        .and_then(|v| v.as_str().map(String::from))
        .ok_or("No drive configured")?;
    let url = format!("https://drive.google.com/drive/folders/{drive_id}");
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

// ── rclone helpers ────────────────────────────────────────────────────────────

fn sa_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .resource_dir()
        .map_err(|e| e.to_string())
        .map(|p| p.join("wild-sync-service-account.json"))
}

fn write_rclone_config(sa: &std::path::Path, remote_name: &str, drive_id: &str) -> Result<std::path::PathBuf, String> {
    let config_path = std::env::temp_dir().join("wild-sync-rclone.conf");
    let content = format!(
        "[{remote_name}]\ntype = drive\nscope = drive\nservice_account_file = {}\nteam_drive = {drive_id}\n",
        sa.display()
    );
    std::fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write rclone config: {e}"))?;
    Ok(config_path)
}

async fn run_rclone(app: &AppHandle, args: Vec<String>) -> Result<String, String> {
    let (mut rx, _child) = app
        .shell()
        .sidecar("rclone")
        .map_err(|e| e.to_string())?
        .args(&args)
        .spawn()
        .map_err(|e| e.to_string())?;

    let mut stdout = String::new();
    let mut stderr = String::new();

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(bytes) => stdout.push_str(&String::from_utf8_lossy(&bytes)),
            CommandEvent::Stderr(bytes) => stderr.push_str(&String::from_utf8_lossy(&bytes)),
            CommandEvent::Terminated(status) => {
                if status.code != Some(0) {
                    return Err(format!("rclone error: {stderr}"));
                }
                break;
            }
            _ => {}
        }
    }
    Ok(stdout)
}

fn parse_lsjson_names(json: &str) -> Result<Vec<String>, String> {
    let items: Vec<serde_json::Value> = serde_json::from_str(json)
        .map_err(|e| format!("Failed to parse rclone output: {e}"))?;
    Ok(items.iter()
        .filter_map(|item| {
            if item["IsDir"].as_bool().unwrap_or(false) {
                item["Name"].as_str().map(String::from)
            } else {
                None
            }
        })
        .collect())
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
async fn fetch_destinations(app: AppHandle) -> Result<serde_json::Value, String> {
    let path = app.path().resource_dir().map_err(|e| e.to_string())?.join("destinations.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read destinations.json: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse destinations.json: {e}"))
}

#[tauri::command]
async fn fetch_designer_names(app: AppHandle, rclone_remote: String, drive_id: String) -> Result<Vec<String>, String> {
    let sa = sa_path(&app)?;
    let config = write_rclone_config(&sa, &rclone_remote, &drive_id)?;
    let output = run_rclone(&app, vec![
        "--config".to_string(), config.to_string_lossy().to_string(),
        "lsjson".to_string(), format!("{rclone_remote}:"),
        "--dirs-only".to_string(), "--no-modtime".to_string(), "--no-mimetype".to_string(),
    ]).await?;
    parse_lsjson_names(&output)
}

#[tauri::command]
async fn fetch_project_names(app: AppHandle, rclone_remote: String, drive_id: String, designer_name: String) -> Result<Vec<String>, String> {
    let sa = sa_path(&app)?;
    let config = write_rclone_config(&sa, &rclone_remote, &drive_id)?;
    let output = run_rclone(&app, vec![
        "--config".to_string(), config.to_string_lossy().to_string(),
        "lsjson".to_string(), format!("{rclone_remote}:{designer_name}"),
        "--dirs-only".to_string(), "--no-modtime".to_string(), "--no-mimetype".to_string(),
    ]).await?;
    parse_lsjson_names(&output)
}

#[tauri::command]
async fn save_config(
    app: AppHandle,
    designer_name: String, project_name: String, watched_folder: String,
    destination_label: String, destination_drive_id: String, rclone_remote: String,
) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("designer_name",       serde_json::json!(designer_name));
    store.set("project_name",        serde_json::json!(project_name));
    store.set("watched_folder",      serde_json::json!(watched_folder));
    store.set("destination_label",   serde_json::json!(destination_label));
    store.set("destination_drive_id",serde_json::json!(destination_drive_id));
    store.set("rclone_remote",       serde_json::json!(rclone_remote));
    store.save().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_config(app: AppHandle) -> Result<Option<serde_json::Value>, String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    if !store.has("designer_name") { return Ok(None); }
    Ok(Some(serde_json::json!({
        "designer_name":        store.get("designer_name"),
        "project_name":         store.get("project_name"),
        "watched_folder":       store.get("watched_folder"),
        "destination_label":    store.get("destination_label"),
        "destination_drive_id": store.get("destination_drive_id"),
        "rclone_remote":        store.get("rclone_remote"),
        "last_sync":            store.get("last_sync"),
    })))
}

#[tauri::command]
async fn start_watching(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_store::StoreExt;
    let store = app.store("config.json").map_err(|e| e.to_string())?;

    let get = |key: &str| -> Result<String, String> {
        store.get(key).and_then(|v| v.as_str().map(String::from))
            .ok_or_else(|| format!("Missing config: {key}"))
    };

    let watched_folder = get("watched_folder")?;
    let designer_name  = get("designer_name")?;
    let project_name   = get("project_name")?;
    let drive_id       = get("destination_drive_id")?;
    let rclone_remote  = get("rclone_remote")?;

    let app_clone = app.clone();

    std::thread::spawn(move || {
        use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};

        let (tx, rx) = std::sync::mpsc::channel();
        let mut debouncer = match new_debouncer(Duration::from_secs(5), move |res| { let _ = tx.send(res); }) {
            Ok(d) => d,
            Err(e) => { eprintln!("Watcher init failed: {e}"); return; }
        };

        if let Err(e) = debouncer.watcher().watch(std::path::Path::new(&watched_folder), RecursiveMode::Recursive) {
            eprintln!("Watch failed: {e}"); return;
        }

        for result in rx {
            if result.is_err() { continue; }

            let app     = app_clone.clone();
            let folder  = watched_folder.clone();
            let designer = designer_name.clone();
            let project  = project_name.clone();
            let drive    = drive_id.clone();
            let remote   = rclone_remote.clone();

            let _ = app.emit("sync-status", serde_json::json!({ "status": "syncing" }));

            tauri::async_runtime::spawn(async move {
                match sync_files(&app, &folder, &designer, &project, &drive, &remote).await {
                    Ok(_) => {
                        let now = chrono::Utc::now().to_rfc3339();
                        if let Ok(store) = app.store("config.json") {
                            store.set("last_sync", serde_json::json!(&now));
                            let _ = store.save();
                        }
                        let _ = app.emit("sync-status", serde_json::json!({ "status": "idle", "last_sync": now }));
                    }
                    Err(e) => {
                        eprintln!("Sync error: {e}");
                        let _ = app.emit("sync-status", serde_json::json!({ "status": "error" }));
                    }
                }
            });
        }
    });

    Ok(())
}

async fn sync_files(app: &AppHandle, local_folder: &str, designer_name: &str, project_name: &str, drive_id: &str, rclone_remote: &str) -> Result<(), String> {
    let sa = sa_path(app)?;
    let config = write_rclone_config(&sa, rclone_remote, drive_id)?;
    run_rclone(app, vec![
        "--config".to_string(), config.to_string_lossy().to_string(),
        "copy".to_string(), local_folder.to_string(),
        format!("{rclone_remote}:{designer_name}/{project_name}"),
        "--use-json-log".to_string(), "--log-level".to_string(), "INFO".to_string(),
    ]).await?;
    Ok(())
}

#[tauri::command]
async fn open_watched_folder(app: AppHandle) -> Result<(), String> {
    do_open_folder(&app)
}

#[tauri::command]
async fn open_drive(app: AppHandle) -> Result<(), String> {
    do_open_drive_url(&app)
}
