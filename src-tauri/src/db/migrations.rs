use rusqlite::{Connection, Result};

pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "001_initial_schema",
        sql: r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                root_path TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS firmware_files (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
                original_path TEXT NOT NULL,
                filename TEXT NOT NULL,
                extension TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                sha256 TEXT NOT NULL,
                md5 TEXT NOT NULL,
                detected_format TEXT NOT NULL,
                format_confidence TEXT NOT NULL,
                import_timestamp TEXT NOT NULL,
                is_read_only INTEGER NOT NULL DEFAULT 1,
                UNIQUE(workspace_id, sha256)
            );

            CREATE TABLE IF NOT EXISTS operations (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
                operation_type TEXT NOT NULL,
                input_file_id TEXT REFERENCES firmware_files(id) ON DELETE SET NULL,
                input_sha256 TEXT,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                error_message TEXT
            );

            CREATE TABLE IF NOT EXISTS operation_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                operation_id TEXT NOT NULL REFERENCES operations(id) ON DELETE CASCADE,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                message TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_firmware_files_workspace ON firmware_files(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_firmware_files_sha256 ON firmware_files(sha256);
            CREATE INDEX IF NOT EXISTS idx_operations_workspace ON operations(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_operation_logs_op ON operation_logs(operation_id);
        "#,
    },
    Migration {
        version: 2,
        name: "002_firmware_artifacts",
        sql: r#"
            CREATE TABLE IF NOT EXISTS firmware_artifacts (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
                source_file_id TEXT NOT NULL REFERENCES firmware_files(id) ON DELETE CASCADE,
                parent_artifact_id TEXT REFERENCES firmware_artifacts(id) ON DELETE SET NULL,
                name TEXT NOT NULL,
                format TEXT NOT NULL,
                source_offset INTEGER NOT NULL,
                raw_size INTEGER NOT NULL,
                sha256 TEXT NOT NULL,
                md5 TEXT,
                file_path TEXT,
                gate1_passed INTEGER NOT NULL DEFAULT 0,
                gate2_passed INTEGER NOT NULL DEFAULT 0,
                gate3_passed INTEGER NOT NULL DEFAULT 0,
                validation_notes TEXT NOT NULL DEFAULT '[]',
                warnings TEXT NOT NULL DEFAULT '[]',
                metadata_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_firmware_artifacts_workspace ON firmware_artifacts(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_firmware_artifacts_source_file ON firmware_artifacts(source_file_id);
            CREATE INDEX IF NOT EXISTS idx_firmware_artifacts_parent ON firmware_artifacts(parent_artifact_id);
            CREATE INDEX IF NOT EXISTS idx_firmware_artifacts_sha256 ON firmware_artifacts(sha256);
        "#,
    },
];

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );
        "#,
        [],
    )?;

    for migration in MIGRATIONS {
        let count: i32 = conn.query_row(
            "SELECT COUNT(1) FROM schema_migrations WHERE version = ?1",
            [migration.version],
            |row| row.get(0),
        )?;

        if count == 0 {
            let tx = conn.transaction()?;
            tx.execute_batch(migration.sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, datetime('now'))",
                (migration.version, migration.name),
            )?;
            tx.commit()?;
        }
    }

    Ok(())
}
