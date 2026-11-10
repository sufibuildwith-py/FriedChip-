use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use sha2::{Digest, Sha256};
use crate::error::{AppError, AppResult};
use crate::models::{FirmwareFormat, FormatConfidence};

pub struct FileInspectionResult {
    pub filename: String,
    pub extension: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub md5: String,
    pub detected_format: FirmwareFormat,
    pub format_confidence: FormatConfidence,
    pub is_read_only: bool,
}

pub struct FileService;

impl FileService {
    /// Safe inspect: opens the file strictly in read-only mode, streams hashes,
    /// and analyzes binary headers for reliable structural signatures.
    pub fn inspect_file<P: AsRef<Path>>(path: P) -> AppResult<FileInspectionResult> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(AppError::NotFound(format!("File does not exist: {}", path_ref.display())));
        }

        // Open strictly read-only
        let mut file = File::open(path_ref).map_err(|e| {
            AppError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to open file in read-only mode ({}): {}", path_ref.display(), e),
            ))
        })?;

        let metadata = file.metadata()?;
        let size_bytes = metadata.len();
        let is_read_only = metadata.permissions().readonly();

        let filename = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let extension = path_ref
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Calculate SHA-256 and MD5 using streaming buffer
        let mut sha256_hasher = Sha256::new();
        let mut buffer = [0u8; 65536]; // 64KB chunks
        let mut md5_context = md5::Context::new();

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            sha256_hasher.update(&buffer[..bytes_read]);
            md5_context.consume(&buffer[..bytes_read]);
        }

        let sha256 = hex::encode(sha256_hasher.finalize());
        let md5 = hex::encode(md5_context.compute().0);

        // Reset seek to beginning for format signature analysis
        file.seek(SeekFrom::Start(0))?;

        let (detected_format, format_confidence) = Self::detect_format(&mut file, size_bytes)?;

        Ok(FileInspectionResult {
            filename,
            extension,
            size_bytes,
            sha256,
            md5,
            detected_format,
            format_confidence,
            is_read_only,
        })
    }

    /// Detect firmware format strictly from structural magic signatures, never filename extension.
    fn detect_format(file: &mut File, size_bytes: u64) -> AppResult<(FirmwareFormat, FormatConfidence)> {
        // Read first 4096 bytes for header inspection
        let mut header = vec![0u8; 4096.min(size_bytes as usize)];
        let _ = file.read(&mut header)?;

        // 1. Intel Flash Descriptor (IFD) Check:
        // Offset 0x10 contains 4-byte signature: 5A A5 F0 0F (0x0FF0A55A in little-endian)
        if header.len() >= 0x14 && header[0x10..0x14] == [0x5A, 0xA5, 0xF0, 0x0F] {
            return Ok((FirmwareFormat::IntelFlashDescriptor, FormatConfidence::Confirmed));
        }

        // 2. UEFI Firmware Volume (FV) Check:
        // Look for '_FVH' magic at offset 0x28 within the first volume header
        if header.len() >= 0x30 && &header[0x28..0x2C] == b"_FVH" {
            return Ok((FirmwareFormat::UefiFirmwareVolume, FormatConfidence::Confirmed));
        }

        // Check if _FVH appears at standard alignment offsets (e.g. 0x28 or 0x48)
        if header.windows(4).any(|w| w == b"_FVH") {
            return Ok((FirmwareFormat::UefiFirmwareVolume, FormatConfidence::Confirmed));
        }

        // 3. Dell PFS Container Check:
        // Starts with magic bytes b"PFS.HDR." or b"PFS.DIR."
        if header.len() >= 8 && (&header[0..8] == b"PFS.HDR." || &header[0..8] == b"PFS.DIR.") {
            return Ok((FirmwareFormat::DellPfsContainer, FormatConfidence::Confirmed));
        }

        // 4. Asus Capsule Check:
        // Often begins with 2048-byte header or CAP magic
        if header.len() >= 2048 {
            // Check if second block (offset 2048) contains an IFD or UEFI volume
            let mut second_block = [0u8; 64];
            file.seek(SeekFrom::Start(2048))?;
            if let Ok(bytes) = file.read(&mut second_block) {
                if bytes >= 0x14 && second_block[0x10..0x14] == [0x5A, 0xA5, 0xF0, 0x0F] {
                    return Ok((FirmwareFormat::AsusCapsule, FormatConfidence::Confirmed));
                }
                if bytes >= 0x30 && second_block.windows(4).any(|w| w == b"_FVH") {
                    return Ok((FirmwareFormat::AsusCapsule, FormatConfidence::Confirmed));
                }
            }
        }

        // 5. HP Insyde Payload Check:
        if header.windows(5).any(|w| w == b"$HPD$" || w == b"$IBIOS") {
            return Ok((FirmwareFormat::HpInsydePayload, FormatConfidence::Confirmed));
        }

        // Default to UNCLASSIFIED with UNCERTAIN confidence.
        // Rule: Never claim verified format from extension alone.
        Ok((FirmwareFormat::Unclassified, FormatConfidence::Uncertain))
    }

    /// Pre-flight source integrity check: verifies the source file exists on disk and
    /// recalculates its streaming SHA-256 to ensure no external mutation has occurred.
    pub fn verify_file_integrity<P: AsRef<Path>>(path: P, expected_sha256: &str) -> AppResult<()> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(AppError::NotFound(format!(
                "Source firmware file missing or relocated: {}",
                path_ref.display()
            )));
        }

        let mut file = File::open(path_ref).map_err(|e| {
            AppError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to open source file for integrity verification: {}", e),
            ))
        })?;

        let mut sha256_hasher = Sha256::new();
        let mut buffer = [0u8; 65536];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            sha256_hasher.update(&buffer[..bytes_read]);
        }

        let actual_sha256 = hex::encode(sha256_hasher.finalize());
        if actual_sha256 != expected_sha256 {
            return Err(AppError::Validation(format!(
                "Source integrity violation for {}: on-disk SHA-256 ({}) does not match recorded SHA-256 ({})",
                path_ref.display(),
                actual_sha256,
                expected_sha256
            )));
        }

        Ok(())
    }

    /// Atomic output writer with explicit collision and overwrite protection.
    /// Writes to a temporary staging file in the target directory, flushes, syncs to disk,
    /// and performs an atomic rename to prevent partial/corrupted files.
    pub fn write_atomic_output<P: AsRef<Path>>(
        target_path: P,
        content: &[u8],
        allow_overwrite: bool,
    ) -> AppResult<std::path::PathBuf> {
        let target = target_path.as_ref();

        // 1. Explicit overwrite collision protection
        if target.exists() && !allow_overwrite {
            return Err(AppError::Duplicate(format!(
                "Output destination already exists: {}. Refusing to overwrite without explicit permission.",
                target.display()
            )));
        }

        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;

        // 2. Create unique temporary staging file in the same filesystem
        let temp_filename = format!(".tmp.{}.part", uuid::Uuid::new_v4());
        let temp_path = parent.join(&temp_filename);

        {
            use std::io::Write;
            let mut temp_file = File::create(&temp_path)?;
            temp_file.write_all(content)?;
            temp_file.sync_all()?;
        }

        // 3. Atomically rename temporary file to target path
        if let Err(e) = std::fs::rename(&temp_path, target) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(AppError::Io(std::io::Error::new(
                e.kind(),
                format!("Atomic rename failed from {} to {}: {}", temp_path.display(), target.display(), e),
            )));
        }

        Ok(target.to_path_buf())
    }
}
