use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup_tray(app.handle())?;

            // Show onboarding window on first launch.
            // The frontend checks local store for existing config and
            // either shows onboarding or goes straight to tray mode.
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

fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "status", "Status: Starting...", false, None::<&str>)?;
    let separator1 = PredefinedMenuItem::separator(app)?;
    let open_folder = MenuItem::with_id(app, "open_folder", "Open Watched Folder", true, None::<&str>)?;
    let open_drive = MenuItem::with_id(app, "open_drive", "Open on Drive", true, None::<&str>)?;
    let separator2 = PredefinedMenuItem::separator(app)?;
    let change_name = MenuItem::with_id(app, "change_name", "Change Name", true, None::<&str>)?;
    let change_dest = MenuItem::with_id(app, "change_dest", "Change Destination", true, None::<&str>)?;
    let separator3 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Wild Sync", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &status,
            &separator1,
            &open_folder,
            &open_drive,
            &separator2,
            &change_name,
            &change_dest,
            &separator3,
            &quit,
        ],
    )?;

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
            "open_folder" => {
                // TODO: open watched folder path from store
            }
            "open_drive" => {
                // TODO: open drive URL from store using opener plugin
            }
            "change_name" | "change_dest" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    // TODO: emit event to frontend to show change screen
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

// ── Tauri commands called from the frontend ───────────────────────────────────

#[tauri::command]
async fn fetch_destinations(app: AppHandle) -> Result<serde_json::Value, String> {
    // TODO: use rclone lsjson or Drive API to fetch destinations.json
    // from the root of Wild Sync Archives - Team (TEAM_DRIVE_ID)
    todo!("fetch destinations.json from Team Drive")
}

#[tauri::command]
async fn fetch_designer_names(
    app: AppHandle,
    rclone_remote: String,
    drive_id: String,
) -> Result<Vec<String>, String> {
    // TODO: run rclone lsjson "[rclone_remote]:" --dirs-only
    // parse JSON response, return folder names as Vec<String>
    todo!("fetch top-level folder names from selected drive")
}

#[tauri::command]
async fn fetch_project_names(
    app: AppHandle,
    rclone_remote: String,
    drive_id: String,
    designer_name: String,
) -> Result<Vec<String>, String> {
    // TODO: run rclone lsjson "[rclone_remote]:[designer_name]/" --dirs-only
    // parse JSON response, return subfolder names as Vec<String>
    todo!("fetch project folder names from designer's Drive folder")
}

#[tauri::command]
async fn save_config(
    app: AppHandle,
    designer_name: String,
    project_name: String,
    watched_folder: String,
    destination_label: String,
    destination_drive_id: String,
    rclone_remote: String,
) -> Result<(), String> {
    // TODO: persist to tauri-plugin-store
    // also generate rclone config file entries for all destinations
    todo!("save config to store")
}

#[tauri::command]
async fn get_config(app: AppHandle) -> Result<Option<serde_json::Value>, String> {
    // TODO: read from tauri-plugin-store, return None if not yet configured
    todo!("read config from store")
}

#[tauri::command]
async fn start_watching(app: AppHandle) -> Result<(), String> {
    // TODO: use tauri-plugin-fs watch() on the configured watched_folder
    // on file change events, debounce 5s then spawn rclone copy
    // parse rclone --use-json-log output to update tray icon state
    todo!("start file watcher with debounce")
}

#[tauri::command]
async fn open_watched_folder(app: AppHandle) -> Result<(), String> {
    // TODO: get path from store, open with opener plugin
    todo!("open watched folder in Finder/Explorer")
}

#[tauri::command]
async fn open_drive(app: AppHandle) -> Result<(), String> {
    // TODO: get drive_id from store, open https://drive.google.com/drive/folders/[drive_id]
    todo!("open Shared Drive in browser")
}
