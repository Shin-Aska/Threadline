pub mod accounts;
pub mod commands;
pub mod composer;
pub mod credentials;
pub mod database;
pub mod error;
pub mod models;
pub mod providers;
use database::Database;
pub struct AppState {
    pub database: Database,
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = Database::open(&dir.join("threadline.sqlite"))
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            db.seed(&accounts::mock_accounts())
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            app.manage(AppState { database: db });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::preview_post,
            commands::publish_post,
            commands::storage_health
        ])
        .run(tauri::generate_context!())
        .expect("error while running Threadline")
}
