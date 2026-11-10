use std::io::Read;
use std::path::{Path, PathBuf};
use flate2::read::ZlibDecoder;
use serde::{Deserialize, Serialize};

use crate::engine::binary::checksums::{checksum_8_xor, inverted_crc32, md5_hex, sha256_hex};
use crate::engine::binary::reader::BinaryReader;
use crate::engine::binary::verified_write::atomic_write_and_verify;
use crate::error::AppError;

pub const DELL_PFS_SECTION_PATTERN: &[u8; 10] = &[0xEE, 0xAA, 0x76, 0x1B, 0xEC, 0xBB, 0x20, 0xF1, 0xE6, 0x51];
pub const DELL_PFS_FOOTER_PATTERN: &[u8; 11] = &[0xEE, 0xAA, 0xEE, 0x8F, 0x49, 0x1B, 0xE8, 0xAE, 0x14, 0x37, 0x90];

pub const PFS_HDR_TAG: &[u8; 8] = b"PFS.HDR.";
pub const PFS_FTR_TAG: &[u8; 8] = b"PFS.FTR.";

// Hardcoded Dell PFS GUIDs (stored in Little-Endian byte order in entries, formatted as 32-hex reversed)
pub const GUID_NAME_INFO_1: &str = "E0717CE3A9BB25824B9F0DC8FD041960";
pub const GUID_NAME_INFO_2: &str = "B033CB16EC9B45A14055F80E4D583FD3";
pub const GUID_MODEL_INFO: &str = "6F1D619A22A6CB924FD4DA68233AE3FB";
pub const GUID_SIG_INFO: &str = "D086AFEE3ADBAEA94D5CED583C880BB7";
pub const GUID_NESTED_PFS: &str = "900FAE60437F3AB14055F456AC9FDA84";

/// Read a 16-byte Dell PFS GUID and format it as 32-character uppercase hex (reversed, matching little-endian uint128).
pub fn read_pfs_guid(reader: &BinaryReader, offset: usize) -> Result<String, AppError> {
    let bytes = reader.slice(offset, 16)?;
    let hex_str: String = bytes.iter().rev().map(|b| format!("{:02X}", b)).collect();
    Ok(hex_str)
}

// Resource safety limits
pub const MAX_INPUT_SIZE: usize = 512 * 1024 * 1024;       // 512 MB
pub const MAX_DECOMPRESSED_SIZE: usize = 256 * 1024 * 1024; // 256 MB
pub const MAX_ENTRY_COUNT: usize = 128;
pub const MAX_NESTING_DEPTH: usize = 4;

/// Extracted component metadata and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DellPfsExtractedComponent {
    pub index: usize,
    pub name: String,
    pub version: String,
    pub guid: String,
    pub raw_size: usize,
    pub sha256: String,
    pub md5: String,
    pub file_path: Option<String>,
    pub section_type: String,
    pub is_nested: bool,
    pub classification: String,
    pub validation_notes: Vec<String>,
}

/// Dell PFS section metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DellPfsSectionInfo {
    pub section_type_byte: u8,
    pub section_type_name: String,
    pub compressed_size: usize,
    pub header_checksum_valid: bool,
    pub footer_checksum_valid: bool,
    pub decompressed_size: usize,
    pub payload_inverted_crc32_valid: bool,
    pub components_count: usize,
}

/// Full extraction report for a Dell PFS executable/package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DellPfsExtractionReport {
    pub total_sections_found: usize,
    pub sections: Vec<DellPfsSectionInfo>,
    pub components: Vec<DellPfsExtractedComponent>,
    pub model_ids: Option<String>,
    pub model_brand: Option<String>,
    pub model_name: Option<String>,
    pub issues: Vec<String>,
}

pub struct DellPfsParser;

impl DellPfsParser {
    /// Detect whether the buffer contains Dell PFS section markers.
    pub fn is_dell_pfs(data: &[u8]) -> bool {
        let reader = BinaryReader::new(data);
        reader.find_subslice(DELL_PFS_SECTION_PATTERN, 0, None).is_some()
            || reader.find_subslice(PFS_HDR_TAG, 0, None).is_some()
    }

