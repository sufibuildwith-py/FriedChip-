use tauri::State;
use serde::{Deserialize, Serialize};
use crate::db::Database;
use crate::models::{AppSetting, FirmwareFile, Operation, OperationLog, Workspace};
use crate::services::WorkspaceService;

#[derive(Serialize, Deserialize)]
pub struct SystemInfo {
    pub engine_version: String,
    pub database_path: String,
    pub storage_root: String,
    pub rust_version: String,
    pub target_os: String,
}

pub struct AppState {
    pub db: Database,
    pub workspace_service: WorkspaceService,
    pub storage_dir: std::path::PathBuf,
}

#[tauri::command]
pub async fn get_active_workspace(state: State<'_, AppState>) -> Result<Workspace, String> {
    state
        .workspace_service
        .ensure_default_workspace(&state.storage_dir)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_workspaces(state: State<'_, AppState>) -> Result<Vec<Workspace>, String> {
    state.db.list_workspaces().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_workspace(
    name: String,
    root_path: String,
    state: State<'_, AppState>,
) -> Result<Workspace, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let ws = Workspace {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        root_path,
        created_at: now.clone(),
        updated_at: now,
        is_default: false,
    };
    state.db.create_workspace(&ws).map_err(|e| e.to_string())?;
    Ok(ws)
}

#[tauri::command]
pub async fn import_firmware_file(
    workspace_id: String,
    file_path: String,
    state: State<'_, AppState>,
) -> Result<FirmwareFile, String> {
    state
        .workspace_service
        .import_firmware_file(&workspace_id, file_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn verify_source_file(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<FirmwareFile, String> {
    state
        .workspace_service
        .verify_source_file(&file_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_workspace_files(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<FirmwareFile>, String> {
    state
        .workspace_service
        .list_files(&workspace_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_operations(
    workspace_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Operation>, String> {
    state
        .workspace_service
        .list_operations(&workspace_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_operation_logs(
    operation_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<OperationLog>, String> {
    state
        .workspace_service
        .get_operation_logs(&operation_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Vec<AppSetting>, String> {
    state.db.get_all_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_setting(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.db.set_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn inspect_firmware_regions(file_path: String) -> Result<crate::engine::binary::ifd::IfdAnalysisReport, String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    crate::engine::binary::ifd::IntelFlashDescriptor::parse(&bytes).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn is_dell_pfs_package(file_path: String) -> Result<bool, String> {
    let path = std::path::Path::new(&file_path);
    if !path.exists() {
        return Ok(false);
    }
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    Ok(crate::engine::vendor::dell_pfs::DellPfsParser::is_dell_pfs(&bytes))
}

#[tauri::command]
pub async fn extract_dell_pfs(
    workspace_id: String,
    file_id: String,
    custom_output_dir: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::engine::vendor::dell_pfs::DellPfsExtractionReport, String> {
    let file = state.db.get_firmware_file(&file_id).map_err(|e| e.to_string())?;
    let src_path = std::path::Path::new(&file.original_path);

    if !src_path.exists() {
        return Err(format!("Source file does not exist on disk: {}", file.original_path));
    }

    let bytes = std::fs::read(src_path).map_err(|e| format!("Failed to read source: {}", e))?;

    // Determine extraction directory
    let output_dir = if let Some(custom) = custom_output_dir {
        std::path::PathBuf::from(custom)
    } else {
        state.storage_dir
            .join("workspaces")
            .join(&workspace_id)
            .join("extracted")
            .join(&file.id)
    };

    std::fs::create_dir_all(&output_dir).map_err(|e| format!("Failed to create output dir: {}", e))?;

    // Check path distinctness
    crate::engine::binary::verified_write::ensure_distinct_paths(src_path, &output_dir)
        .map_err(|e| e.to_string())?;

    // Create Operation record
    let op_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let op = Operation {
        id: op_id.clone(),
        workspace_id: workspace_id.clone(),
        operation_type: "DELL_PFS_EXTRACTION".to_string(),
        input_file_id: Some(file.id.clone()),
        input_sha256: Some(file.sha256.clone()),
        status: crate::models::OperationStatus::Running,
        created_at: now.clone(),
        completed_at: None,
        error_message: None,
    };
    let _ = state.db.create_operation(&op);

    let report_res = crate::engine::vendor::dell_pfs::DellPfsParser::extract(&bytes, Some(&output_dir), &file.filename);

    match report_res {
        Ok(report) => {
            let completed_at = chrono::Utc::now().to_rfc3339();
            let _ = state.db.update_operation_status(
                &op_id,
                crate::models::OperationStatus::Completed,
                Some(&completed_at),
                None,
            );

            // Register extracted components into SQLite workspace library
            for comp in &report.components {
                if let Some(ref disk_path) = comp.file_path {
                    let comp_path = std::path::Path::new(disk_path);
                    let ext = comp_path.extension().and_then(|s| s.to_str()).unwrap_or("bin").to_string();
                    let comp_file = FirmwareFile {
                        id: uuid::Uuid::new_v4().to_string(),
                        workspace_id: workspace_id.clone(),
                        original_path: disk_path.clone(),
                        filename: comp.name.clone(),
                        extension: ext,
                        size_bytes: comp.raw_size as u64,
                        sha256: comp.sha256.clone(),
                        md5: comp.md5.clone(),
                        detected_format: crate::models::FirmwareFormat::Unclassified,
                        format_confidence: crate::models::FormatConfidence::Confirmed,
                        import_timestamp: completed_at.clone(),
                        is_read_only: true,
                    };
                    // Ignore duplicate errors if re-extracting
                    let _ = state.db.insert_firmware_file(&comp_file);
                }
            }

            // Log details
            let log = OperationLog {
                id: 0,
                operation_id: op_id,
                timestamp: completed_at,
                level: "INFO".to_string(),
                message: format!(
                    "Dell PFS extraction succeeded. {} sections parsed, {} components extracted to {}",
                    report.total_sections_found,
                    report.components.len(),
                    output_dir.display()
                ),
            };
            let _ = state.db.append_operation_log(&log);

            Ok(report)
        }
        Err(err) => {
            let completed_at = chrono::Utc::now().to_rfc3339();
            let err_msg = err.to_string();
            let _ = state.db.update_operation_status(
                &op_id,
                crate::models::OperationStatus::Failed,
                Some(&completed_at),
                Some(&err_msg),
            );
            let log = OperationLog {
                id: 0,
                operation_id: op_id,
                timestamp: completed_at,
                level: "ERROR".to_string(),
                message: format!("Dell PFS extraction failed: {}", err_msg),
            };
            let _ = state.db.append_operation_log(&log);
            Err(err_msg)
        }
    }
}

#[tauri::command]
pub async fn get_system_info(state: State<'_, AppState>) -> Result<SystemInfo, String> {
    let db_path = state.storage_dir.join("metadata.db").to_string_lossy().to_string();
    let storage_root = state.storage_dir.to_string_lossy().to_string();

    Ok(SystemInfo {
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
        database_path: db_path,
        storage_root,
        rust_version: "1.99.1 (MSVC)".to_string(),
        target_os: "Windows x86_64".to_string(),
    })
}
