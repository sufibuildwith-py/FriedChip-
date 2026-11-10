pub mod migrations;

use std::path::Path;
use std::sync::{Arc, Mutex};
use rusqlite::{params, Connection};
use crate::error::{AppError, AppResult};
use crate::models::{AppSetting, FirmwareFile, FirmwareFormat, FormatConfidence, Operation, OperationLog, OperationStatus, Workspace};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new_in_memory() -> AppResult<Self> {
        let mut conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrations::run_migrations(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_or_create<P: AsRef<Path>>(path: P) -> AppResult<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrations::run_migrations(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // Workspaces
    pub fn create_workspace(&self, ws: &Workspace) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO workspaces (id, name, root_path, created_at, updated_at, is_default)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                ws.id,
                ws.name,
                ws.root_path,
                ws.created_at,
                ws.updated_at,
                if ws.is_default { 1 } else { 0 }
            ],
        )?;
        Ok(())
    }

    pub fn get_workspace(&self, id: &str) -> AppResult<Workspace> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, root_path, created_at, updated_at, is_default FROM workspaces WHERE id = ?1",
        )?;
        let ws = stmt.query_row([id], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                is_default: row.get::<_, i32>(5)? == 1,
            })
        })?;
        Ok(ws)
    }

    pub fn get_default_workspace(&self) -> AppResult<Option<Workspace>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, root_path, created_at, updated_at, is_default FROM workspaces WHERE is_default = 1 LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                is_default: true,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_workspaces(&self) -> AppResult<Vec<Workspace>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, root_path, created_at, updated_at, is_default FROM workspaces ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                root_path: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                is_default: row.get::<_, i32>(5)? == 1,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // Firmware Files
    pub fn insert_firmware_file(&self, file: &FirmwareFile) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO firmware_files (
                id, workspace_id, original_path, filename, extension,
                size_bytes, sha256, md5, detected_format, format_confidence,
                import_timestamp, is_read_only
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                file.id,
                file.workspace_id,
                file.original_path,
                file.filename,
                file.extension,
                file.size_bytes as i64,
                file.sha256,
                file.md5,
                file.detected_format.to_string(),
                file.format_confidence.to_string(),
                file.import_timestamp,
                if file.is_read_only { 1 } else { 0 }
            ],
        ).map_err(|e| match e {
            rusqlite::Error::SqliteFailure(err, _) if err.extended_code == 2067 => {
                AppError::Duplicate(format!("File with hash {} already exists in workspace", file.sha256))
            }
            _ => AppError::Database(e),
        })?;
        Ok(())
    }

    pub fn get_firmware_file(&self, id: &str) -> AppResult<FirmwareFile> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, workspace_id, original_path, filename, extension,
                   size_bytes, sha256, md5, detected_format, format_confidence,
                   import_timestamp, is_read_only
            FROM firmware_files
            WHERE id = ?1
            "#,
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            let format_str: String = row.get(8)?;
            let conf_str: String = row.get(9)?;
            Ok(FirmwareFile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                original_path: row.get(2)?,
                filename: row.get(3)?,
                extension: row.get(4)?,
                size_bytes: row.get::<_, i64>(5)? as u64,
                sha256: row.get(6)?,
                md5: row.get(7)?,
                detected_format: FirmwareFormat::from(format_str.as_str()),
                format_confidence: FormatConfidence::from(conf_str.as_str()),
                import_timestamp: row.get(10)?,
                is_read_only: row.get::<_, i32>(11)? == 1,
            })
        } else {
            Err(AppError::NotFound(format!("Firmware file record not found for ID: {}", id)))
        }
    }

    pub fn find_file_by_sha256(&self, workspace_id: &str, sha256: &str) -> AppResult<Option<FirmwareFile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, workspace_id, original_path, filename, extension,
                   size_bytes, sha256, md5, detected_format, format_confidence,
                   import_timestamp, is_read_only
            FROM firmware_files
            WHERE workspace_id = ?1 AND sha256 = ?2
            "#,
        )?;
        let mut rows = stmt.query(params![workspace_id, sha256])?;
        if let Some(row) = rows.next()? {
            let format_str: String = row.get(8)?;
            let conf_str: String = row.get(9)?;
            Ok(Some(FirmwareFile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                original_path: row.get(2)?,
                filename: row.get(3)?,
                extension: row.get(4)?,
                size_bytes: row.get::<_, i64>(5)? as u64,
                sha256: row.get(6)?,
                md5: row.get(7)?,
                detected_format: FirmwareFormat::from(format_str.as_str()),
                format_confidence: FormatConfidence::from(conf_str.as_str()),
                import_timestamp: row.get(10)?,
                is_read_only: row.get::<_, i32>(11)? == 1,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_files_by_workspace(&self, workspace_id: &str) -> AppResult<Vec<FirmwareFile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, workspace_id, original_path, filename, extension,
                   size_bytes, sha256, md5, detected_format, format_confidence,
                   import_timestamp, is_read_only
            FROM firmware_files
            WHERE workspace_id = ?1
            ORDER BY import_timestamp DESC
            "#,
        )?;
        let rows = stmt.query_map([workspace_id], |row| {
            let format_str: String = row.get(8)?;
            let conf_str: String = row.get(9)?;
            Ok(FirmwareFile {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                original_path: row.get(2)?,
                filename: row.get(3)?,
                extension: row.get(4)?,
                size_bytes: row.get::<_, i64>(5)? as u64,
                sha256: row.get(6)?,
                md5: row.get(7)?,
                detected_format: FirmwareFormat::from(format_str.as_str()),
                format_confidence: FormatConfidence::from(conf_str.as_str()),
                import_timestamp: row.get(10)?,
                is_read_only: row.get::<_, i32>(11)? == 1,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // Operations
    pub fn create_operation(&self, op: &Operation) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO operations (
                id, workspace_id, operation_type, input_file_id, input_sha256,
                status, created_at, completed_at, error_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                op.id,
                op.workspace_id,
                op.operation_type,
                op.input_file_id,
                op.input_sha256,
                op.status.to_string(),
                op.created_at,
                op.completed_at,
                op.error_message
            ],
        )?;
        Ok(())
    }

    pub fn update_operation_status(
        &self,
        id: &str,
        status: OperationStatus,
        completed_at: Option<&str>,
        error_message: Option<&str>,
    ) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            UPDATE operations
            SET status = ?1, completed_at = ?2, error_message = ?3
            WHERE id = ?4
            "#,
            params![status.to_string(), completed_at, error_message, id],
        )?;
        Ok(())
    }

    pub fn list_operations_by_workspace(&self, workspace_id: &str) -> AppResult<Vec<Operation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, workspace_id, operation_type, input_file_id, input_sha256,
                   status, created_at, completed_at, error_message
            FROM operations
            WHERE workspace_id = ?1
            ORDER BY created_at DESC
            "#,
        )?;
        let rows = stmt.query_map([workspace_id], |row| {
            let status_str: String = row.get(5)?;
            Ok(Operation {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                operation_type: row.get(2)?,
                input_file_id: row.get(3)?,
                input_sha256: row.get(4)?,
                status: OperationStatus::from(status_str.as_str()),
                created_at: row.get(6)?,
                completed_at: row.get(7)?,
                error_message: row.get(8)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // Operation Logs
    pub fn append_operation_log(&self, log: &OperationLog) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO operation_logs (operation_id, timestamp, level, message)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![log.operation_id, log.timestamp, log.level, log.message],
        )?;
        Ok(())
    }

    pub fn get_operation_logs(&self, operation_id: &str) -> AppResult<Vec<OperationLog>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, operation_id, timestamp, level, message
            FROM operation_logs
            WHERE operation_id = ?1
            ORDER BY id ASC
            "#,
        )?;
        let rows = stmt.query_map([operation_id], |row| {
            Ok(OperationLog {
                id: row.get(0)?,
                operation_id: row.get(1)?,
                timestamp: row.get(2)?,
                level: row.get(3)?,
                message: row.get(4)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    // Settings
    pub fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO app_settings (key, value, updated_at)
            VALUES (?1, ?2, datetime('now'))
            ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')
            "#,
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> AppResult<Vec<AppSetting>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value, updated_at FROM app_settings")?;
        let rows = stmt.query_map([], |row| {
            Ok(AppSetting {
                key: row.get(0)?,
                value: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}