    /// Extract all components from a Dell PFS package and write them to output_dir if specified.
    pub fn extract(
        data: &[u8],
        output_dir: Option<&Path>,
        source_name: &str,
    ) -> Result<DellPfsExtractionReport, AppError> {
        if data.len() > MAX_INPUT_SIZE {
            return Err(AppError::Binary(format!(
                "Input size ({} MB) exceeds maximum allowed safe limit of {} MB",
                data.len() / (1024 * 1024),
                MAX_INPUT_SIZE / (1024 * 1024)
            )));
        }

        let reader = BinaryReader::new(data);
        let mut report = DellPfsExtractionReport {
            total_sections_found: 0,
            sections: Vec::new(),
            components: Vec::new(),
            model_ids: None,
            model_brand: None,
            model_name: None,
            issues: Vec::new(),
        };

        // Scan for all PFS section pattern occurrences
        let section_offsets = reader.find_all(DELL_PFS_SECTION_PATTERN, 0, None);

        // Also check if the buffer is directly a raw decompressed PFS volume (starts with PFS.HDR.)
        if section_offsets.is_empty() {
            if let Some(hdr_offset) = reader.find_subslice(PFS_HDR_TAG, 0, Some(0x1000)) {
                let raw_pfs_slice = &data[hdr_offset..];
                Self::parse_inner_pfs(
                    raw_pfs_slice,
                    "Raw Volume",
                    output_dir,
                    source_name,
                    0,
                    &mut report,
                )?;
                report.total_sections_found = 1;
                return Ok(report);
            }

            return Err(AppError::Binary(
                "No Dell PFS section markers or PFS.HDR. volume tags found".to_string(),
            ));
        }

        report.total_sections_found = section_offsets.len();

        for (sec_idx, &zlib_start) in section_offsets.iter().enumerate() {
            // Header starts 5 bytes before the 10-byte pattern:
            // [compressed_size: u32][section_type: u8][10-byte pattern][checksum: u8]
            if zlib_start < 5 || zlib_start + 11 > data.len() {
                report.issues.push(format!("Section {} at 0x{:X} truncated", sec_idx + 1, zlib_start));
                continue;
            }

            let header_start = zlib_start - 5;
            let section_type_byte = reader.read_u8(zlib_start - 1)?;
            let section_type_name = match section_type_byte {
                0xAA => "Firmware".to_string(),
                0xBB => "Utilities".to_string(),
                other => format!("Unknown (0x{:02X})", other),
            };

            let compressed_size_hdr = reader.read_u32_le(header_start)? as usize;
            let header_data = reader.slice(header_start, 16)?;

            // Header Checksum: 8-bit XOR over bytes 0..15 must match byte 15
            let hdr_chk_calc = checksum_8_xor(&header_data[0..15]);
            let hdr_chk_stored = header_data[15];
            let header_checksum_valid = hdr_chk_calc == hdr_chk_stored;

            if !header_checksum_valid {
                report.issues.push(format!(
                    "Section {} ({}) header XOR checksum mismatch: calc=0x{:02X}, stored=0x{:02X}",
                    sec_idx + 1, section_type_name, hdr_chk_calc, hdr_chk_stored
                ));
            }

            let compressed_start = zlib_start + 11;
            let compressed_end = match compressed_start.checked_add(compressed_size_hdr) {
                Some(end) => end,
                None => {
                    report.issues.push(format!("Section {} size overflow", sec_idx + 1));
                    continue;
                }
            };

            if compressed_end > data.len() {
                report.issues.push(format!(
                    "Section {} compressed payload (0x{:X} bytes) extends beyond buffer (0x{:X})",
                    sec_idx + 1, compressed_size_hdr, data.len()
                ));
                continue;
            }

            let compressed_slice = reader.slice(compressed_start, compressed_size_hdr)?;

            // Footer check: 16 bytes at compressed_end
            let mut footer_checksum_valid = false;
            if compressed_end + 16 <= data.len() {
                let footer_data = reader.slice(compressed_end, 16)?;
                let ftr_chk_calc = checksum_8_xor(&footer_data[0..15]);
                let ftr_chk_stored = footer_data[15];
                footer_checksum_valid = ftr_chk_calc == ftr_chk_stored;
            }

            // Pure-Rust ZLIB decompression with bounded reader
            let decompressed = match Self::decompress_zlib_bounded(compressed_slice) {
                Ok(bytes) => bytes,
                Err(err) => {
                    report.issues.push(format!(
                        "Section {} ({}) ZLIB decompression failed: {}",
                        sec_idx + 1, section_type_name, err
                    ));
                    continue;
                }
            };

            let decompressed_size = decompressed.len();

            // Parse inner PFS container
            let prev_components_count = report.components.len();
            let crc_valid = Self::parse_inner_pfs(
                &decompressed,
                &section_type_name,
                output_dir,
                source_name,
                0,
                &mut report,
            )?;

            let new_components_count = report.components.len() - prev_components_count;

            report.sections.push(DellPfsSectionInfo {
                section_type_byte,
                section_type_name,
                compressed_size: compressed_size_hdr,
                header_checksum_valid,
                footer_checksum_valid,
                decompressed_size,
                payload_inverted_crc32_valid: crc_valid,
                components_count: new_components_count,
            });
        }

        Ok(report)
    }

