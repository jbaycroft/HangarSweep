use std::sync::Mutex;
use sqlx::SqlitePool;
use tauri::Manager;

pub mod auth;
pub mod commands;
pub mod db;
pub mod esi;

/// In-flight PKCE auth state kept between browser redirect and callback.
pub struct PendingAuth {
    pub verifier: String,
    pub state: String,
}

/// Global application state injected into every Tauri command.
pub struct AppState {
    pub db: SqlitePool,
    pub pending_auth: Mutex<Option<PendingAuth>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let handle = app.handle().clone();
            // Initialise DB synchronously inside the async runtime that Tauri already starts
            let db = tauri::async_runtime::block_on(db::init_db(&handle))
                .expect("Failed to initialise SQLite database");

            app.manage(AppState {
                db,
                pending_auth: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::get_characters,
            commands::delete_character,
            commands::sync_all,
            commands::get_liquidity_summary,
            commands::get_assets_at_location,
            commands::export_multibuy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
