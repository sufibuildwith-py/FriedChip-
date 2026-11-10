use std::path::Path;
use chrono::Utc;
use uuid::Uuid;
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::models::{FirmwareFile, Operation, OperationLog, OperationStatus, Workspace};
use crate::services::file_service::FileService;

pub struct WorkspaceService {
    db: Database,
}

impl WorkspaceService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Ensure at least one default workspace exists
    pub fn ensure_default_workspace<P: AsRef<Path>>(&self, default_path: P) -> AppResult<Workspace> {
        if let Some(ws) = self.db.get_default_workspace()? {
            return Ok(ws);
        }

        let now = Utc::now().to_rfc3339();
        let ws = Workspace {
            id: Uuid::new_v4().to_string(),
            name: "Default Workspace".to_string(),
            root_path: default_path.as_ref().to_string_lossy().to_string(),
            created_at: now.clone(),
            updated_at: now,
            is_default: true,
        };

        self.db.create_workspace(&ws)?;
        Ok(ws)
    }

    /// Safe import workflow:
    /// 1. Inspect source file (opens strictly read-only, computes SHA-256 + MD5, detects signature)
    /// 2. Check for duplicate SHA-256 in workspace
    /// 3. Create structured operation record
    /// 4. Log validation details
    /// 5. Commit metadata to SQLite
    /// 6. Mark operation completed
    pub fn import_firmware_file<P: AsRef<Path>>(
        &self,
        workspace_id: &str,
        file_path: P,
    ) -> AppResult<FirmwareFile> {
        let path_ref = file_path.as_ref();
        let now = Utc::now().to_rfc3339();
        let op_id = Uuid::new_v4().to_string();

        // 1. Initial Operation Record (Status: RUNNING)
        let mut op = Operation {
            id: op_id.clone(),
            workspace_id: workspace_id.to_string(),
            operation_type: "FILE_IMPORT".to_string(),
            input_file_id: None,
            input_sha256: None,
            status: OperationStatus::Running,
            created_at: now.clone(),
            completed_at: None,
            error_message: None,
        };
        self.db.create_operation(&op)?;

        self.log_op(&op_id, "INFO", &format!("Initiating safe read-only import of file: {}", path_ref.display()))?;

        // 2. Perform safe inspection
        let inspection = match FileService::inspect_file(path_ref) {
            Ok(res) => res,
            Err(e) => {
                let err_msg = e.to_string();
                self.log_op(&op_id, "ERROR", &format!("Inspection failed: {}", err_msg))?;
                self.db.update_operation_status(&op_id, OperationStatus::Failed, Some(&Utc::now().to_rfc3339()), Some(&err_msg))?;
                return Err(e);
            }
        };

        op.input_sha256 = Some(inspection.sha256.clone());
        self.log_op(&op_id, "INFO", &format!("Computed primary SHA-256: {}", inspection.sha256))?;
        self.log_op(&op_id, "INFO", &format!("Computed legacy MD5: {}", inspection.md5))?;
        self.log_op(&op_id, "INFO", &format!("File size: {} bytes ({:.2} MB)", inspection.size_bytes, inspection.size_bytes as f64 / 1_048_576.0))?;
        self.log_op(&op_id, "INFO", &format!("Signature Analysis: {} [Confidence: {}]", inspection.detected_format.to_string(), inspection.format_confidence.to_string()))?;

        // 3. Duplicate check
        if let Some(existing) = self.db.find_file_by_sha256(workspace_id, &inspection.sha256)? {
            let err_msg = format!("File with identical SHA-256 already imported (ID: {}, Filename: {})", existing.id, existing.filename);
            self.log_op(&op_id, "WARN", &err_msg)?;
            self.db.update_operation_status(&op_id, OperationStatus::Failed, Some(&Utc::now().to_rfc3339()), Some(&err_msg))?;
            return Err(AppError::Duplicate(err_msg));
        }

        // 4. Create FirmwareFile model
        let file_id = Uuid::new_v4().to_string();
        let firmware_file = FirmwareFile {
            id: file_id.clone(),
            workspace_id: workspace_id.to_string(),
            original_path: path_ref.to_string_lossy().to_string(),
            filename: inspection.filename,
            extension: inspection.extension,
            size_bytes: inspection.size_bytes,
            sha256: inspection.sha256,
            md5: inspection.md5,
            detected_format: inspection.detected_format,
            format_confidence: inspection.format_confidence,
            import_timestamp: now,
            is_read_only: inspection.is_read_only,
        };

        // 5. Commit to database
        if let Err(e) = self.db.insert_firmware_file(&firmware_file) {
            let err_msg = format!("Database persist failed: {}", e);
            self.log_op(&op_id, "ERROR", &err_msg)?;
            self.db.update_operation_status(&op_id, OperationStatus::Failed, Some(&Utc::now().to_rfc3339()), Some(&err_msg))?;
            return Err(e);
        }

        self.log_op(&op_id, "INFO", &format!("Successfully registered firmware file in workspace metadata store (ID: {})", file_id))?;

        // 6. Complete operation
        self.db.update_operation_status(&op_id, OperationStatus::Completed, Some(&Utc::now().to_rfc3339()), None)?;

        Ok(firmware_file)
    }

    /// Pre-flight source verification: validates that an imported source file has not been
    /// deleted, moved, or altered externally on disk before any operation consumes it.
    pub fn verify_source_file(&self, file_id: &str) -> AppResult<FirmwareFile> {
        let file = self.db.get_firmware_file(file_id)?;
        FileService::verify_file_integrity(&file.original_path, &file.sha256)?;
        Ok(file)
    }

    pub fn list_files(&self, workspace_id: &str) -> AppResult<Vec<FirmwareFile>> {
        self.db.list_files_by_workspace(workspace_id)
    }

    pub fn list_operations(&self, workspace_id: &str) -> AppResult<Vec<Operation>> {
        self.db.list_operations_by_workspace(workspace_id)
    }

    pub fn get_operation_logs(&self, operation_id: &str) -> AppResult<Vec<OperationLog>> {
        self.db.get_operation_logs(operation_id)
    }

    fn log_op(&self, op_id: &str, level: &str, message: &str) -> AppResult<()> {
        let log = OperationLog {
            id: 0,
            operation_id: op_id.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            message: message.to_string(),
        };
        self.db.append_operation_log(&log)
    }
}
