pub mod engine_tests;

use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
use crate::db::Database;
use crate::error::AppError;
use crate::models::{FirmwareFormat, FormatConfidence, OperationStatus};
use crate::services::file_service::FileService;
use crate::services::workspace_service::WorkspaceService;

#[test]
fn test_sha256_correctness() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("empty_file.bin");
    File::create(&file_path).unwrap();

    let result = FileService::inspect_file(&file_path).unwrap();
    // SHA-256 of empty file is e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    assert_eq!(
        result.sha256,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(result.size_bytes, 0);

    // Test with known string: "LBE_WORKSTATION_CLEAN_ROOM"
    let file_path2 = dir.path().join("known_string.bin");
    let mut f2 = File::create(&file_path2).unwrap();
    f2.write_all(b"LBE_WORKSTATION_CLEAN_ROOM").unwrap();
    drop(f2);

    let result2 = FileService::inspect_file(&file_path2).unwrap();
    use sha2::{Digest, Sha256};
    let expected_hash = hex::encode(Sha256::digest(b"LBE_WORKSTATION_CLEAN_ROOM"));
    assert_eq!(result2.sha256, expected_hash);
}

#[test]
fn test_fresh_database_initialization() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("fresh.db");

    let db = Database::open_or_create(&db_path).unwrap();
    let service = WorkspaceService::new(db);
    let ws = service.ensure_default_workspace(dir.path()).unwrap();

    assert!(ws.is_default);
    assert_eq!(ws.name, "Default Workspace");

    let files = service.list_files(&ws.id).unwrap();
    assert_eq!(files.len(), 0);

    let ops = service.list_operations(&ws.id).unwrap();
    assert_eq!(ops.len(), 0);
}

#[test]
fn test_database_migrations_idempotency() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();

    // Run first time
    let res1 = crate::db::migrations::run_migrations(&mut conn);
    assert!(res1.is_ok());

    // Run second time (should be completely safe and idempotent)
    let res2 = crate::db::migrations::run_migrations(&mut conn);
    assert!(res2.is_ok());

    let count: i32 = conn
        .query_row("SELECT COUNT(1) FROM schema_migrations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, crate::db::migrations::MIGRATIONS.len() as i32);
}

#[test]
fn test_file_import_and_metadata_persistence() {
    let dir = tempdir().unwrap();
    let db = Database::new_in_memory().unwrap();
    let service = WorkspaceService::new(db.clone());
    let ws = service.ensure_default_workspace(dir.path()).unwrap();

    // Create synthetic Intel Flash Descriptor image (16 bytes reserved, 5A A5 F0 0F at 0x10)
    let file_path = dir.path().join("test_spi_dump.bin");
    let mut data = vec![0xFFu8; 1024];
    data[0x10..0x14].copy_from_slice(&[0x5A, 0xA5, 0xF0, 0x0F]);
    fs::write(&file_path, &data).unwrap();

    let imported = service.import_firmware_file(&ws.id, &file_path).unwrap();

    assert_eq!(imported.detected_format, FirmwareFormat::IntelFlashDescriptor);
    assert_eq!(imported.format_confidence, FormatConfidence::Confirmed);
    assert_eq!(imported.size_bytes, 1024);

    let files = service.list_files(&ws.id).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].id, imported.id);
    assert_eq!(files[0].sha256, imported.sha256);

    let ops = service.list_operations(&ws.id).unwrap();
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0].operation_type, "FILE_IMPORT");
    assert_eq!(ops[0].status, OperationStatus::Completed);
}

#[test]
fn test_unknown_file_identification_not_inferred_from_extension() {
    let dir = tempdir().unwrap();
    let db = Database::new_in_memory().unwrap();
    let service = WorkspaceService::new(db);
    let ws = service.ensure_default_workspace(dir.path()).unwrap();

    // File named "bios_dump.bin" but without any recognized firmware signatures
    let file_path = dir.path().join("bios_dump.bin");
    let arbitrary_bytes = b"Just some arbitrary text and data that has no UEFI or IFD headers.";
    fs::write(&file_path, arbitrary_bytes).unwrap();

    let imported = service.import_firmware_file(&ws.id, &file_path).unwrap();

    // Must be classified as Unclassified, NOT assumed to be a valid BIOS merely from ".bin" extension
    assert_eq!(imported.detected_format, FirmwareFormat::Unclassified);
    assert_eq!(imported.format_confidence, FormatConfidence::Uncertain);
}

#[test]
fn test_duplicate_import_after_application_restart() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("restart_dup.db");
    let file_path = dir.path().join("source_dump.bin");
    fs::write(&file_path, b"unique_payload_for_dup_check_999").unwrap();

    let ws_id = {
        let db1 = Database::open_or_create(&db_path).unwrap();
        let s1 = WorkspaceService::new(db1);
        let ws = s1.ensure_default_workspace(dir.path()).unwrap();
        let _ = s1.import_firmware_file(&ws.id, &file_path).unwrap();
        ws.id
    };

    // Reopen DB simulating restart
    {
        let db2 = Database::open_or_create(&db_path).unwrap();
        let s2 = WorkspaceService::new(db2);
        let res = s2.import_firmware_file(&ws_id, &file_path);
        assert!(res.is_err());
        match res {
            Err(AppError::Duplicate(_)) => {}
            other => panic!("Expected AppError::Duplicate after restart, got: {:?}", other),
        }
    }
}

