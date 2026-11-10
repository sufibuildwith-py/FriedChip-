use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use crate::error::AppError;

/// Verifies that source and destination paths do not resolve to the same underlying file.
///
/// Prevents accidental overwrite of source firmware dumps.
pub fn ensure_distinct_paths(src: &Path, dst: &Path) -> Result<(), AppError> {
    if let (Ok(canon_src), Ok(canon_dst)) = (fs::canonicalize(src), fs::canonicalize(dst)) {
        if canon_src == canon_dst {
            return Err(AppError::Validation(format!(
                "Output path '{}' resolves to original input file '{}'. In-place overwriting is prohibited.",
                dst.display(),
                src.display()
            )));
        }
    }
    Ok(())
}

/// Atomically writes binary data to `dst_path` with durability and read-back verification.
///
/// Strategy:
/// 1. Verifies parent directory exists (creates if missing).
/// 2. If `allow_overwrite` is false and destination exists, returns `AppError::Duplicate`.
/// 3. Writes payload to `<dst_path>.part.<uuid>` in the same folder.
/// 4. Flushes OS buffers via `sync_all()`.
/// 5. Renames temporary file to `dst_path` (atomic on POSIX, atomic replace on Windows).
/// 6. Re-opens `dst_path`, computes SHA-256 of persisted bytes, and verifies against written data.
///
/// Returns the verified SHA-256 hex string.
pub fn atomic_write_and_verify(
    dst_path: &Path,
    data: &[u8],
    allow_overwrite: bool,
) -> Result<String, AppError> {
    if dst_path.exists() && !allow_overwrite {
        return Err(AppError::Duplicate(format!(
            "Target file already exists and overwrite protection is enabled: {}",
            dst_path.display()
        )));
    }

    let parent = dst_path
        .parent()
        .ok_or_else(|| AppError::Validation(format!("Invalid path has no parent: {}", dst_path.display())))?;
    fs::create_dir_all(parent)?;

    let temp_name = format!(
        "{}.part.{}",
        dst_path.file_name().and_then(|n| n.to_str()).unwrap_or("output"),
        uuid::Uuid::new_v4().simple()
    );
    let temp_path: PathBuf = parent.join(temp_name);

    // Write to temporary file with sync
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;

        file.write_all(data)?;
        file.sync_all()?;
    }

    // Atomic move to target path
    if let Err(e) = fs::rename(&temp_path, dst_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::Io(e));
    }

    // Read-back verification
    let mut verified_hasher = Sha256::new();
    let mut read_back_file = File::open(dst_path)?;
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let bytes_read = read_back_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        verified_hasher.update(&buffer[..bytes_read]);
    }

    let read_back_hash = hex::encode(verified_hasher.finalize());

    // Compare with in-memory hash
    let mut expected_hasher = Sha256::new();
    expected_hasher.update(data);
    let expected_hash = hex::encode(expected_hasher.finalize());

    if read_back_hash != expected_hash {
        let _ = fs::remove_file(dst_path);
        return Err(AppError::Internal(format!(
            "Read-back SHA-256 verification mismatch for '{}'! Expected: {}, got: {}",
            dst_path.display(),
            expected_hash,
            read_back_hash
        )));
    }

    Ok(read_back_hash)
}
