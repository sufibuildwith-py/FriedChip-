pub mod error;
pub mod models;
pub mod db;
pub mod services;
pub mod commands;
pub mod engine;

#[cfg(test)]
pub mod tests;

use std::path::PathBuf;
use tauri::Manager;
use crate::commands::AppState;
use crate::db::Database;
use crate::services::WorkspaceService;

pub fn run() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Panic in LBE Workstation: {:?}\n", info);
        let _ = std::fs::write("crash_panic.log", &msg);
        let _ = std::fs::write(std::env::temp_dir().join("lbe_crash_panic.log"), &msg);
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Determine storage directory: try app_data_dir or fallback to current exe dir / storage
            let storage_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("./storage"));

            std::fs::create_dir_all(&storage_dir)?;
            let db_path = storage_dir.join("workstation.db");

            let db = Database::open_or_create(&db_path)?;
            let workspace_service = WorkspaceService::new(db.clone());

            // Ensure initial default workspace
            let _ = workspace_service.ensure_default_workspace(&storage_dir);

            app.manage(AppState {
                db,
                workspace_service,
                storage_dir,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_active_workspace,
            commands::list_workspaces,
            commands::create_workspace,
            commands::import_firmware_file,
            commands::verify_source_file,
            commands::list_workspace_files,
            commands::list_operations,
            commands::get_operation_logs,
            commands::get_settings,
            commands::update_setting,
            commands::get_system_info,
            commands::inspect_firmware_regions,
            commands::is_dell_pfs_package,
            commands::extract_dell_pfs
        ])
        .run(tauri::generate_context!())
        .expect("error while running LBE Workstation application");
}