#[test]
fn test_persistent_workspace_recovery() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("persistent_recovery.db");
    let file_path = dir.path().join("firmware_sample.bin");
    fs::write(&file_path, b"test_firmware_recovery_data_98765").unwrap();

    let ws_id = {
        let db1 = Database::open_or_create(&db_path).unwrap();
        let s1 = WorkspaceService::new(db1);
        let ws1 = s1.ensure_default_workspace(dir.path()).unwrap();
        let _ = s1.import_firmware_file(&ws1.id, &file_path).unwrap();
        ws1.id
    };

    {
        let db2 = Database::open_or_create(&db_path).unwrap();
        let s2 = WorkspaceService::new(db2);

        let ws2 = s2.ensure_default_workspace(dir.path()).unwrap();
        assert_eq!(ws2.id, ws_id);

        let files = s2.list_files(&ws_id).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "firmware_sample.bin");

        let ops = s2.list_operations(&ws_id).unwrap();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation_type, "FILE_IMPORT");

        let logs = s2.get_operation_logs(&ops[0].id).unwrap();
        assert!(!logs.is_empty());
    }
}

#[test]
fn test_source_modification_detected_after_import() {
    let dir = tempdir().unwrap();
    let db = Database::new_in_memory().unwrap();
    let service = WorkspaceService::new(db);
    let ws = service.ensure_default_workspace(dir.path()).unwrap();

    let file_path = dir.path().join("tampered_dump.bin");
    fs::write(&file_path, b"pristine_initial_firmware_bytes").unwrap();

    let imported = service.import_firmware_file(&ws.id, &file_path).unwrap();

    // Verify initially passes pre-flight check
    assert!(service.verify_source_file(&imported.id).is_ok());

    // Tamper with the source file on disk
    fs::write(&file_path, b"pristine_altered_firmware_bytes").unwrap();

    // Must be detected as an integrity violation
    let verify_res = service.verify_source_file(&imported.id);
    assert!(verify_res.is_err());
    match verify_res {
        Err(AppError::Validation(msg)) => {
            assert!(msg.contains("Source integrity violation"));
        }
        other => panic!("Expected AppError::Validation, got: {:?}", other),
    }
}

#[test]
fn test_source_deletion_or_relocation_detected() {
    let dir = tempdir().unwrap();
    let db = Database::new_in_memory().unwrap();
    let service = WorkspaceService::new(db);
    let ws = service.ensure_default_workspace(dir.path()).unwrap();

    let file_path = dir.path().join("deleted_dump.bin");
    fs::write(&file_path, b"ephemeral_bytes_to_be_deleted").unwrap();

    let imported = service.import_firmware_file(&ws.id, &file_path).unwrap();

    // Delete source file from disk
    fs::remove_file(&file_path).unwrap();

    // Pre-flight check must detect missing source file
    let verify_res = service.verify_source_file(&imported.id);
    assert!(verify_res.is_err());
    match verify_res {
        Err(AppError::NotFound(msg)) => {
            assert!(msg.contains("missing or relocated"));
        }
        other => panic!("Expected AppError::NotFound, got: {:?}", other),
    }
}

#[test]
fn test_output_collision_and_atomic_overwrite_rejection() {
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("target_repaired.bin");
    let content1 = b"repaired_image_v1";

    // 1. Initial atomic write
    let written = FileService::write_atomic_output(&output_path, content1, false).unwrap();
    assert_eq!(written, output_path);
    assert_eq!(fs::read(&output_path).unwrap(), content1);

    // 2. Second write without allow_overwrite must be rejected
    let content2 = b"repaired_image_v2";
    let collision_err = FileService::write_atomic_output(&output_path, content2, false);
    assert!(collision_err.is_err());
    match collision_err {
        Err(AppError::Duplicate(msg)) => {
            assert!(msg.contains("already exists"));
        }
        other => panic!("Expected AppError::Duplicate, got: {:?}", other),
    }

    // Existing file content was not overwritten
    assert_eq!(fs::read(&output_path).unwrap(), content1);

    // 3. Write with explicit overwrite allowed must succeed atomically
    let overwrite_res = FileService::write_atomic_output(&output_path, content2, true);
    assert!(overwrite_res.is_ok());
    assert_eq!(fs::read(&output_path).unwrap(), content2);
}

#[test]
fn test_source_file_preservation() {
    let dir = tempdir().unwrap();
    let db = Database::new_in_memory().unwrap();
    let service = WorkspaceService::new(db);
    let ws = service.ensure_default_workspace(dir.path()).unwrap();

    let file_path = dir.path().join("precious_customer_dump.bin");
    let original_bytes = vec![0x42u8; 8192];
    fs::write(&file_path, &original_bytes).unwrap();

    let meta_before = fs::metadata(&file_path).unwrap();
    let modified_before = meta_before.modified().unwrap();

    let _ = service.import_firmware_file(&ws.id, &file_path).unwrap();

    let bytes_after = fs::read(&file_path).unwrap();
    let meta_after = fs::metadata(&file_path).unwrap();
    let modified_after = meta_after.modified().unwrap();

    assert_eq!(bytes_after, original_bytes);
    assert_eq!(meta_after.len(), 8192);
    assert_eq!(modified_before, modified_after);
}