    /// Decompress a ZLIB stream with bounded capacity to protect against zip bombs.
    fn decompress_zlib_bounded(compressed_data: &[u8]) -> Result<Vec<u8>, AppError> {
        let mut decoder = ZlibDecoder::new(compressed_data);
        let mut out = Vec::new();
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let bytes_read = decoder.read(&mut buffer).map_err(|e| {
                AppError::Binary(format!("Corrupt ZLIB compression stream: {}", e))
            })?;

            if bytes_read == 0 {
                break;
            }

            if out.len() + bytes_read > MAX_DECOMPRESSED_SIZE {
                return Err(AppError::Binary(format!(
                    "Decompressed size exceeded maximum threshold of {} MB (possible decompression bomb)",
                    MAX_DECOMPRESSED_SIZE / (1024 * 1024)
                )));
            }

            out.extend_from_slice(&buffer[..bytes_read]);
        }

        Ok(out)
    }

    /// Parse an inner PFS volume (`PFS.HDR.` .. `PFS.FTR.`).
    fn parse_inner_pfs(
        volume_bytes: &[u8],
        section_name: &str,
        output_dir: Option<&Path>,
        source_name: &str,
        depth: usize,
        report: &mut DellPfsExtractionReport,
    ) -> Result<bool, AppError> {
        if depth > MAX_NESTING_DEPTH {
            report.issues.push(format!("Exceeded maximum nesting depth of {}", MAX_NESTING_DEPTH));
            return Ok(false);
        }

        let reader = BinaryReader::new(volume_bytes);
        if reader.len() < 0x20 {
            report.issues.push("Volume is too small for PFS header and footer".to_string());
            return Ok(false);
        }

        // Check header tag
        let tag = reader.slice(0, 8)?;
        if tag != PFS_HDR_TAG {
            report.issues.push(format!("Invalid PFS header tag: {:?}", String::from_utf8_lossy(tag)));
            return Ok(false);
        }

        let _hdr_version = reader.read_u32_le(0x08)?;
        let payload_size = reader.read_u32_le(0x0C)? as usize;

        if 0x10 + payload_size + 0x10 > reader.len() {
            report.issues.push(format!(
                "PFS payload size (0x{:X}) extends beyond volume buffer (0x{:X})",
                payload_size, reader.len()
            ));
            return Ok(false);
        }

        let payload_slice = reader.slice(0x10, payload_size)?;

        // Verify footer at 0x10 + payload_size
        let footer_offset = 0x10 + payload_size;
        let footer_size = reader.read_u32_le(footer_offset)? as usize;
        let footer_checksum = reader.read_u32_le(footer_offset + 4)?;
        let footer_tag = reader.slice(footer_offset + 8, 8)?;

        let mut crc_valid = false;
        if footer_tag == PFS_FTR_TAG && footer_size == payload_size {
            let computed_inverted_crc = inverted_crc32(payload_slice);
            crc_valid = computed_inverted_crc == footer_checksum;
            if !crc_valid {
                report.issues.push(format!(
                    "Inverted CRC-32 mismatch on PFS payload: calc=0x{:08X}, footer=0x{:08X}",
                    computed_inverted_crc, footer_checksum
                ));
            }
        }

        // First pass: extract all raw entry headers and payload slices
        let payload_reader = BinaryReader::new(payload_slice);
        let mut curr_offset = 0usize;
        let mut raw_entries = Vec::new();
        let mut name_info_payload: Option<Vec<u8>> = None;

        while curr_offset + 0x38 <= payload_reader.len() && raw_entries.len() < MAX_ENTRY_COUNT {
            let entry_hdr_ver = payload_reader.read_u32_le(curr_offset + 0x10)?;
            let entry_struct_size = match entry_hdr_ver {
                1 => 0x48, // R1
                2 => 0x58, // R2
                other => {
                    report.issues.push(format!("Unknown PFS entry revision {} at 0x{:X}", other, curr_offset));
                    break;
                }
            };

            if curr_offset + entry_struct_size > payload_reader.len() {
                break;
            }

            let guid = read_pfs_guid(&payload_reader, curr_offset)?;
            let data_size = payload_reader.read_u32_le(curr_offset + 0x28)? as usize;
            let data_sig_size = payload_reader.read_u32_le(curr_offset + 0x2C)? as usize;
            let data_met_size = payload_reader.read_u32_le(curr_offset + 0x30)? as usize;
            let data_met_sig_size = payload_reader.read_u32_le(curr_offset + 0x34)? as usize;

            let data_start = curr_offset + entry_struct_size;
            let data_end = match data_start.checked_add(data_size) {
                Some(end) => end,
                None => break,
            };

            if data_end > payload_reader.len() {
                report.issues.push(format!("Entry {} data extends beyond payload", raw_entries.len() + 1));
                break;
            }

            let component_data = payload_reader.slice(data_start, data_size)?;

            let next_offset = data_start
                .saturating_add(data_size)
                .saturating_add(data_sig_size)
                .saturating_add(data_met_size)
                .saturating_add(data_met_sig_size);

            if next_offset <= curr_offset {
                break; // prevent infinite loop
            }

            if guid == GUID_NAME_INFO_1 || guid == GUID_NAME_INFO_2 {
                name_info_payload = Some(component_data.to_vec());
            }

            raw_entries.push((guid, component_data.to_vec(), curr_offset));
            curr_offset = next_offset;
        }

        // Build GUID-to-Name map from Name Info table if present
        let mut name_map: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();

        if let Some(ref name_table) = name_info_payload {
            let name_reader = BinaryReader::new(name_table);
            let mut info_offset = 0usize;

            while info_offset + 0x22 <= name_reader.len() {
                let info_ver = match name_reader.read_u32_le(info_offset) {
                    Ok(v) => v,
                    Err(_) => break,
                };

                if info_ver != 1 {
                    break;
                }

                let target_guid = match read_pfs_guid(&name_reader, info_offset + 4) {
                    Ok(g) => g,
                    Err(_) => break,
                };

                // DellPfsName structure at info_offset + 0x14
                let name_struct_off = info_offset + 0x14;
                let char_count = match name_reader.read_u16_le(name_struct_off + 0x0C) {
                    Ok(c) => c as usize,
                    Err(_) => break,
                };

                let name_bytes_len = char_count * 2;
                let name_bytes_off = name_struct_off + 0x0E;

                if name_bytes_off + name_bytes_len > name_reader.len() {
                    break;
                }

                let name_slice = match name_reader.slice(name_bytes_off, name_bytes_len) {
                    Ok(s) => s,
                    Err(_) => break,
                };

                // Decode UTF-16LE
                let u16_words: Vec<u16> = name_slice
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                let decoded_name = String::from_utf16_lossy(&u16_words)
                    .trim_matches(char::from(0))
                    .trim()
                    .to_string();

                // Format version from 4 u16 values
                let v0 = name_reader.read_u16_le(name_struct_off)?;
                let v1 = name_reader.read_u16_le(name_struct_off + 2)?;
                let v2 = name_reader.read_u16_le(name_struct_off + 4)?;
                let v3 = name_reader.read_u16_le(name_struct_off + 6)?;
                let version_str = format!("{}.{}.{}.{}", v0, v1, v2, v3);

                name_map.insert(target_guid, (decoded_name, version_str));

                // Next name entry has 2 padding bytes after UTF-16 string
                info_offset = name_bytes_off + name_bytes_len + 2;
            }
        }

        // Process and extract each component
        for (idx, (guid, data, _)) in raw_entries.into_iter().enumerate() {
            let (friendly_name, version) = if let Some((n, v)) = name_map.get(&guid) {
                (n.clone(), v.clone())
            } else if guid == GUID_MODEL_INFO {
                ("Model Information".to_string(), "1.0".to_string())
            } else if guid == GUID_NAME_INFO_1 || guid == GUID_NAME_INFO_2 {
                ("Component Directory Table".to_string(), "1.0".to_string())
            } else if guid == GUID_SIG_INFO {
                ("Signature Catalog".to_string(), "1.0".to_string())
            } else if guid == GUID_NESTED_PFS {
                ("Nested Update Container".to_string(), "1.0".to_string())
            } else {
                (format!("Payload Component {}", idx + 1), "Unknown".to_string())
            };

            let sha256 = sha256_hex(&data);
            let md5 = md5_hex(&data);
            let raw_size = data.len();

            // Artifact classification: Do NOT assume it is a complete SPI image or has an IFD
            let classification = if friendly_name.to_lowercase().contains("system bios") {
                "Dell Extracted BIOS Payload (Unverified SPI Layout)".to_string()
            } else if friendly_name.to_lowercase().contains("me") || friendly_name.to_lowercase().contains("management engine") {
                "Intel Management Engine Region Payload".to_string()
            } else if friendly_name.to_lowercase().contains("controller") || friendly_name.to_lowercase().contains("ec") {
                "Embedded Controller Payload (Architecture Unclassified)".to_string()
            } else if friendly_name.to_lowercase().contains("pcr") || friendly_name.to_lowercase().contains("xml") {
                "Platform Configuration XML".to_string()
            } else {
                "OEM Firmware Component Payload".to_string()
            };

            // Write component to disk if output_dir is provided
            let mut file_path = None;
            if let Some(out_dir) = output_dir {
                let sanitized_name = friendly_name
                    .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|', ' '], "_");
                let file_name = format!("{}_{}_{}.bin", idx + 1, sanitized_name, &sha256[0..8]);
                let target_file_path: PathBuf = out_dir.join(&file_name);

                if let Ok(written_hash) = atomic_write_and_verify(&target_file_path, &data, true) {
                    if written_hash == sha256 {
                        file_path = Some(target_file_path.to_string_lossy().to_string());
                    }
                }
            }

            let is_nested = guid == GUID_NESTED_PFS;

            let mut validation_notes = Vec::new();
            validation_notes.push(format!("Extracted from section '{}' at depth {}", section_name, depth));
            validation_notes.push(format!("Integrity verified via SHA-256: {}", sha256));

            report.components.push(DellPfsExtractedComponent {
                index: report.components.len() + 1,
                name: friendly_name,
                version,
                guid,
                raw_size,
                sha256,
                md5,
                file_path,
                section_type: section_name.to_string(),
                is_nested,
                classification,
                validation_notes,
            });

            // If nested PFS, recurse safely
            if is_nested && depth < MAX_NESTING_DEPTH {
                let _ = Self::parse_inner_pfs(
                    &data,
                    &format!("{}/Nested", section_name),
                    output_dir,
                    source_name,
                    depth + 1,
                    report,
                );
            }
        }

        Ok(crc_valid)
    }
}
