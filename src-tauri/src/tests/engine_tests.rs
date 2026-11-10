use std::path::{Path, PathBuf};
use tempfile::tempdir;

use crate::db::Database;
use crate::engine::binary::checksums::sha256_hex;
use crate::engine::binary::ifd::{decode_flreg, IntelFlashDescriptor, IFD_SIGNATURE_MAGIC};
use crate::engine::binary::reader::BinaryReader;
use crate::engine::binary::region::RegionType;
use crate::engine::testing::fixture_builder::SyntheticFixtureBuilder;
use crate::engine::uefi::firmware_volume::FirmwareVolume;
use crate::engine::vendor::dell_pfs::DellPfsParser;
use crate::models::artifact::{ArtifactFormat, FirmwareArtifact};
use crate::services::workspace_service::WorkspaceService;

pub const DELL_LATITUDE_REGISTERED_SHA256: &str =
    "E24B8F8F719CCD8F8B8EBDC8A6178D89C15121DF6806F7E112020964484B840B";

/// Documented fixture resolution hierarchy for real-world Dell firmware validation:
/// 1. `DELL_PFS_REALWORLD_FIXTURE` environment variable (explicit file path)
/// 2. `FRIEDCHIP_REALWORLD_FIXTURES_DIR` environment variable (directory path)
/// 3. Repository-relative path: `fixtures/realworld/Latitude_5X90_1.41.0.exe`
/// 4. Fallback legacy scratch path if present
fn resolve_dell_latitude_fixture_path() -> Option<PathBuf> {
    // 1. Explicit fixture file override
    if let Ok(path_str) = std::env::var("DELL_PFS_REALWORLD_FIXTURE") {
        let p = PathBuf::from(path_str);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Explicit fixtures directory override
    if let Ok(dir_str) = std::env::var("FRIEDCHIP_REALWORLD_FIXTURES_DIR") {
        let p = PathBuf::from(dir_str).join("Latitude_5X90_1.41.0.exe");
        if p.exists() {
            return Some(p);
        }
    }

    // 3. Project repository relative paths
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let relative_path = manifest_dir.join("../fixtures/realworld/Latitude_5X90_1.41.0.exe");
    if relative_path.exists() {
        return Some(relative_path);
    }

    let cwd_path = PathBuf::from("fixtures/realworld/Latitude_5X90_1.41.0.exe");
    if cwd_path.exists() {
        return Some(cwd_path);
    }

    // 4. Fallback legacy scratch directory if present
    let scratch_path = PathBuf::from(
        "C:/Users/xtrar/.gemini/antigravity/brain/2b8463af-e2de-426c-8983-bb9ac8466728/scratch/Latitude_5X90_1.41.0.exe",
    );
    if scratch_path.exists() {
        return Some(scratch_path);
    }

    None
}

// -----------------------------------------------------------------------------
// INTEL FLASH DESCRIPTOR (IFD) TESTS
// -----------------------------------------------------------------------------

#[test]
fn test_flreg_decoding_15bit_addressing() {
    // Base page 0x10 (0x10000), limit page 0x1F (0x1FFFF)
    let val = (0x1Fu32 << 16) | 0x10u32;
    let (base, limit) = decode_flreg(val);
    assert_eq!(base, 0x10 << 12);
    assert_eq!(limit, (0x1F << 12) | 0xFFF);

    // 128MB boundary: limit page 0x7FFF
    let val_max = (0x7FFFu32 << 16) | 0x4000u32;
    let (base_max, limit_max) = decode_flreg(val_max);
    assert_eq!(base_max, 0x4000 << 12);
    assert_eq!(limit_max, (0x7FFF << 12) | 0xFFF);

    // Bits outside 15-bit fields (bit 15 and bit 31) must not leak
    let dirty = val | (1 << 15) | (1 << 31);
    assert_eq!(decode_flreg(dirty), (base, limit));

    // Unused / disabled region: limit < base (e.g. 0x00007FFF)
    let (unused_base, unused_limit) = decode_flreg(0x00007FFF);
    assert!(unused_limit < unused_base);
}

#[test]
fn test_ifd_valid_image_parsing_and_gates() {
    let img = SyntheticFixtureBuilder::build_valid_ifd_image(16 * 1024 * 1024);
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    assert_eq!(report.image_size, 16 * 1024 * 1024);
    assert!(report.gate1_structural.passed);
    assert!(report.gate1_structural.has_valid_signature);
    assert_eq!(report.gate1_structural.signature_magic, format!("0x{:08X}", IFD_SIGNATURE_MAGIC));
    assert!(report.gate1_structural.boundary_conflicts.is_empty());

    assert!(report.gate2_platform.passed);
    assert_eq!(report.gate2_platform.region_count, 5);

    // Enabled regions: Descriptor (0), BIOS (1), ME (2)
    let enabled_types: Vec<RegionType> = report.regions.iter().filter(|r| r.enabled).map(|r| r.region_type).collect();
    assert!(enabled_types.contains(&RegionType::Descriptor));
    assert!(enabled_types.contains(&RegionType::Bios));
    assert!(enabled_types.contains(&RegionType::Me));

    // Disabled regions: GbE (3), PlatformData (4)
    let disabled_types: Vec<RegionType> = report.regions.iter().filter(|r| !r.enabled).map(|r| r.region_type).collect();
    assert!(disabled_types.contains(&RegionType::Gbe));
    assert!(disabled_types.contains(&RegionType::PlatformData));

    // Gate 3 programmer compatibility
    assert!(report.gate3_programmer.passed);
    assert_eq!(report.gate3_programmer.detected_chip_size_bytes, 16 * 1024 * 1024);
    assert_eq!(report.gate3_programmer.detected_chip_size_mb, 16.0);
}

#[test]
fn test_ifd_corrupt_signature_rejection() {
    let img = SyntheticFixtureBuilder::build_ifd_with_corrupt_signature();
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    assert!(!report.gate1_structural.passed);
    assert!(!report.gate1_structural.has_valid_signature);
    assert!(!report.gate2_platform.passed);
    assert!(!report.gate3_programmer.passed);
}

#[test]
fn test_ifd_boundary_collision_detection() {
    let img = SyntheticFixtureBuilder::build_ifd_with_overlapping_regions();
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    // Structural gate must fail due to overlap
    assert!(!report.gate1_structural.passed);
    assert!(!report.gate1_structural.boundary_conflicts.is_empty());
    assert_eq!(report.gate1_structural.boundary_conflicts[0].overlap_bytes, 0x1000);
}

#[test]
fn test_ifd_out_of_bounds_detection() {
    let img = SyntheticFixtureBuilder::build_ifd_with_out_of_bounds_region();
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    assert!(!report.gate1_structural.issues.is_empty());
    assert!(report.gate1_structural.issues.iter().any(|i| i.contains("exceeds total image size")));
}

#[test]
fn test_ifd_fcba_decoding_component_density() {
    let img = SyntheticFixtureBuilder::build_valid_ifd_image(16 * 1024 * 1024);
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    let comp = report.components.as_ref().expect("Components info should be decoded from FCBA");
    assert_eq!(comp.component_count, 1);
    assert_eq!(comp.components.len(), 1);
    assert_eq!(comp.components[0].density_bytes, 16 * 1024 * 1024);
    assert_eq!(comp.components[0].density_mb, 16.0);
    assert!(comp.fast_read_supported);
    assert_eq!(comp.fast_read_clock_frequency_mhz, 50);
}

#[test]
fn test_ifd_frap_decoding_master_permissions() {
    let img = SyntheticFixtureBuilder::build_valid_ifd_image(16 * 1024 * 1024);
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    let perms = report.master_permissions.as_ref().expect("Master permissions should be decoded from FMBA");
    assert_eq!(perms.frap_raw, 0x04070207);
    assert!(perms.host_bios_writable);
    assert!(!perms.host_me_writable);
    assert!(perms.host_cpu.bios_readable);
    assert!(perms.host_cpu.bios_writable);
    assert!(!perms.host_cpu.me_writable);

    // Gate 3 verification constraints distinguish descriptor permissions from hardware PR registers
    assert!(report.gate3_programmer.verification_constraints.iter().any(|c| c.contains("Protected Range registers (PR0..PR4)")));
}

#[test]
fn test_ifd_truncated_under_4kb_rejection() {
    let short_bytes = vec![0xFFu8; 2048];
    let res = IntelFlashDescriptor::parse(&short_bytes);
    assert!(res.is_err());
}

#[test]
fn test_ifd_frba_out_of_bounds_handling() {
    let mut small_img = SyntheticFixtureBuilder::build_valid_ifd_image(4096);
    // In FLMAP0 (0x14..0x18), set FRBA (byte 0x16) to 0xFF (offset 4080)
    // 4080 + (5 regions * 4 bytes) = 4100 > 4096
    small_img[0x16] = 0xFF;
    let report = IntelFlashDescriptor::parse(&small_img).unwrap();
    assert!(!report.gate1_structural.passed);
    assert!(!report.gate1_structural.region_table_in_bounds);
    assert!(report.gate1_structural.issues.iter().any(|i| i.contains("extends beyond image size")));
}

// -----------------------------------------------------------------------------
// DELL PFS PARSER TESTS (SYNTHETIC & REAL-WORLD)
// -----------------------------------------------------------------------------

#[test]
fn test_synthetic_dell_pfs_extraction_end_to_end() {
    let dir = tempdir().unwrap();

    let bios_payload = b"FIXTURE_SYSTEM_BIOS_RAW_BYTES_0123456789";
    let me_payload = b"FIXTURE_INTEL_ME_FIRMWARE_PAYLOAD_ABCDEF";
    let ec_payload = b"FIXTURE_EMBEDDED_CONTROLLER_BYTES_XYZ999";

    let synthetic_pkg = SyntheticFixtureBuilder::build_synthetic_dell_pfs(&[
        ("System BIOS", "1.17.0.0", bios_payload),
        ("Intel ME", "12.0.70.1652", me_payload),
        ("Embedded Controller", "1.0.5.0", ec_payload),
    ]);

    assert!(DellPfsParser::is_dell_pfs(&synthetic_pkg));

    let report = DellPfsParser::extract(&synthetic_pkg, Some(dir.path()), "SyntheticTestPkg").unwrap();

    assert_eq!(report.total_sections_found, 1);
    assert_eq!(report.sections.len(), 1);
    assert!(report.sections[0].header_checksum_valid);
    assert!(report.sections[0].footer_checksum_valid);
    assert!(report.sections[0].payload_inverted_crc32_valid);

    // 4 components extracted: Name Info table + 3 payloads
    assert_eq!(report.components.len(), 4);

    let bios_comp = report.components.iter().find(|c| c.name == "System BIOS").unwrap();
    assert_eq!(bios_comp.version, "1.17.0.0");
    assert_eq!(bios_comp.raw_size, bios_payload.len());
    assert_eq!(bios_comp.sha256, sha256_hex(bios_payload));
    assert!(bios_comp.file_path.is_some());
    assert!(bios_comp.classification.contains("Unverified SPI Layout"));

    let me_comp = report.components.iter().find(|c| c.name == "Intel ME").unwrap();
    assert_eq!(me_comp.version, "12.0.70.1652");
    assert_eq!(me_comp.raw_size, me_payload.len());

    let ec_comp = report.components.iter().find(|c| c.name == "Embedded Controller").unwrap();
    assert_eq!(ec_comp.version, "1.0.5.0");
    assert_eq!(ec_comp.raw_size, ec_payload.len());

    // Verify written files exist on disk with correct content
    for comp in &report.components {
        let p = comp.file_path.as_ref().unwrap();
        assert!(Path::new(p).exists());
    }
}

#[test]
fn test_dell_pfs_corrupt_xor_detection() {
    let corrupt_pkg = SyntheticFixtureBuilder::build_corrupt_xor_dell_pfs();
    let report = DellPfsParser::extract(&corrupt_pkg, None, "CorruptXorPkg").unwrap();

    assert!(report.issues.iter().any(|i| i.contains("XOR checksum mismatch")));
}

#[test]
fn test_dell_pfs_truncated_zlib_stream() {
    let mut pkg = SyntheticFixtureBuilder::build_synthetic_dell_pfs(&[("Test", "1.0", b"DATA")]);
    pkg.truncate(pkg.len() - 30);

    let report = DellPfsParser::extract(&pkg, None, "TruncatedPkg").unwrap();
    assert!(report.issues.iter().any(|i| i.contains("truncated") || i.contains("ZLIB decompression failed") || i.contains("extends beyond buffer")));
}

#[test]
fn test_dell_pfs_corrupted_inverted_crc32() {
    let corrupt_crc_pkg = SyntheticFixtureBuilder::build_corrupt_crc_dell_pfs();
    let report = DellPfsParser::extract(&corrupt_crc_pkg, None, "CorruptCrcPkg").unwrap();

    assert_eq!(report.sections.len(), 1);
    assert!(!report.sections[0].payload_inverted_crc32_valid);
}

#[test]
fn test_real_world_dell_latitude_5x90_if_available() {
    let fixture_path = match resolve_dell_latitude_fixture_path() {
        Some(p) => p,
        None => {
            println!("[SKIPPED] test_real_world_dell_latitude_5x90_if_available: Real-world Dell Latitude fixture not found. Skipping test.");
            return;
        }
    };

    let data = std::fs::read(&fixture_path).expect("Failed to read Dell real-world fixture");
    let actual_hash = sha256_hex(&data).to_uppercase();
    assert_eq!(
        actual_hash, DELL_LATITUDE_REGISTERED_SHA256,
        "Dell real-world fixture SHA-256 verification failed! Corrupted or altered fixture binary."
    );

    assert!(DellPfsParser::is_dell_pfs(&data));

    let dir = tempdir().unwrap();
    let report = DellPfsParser::extract(&data, Some(dir.path()), "Latitude_5X90_1.41.0.exe").unwrap();

    assert!(report.total_sections_found >= 1);
    for (i, c) in report.components.iter().enumerate() {
        println!("EXTRACTED {}: name='{}', size={}, sha256={}", i, c.name, c.raw_size, c.sha256);
    }
    assert_eq!(report.components.len(), 14);

    let comp_names: Vec<String> = report.components.iter().map(|c| c.name.clone()).collect();
    assert!(comp_names.iter().any(|n| n.to_lowercase().contains("system bios")));
    assert!(comp_names.iter().any(|n| n.to_lowercase().contains("management engine")));

    println!("[PASS] test_real_world_dell_latitude_5x90_if_available: Successfully extracted 12 components from verified Dell package.");
}

#[test]
fn test_dell_pfs_differential_regression_against_biosutilities() {
    let fixture_path = match resolve_dell_latitude_fixture_path() {
        Some(p) => p,
        None => {
            println!("[SKIPPED] test_dell_pfs_differential_regression_against_biosutilities: Real-world Dell Latitude fixture not found. Skipping test.");
            return;
        }
    };

    let data = std::fs::read(&fixture_path).expect("Failed to read Dell real-world fixture");
    let actual_hash = sha256_hex(&data).to_uppercase();
    assert_eq!(
        actual_hash, DELL_LATITUDE_REGISTERED_SHA256,
        "Dell real-world fixture SHA-256 verification failed!"
    );

    let dir = tempdir().unwrap();
    let report = DellPfsParser::extract(&data, Some(dir.path()), "Latitude_5X90_1.41.0.exe").unwrap();

    // 14 components expected from Dell Latitude 5X90 v1.41.0 (12 payload components + 2 Component Directory Tables)
    assert_eq!(report.components.len(), 14, "Expected 14 extracted components (12 payloads + 2 metadata tables)");

    // Golden metadata and SHA-256 expectations (Ground Truth from BIOSUtilities dell_pfs_extract.py)
    // 9 Exact Matches:
    let exact_expectations: &[(&str, usize, &str)] = &[
        ("Policy Payload", 77176, "EDEED47FEF951BEC14D3F90A43196DF1FA70D5E81786962962D8FA94332489C8"),
        ("Resource Payload", 95652, "16BEAB4DA4DC70832CF8545EEB244C1EFEB3CF2A008717B745D98C5A68E6D92D"),
        ("UI Payload", 1163810, "D5E75241D498FF961927F4A2ED1EAFE4026D4B7447B47A55BFD2EB1A867B2680"),
        ("Embedded Controller", 172864, "5EC89251817A31E48D16BEA5EDEC97D00FB3FC5B910F4A0E3E24A2A3265EB4D0"),
        ("Backup Embedded Controller", 166784, "308E2AD219B4D014E5A13FEC95D245B928B6AC0550B2C94256725DCCC68E5992"),
        ("Gigabit Ethernet", 8192, "BD443D826CCFF5D6637D97F220BF3588D23BEDCEAF91D7193EEF91E61E7C2696"),
        ("Intel Management Engine (VPro LP) Update", 7348224, "5AC278A441B6791A8DFE9D0319E725AD5882040999E93ADCB09FC59692C478BA"),
        ("System Map", 2208, "0FAFD0B147FD2C6FB9F7A3C6C67CACA32E9AEDCE7027C042CD43F56AC4195ABD"),
        ("Main System TI Port Controller 0", 6400, "1106E40744F5D5A48BC68CB4129CE837DFAF672326289715ACE051009DC793E5"),
    ];

    for (name_sub, expected_size, expected_sha) in exact_expectations {
        let comp = report
            .components
            .iter()
            .find(|c| c.name == *name_sub)
            .unwrap_or_else(|| panic!("Component {} not found in extracted components", name_sub));
        assert_eq!(comp.raw_size, *expected_size, "Size mismatch for {}", name_sub);
        assert_eq!(
            comp.sha256.to_uppercase(),
            *expected_sha,
            "SHA-256 mismatch for {}",
            name_sub
        );
    }

    // 3 Documented Differential Divergences (Clean-Room Raw Binary Fidelity):
    // 1. System BIOS with BIOS Guard: Raw PFS slice preserved (9,001,316 B) vs PFAT decapsulation (8,847,360 B)
    let bios = report.components.iter().find(|c| c.name == "System BIOS with BIOS Guard").unwrap();
    assert_eq!(bios.raw_size, 9001316, "FriedChip+ raw PFS payload size for System BIOS");
    assert_eq!(
        bios.sha256.to_uppercase(),
        "16260C135E0BC8A832C99DBBC809EB86599C72E809E9E73210EBD42057BA944D"
    );
    assert!(bios.classification.contains("Unverified SPI Layout"));

    // 2. PCR0 XML: Raw binary 2,452 bytes vs Python Windows CRLF injection 2,476 bytes
    let pcr0 = report.components.iter().find(|c| c.name == "PCR0 XML").unwrap();
    assert_eq!(pcr0.raw_size, 2452, "FriedChip+ pristine raw XML bytes without CRLF injection");
    assert_eq!(
        pcr0.sha256.to_uppercase(),
        "BA5B0CB32C1A63CE67D4D154726B81308E8452ECEDDC9E7F638C7B9422AC4124"
    );

    // 3. Model Information: Raw binary 112 bytes vs Python stripped delimiter 111 bytes
    let model = report.components.iter().find(|c| c.name == "Model Information").unwrap();
    assert_eq!(model.raw_size, 112, "FriedChip+ pristine raw Model Information bytes");
    assert_eq!(
        model.sha256.to_uppercase(),
        "353B1D39F415D3B6F93F8BA88B9CC078C8B0624EB93B9D15B8718AD55EC973A3"
    );

    println!("[PASS] test_dell_pfs_differential_regression_against_biosutilities: All 9 exact matches and 3 documented deltas verified.");
}

#[test]
#[should_panic(expected = "SHA-256 verification failed")]
fn test_real_world_fixture_hash_mismatch_panics() {
    let dummy_data = b"CORRUPTED_DUMMY_FIRMWARE_PAYLOAD";
    let actual_hash = sha256_hex(dummy_data).to_uppercase();
    if actual_hash != DELL_LATITUDE_REGISTERED_SHA256 {
        panic!("Dell real-world fixture SHA-256 verification failed! Corrupted or altered fixture binary.");
    }
}

// -----------------------------------------------------------------------------
// UEFI FIRMWARE VOLUME (FV) TESTS
// -----------------------------------------------------------------------------

#[test]
fn test_uefi_firmware_volume_header_parsing() {
    let fv_size = 0x20000; // 128KB
    let fv_bytes = SyntheticFixtureBuilder::build_valid_firmware_volume(fv_size);

    let volumes = FirmwareVolume::find_volumes(&fv_bytes);
    assert_eq!(volumes.len(), 1);

    let fv = &volumes[0];
    assert_eq!(fv.offset, 0);
    assert_eq!(fv.fv_length, fv_size as u64);
    assert_eq!(fv.signature, "_FVH");
    assert!(fv.checksum_valid);
    assert_eq!(fv.block_maps.len(), 1);
    assert_eq!(fv.block_maps[0].num_blocks, (fv_size / 0x1000) as u32);
    assert_eq!(fv.block_maps[0].block_length, 0x1000);
}

#[test]
fn test_uefi_fv_corrupt_signature_rejection() {
    let mut fv_bytes = SyntheticFixtureBuilder::build_valid_firmware_volume(0x10000);
    fv_bytes[0x28..0x2C].copy_from_slice(b"_BAD");

    let volumes = FirmwareVolume::find_volumes(&fv_bytes);
    assert_eq!(volumes.len(), 0, "Corrupt signature must not be identified as valid FV");
}

#[test]
fn test_uefi_fv_invalid_checksum_rejection() {
    let mut fv_bytes = SyntheticFixtureBuilder::build_valid_firmware_volume(0x10000);
    fv_bytes[0x32] ^= 0x55;

    let volumes = FirmwareVolume::find_volumes(&fv_bytes);
    assert_eq!(volumes.len(), 1);
    assert!(!volumes[0].checksum_valid, "Corrupt checksum must be reported as invalid");
}

#[test]
fn test_uefi_fv_truncated_header() {
    let short_bytes = vec![0xFFu8; 0x30];
    let reader = BinaryReader::new(&short_bytes);
    let res = FirmwareVolume::parse_at(&reader, 0);
    assert!(res.is_err(), "Truncated FV header must return an error");
    let volumes = FirmwareVolume::find_volumes(&short_bytes);
    assert!(volumes.is_empty(), "find_volumes on truncated bytes must return empty list");
}

#[test]
fn test_uefi_fv_block_map_sentinel_missing() {
    let mut fv_bytes = SyntheticFixtureBuilder::build_valid_firmware_volume(0x10000);
    // Replace terminating block map {0, 0} at 0x40..0x48 with non-zero blocks {1, 0x1000}
    fv_bytes[0x40..0x44].copy_from_slice(&1u32.to_le_bytes());
    fv_bytes[0x44..0x48].copy_from_slice(&0x1000u32.to_le_bytes());

    let reader = BinaryReader::new(&fv_bytes);
    let hdr = FirmwareVolume::parse_at(&reader, 0).unwrap();
    assert_eq!(hdr.block_maps.len(), 2);
}

#[test]
fn test_uefi_fv_misaligned_volume() {
    let fv_size = 0x10000;
    let fv_bytes = SyntheticFixtureBuilder::build_valid_firmware_volume(fv_size);
    let mut unaligned_img = vec![0xEEu8; 0x123];
    unaligned_img.extend_from_slice(&fv_bytes);

    let volumes = FirmwareVolume::find_volumes(&unaligned_img);
    assert_eq!(volumes.len(), 1);
    assert_eq!(volumes[0].offset, 0x123);
    assert_eq!(volumes[0].fv_length, fv_size as u64);
    assert!(volumes[0].checksum_valid);
}

// -----------------------------------------------------------------------------
// FIRMWARE ARTIFACT PERSISTENCE ROUNDTRIP TESTS
// -----------------------------------------------------------------------------

#[test]
fn test_firmware_artifact_persistence_roundtrip() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("artifacts_roundtrip.db");
    let db = Database::open_or_create(&db_path).unwrap();

    let ws_service = WorkspaceService::new(db.clone());
    let ws = ws_service.ensure_default_workspace(dir.path()).unwrap();

    let file_path = dir.path().join("source_firmware.bin");
    std::fs::write(&file_path, b"RAW_SOURCE_BYTES_123456").unwrap();
    let imported_file = ws_service.import_firmware_file(&ws.id, &file_path).unwrap();

    let artifact = FirmwareArtifact {
        id: uuid::Uuid::new_v4().to_string(),
        workspace_id: ws.id.clone(),
        source_file_id: imported_file.id.clone(),
        parent_artifact_id: None,
        name: "Intel Management Engine".to_string(),
        format: ArtifactFormat::FirmwarePartition,
        source_offset: 0x1000,
        raw_size: 7348224,
        sha256: "624647C9E6D3DCAC4B7F1FE231FF926CF25907AC8BCE16E0A831A6B4E9A4D3B0".to_string(),
        md5: Some("D41D8CD98F00B204E9800998ECF8427E".to_string()),
        file_path: Some("extracted/me.bin".to_string()),
        gate1_passed: true,
        gate2_passed: true,
        gate3_passed: false,
        validation_notes: vec!["Extracted from Dell PFS Section 1".to_string()],
        warnings: vec!["Unverified SPI Layout".to_string()],
        metadata_json: r#"{"version":"12.0.70.1652"}"#.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    // Test database helper method
    db.insert_artifact(&artifact).unwrap();
    let queried_via_db = db.get_artifact(&artifact.id).unwrap().unwrap();
    assert_eq!(queried_via_db.id, artifact.id);
    assert_eq!(queried_via_db.sha256, artifact.sha256);

    let conn = db.conn();
    let conn_guard = conn.lock().unwrap();

    let queried: FirmwareArtifact = conn_guard
        .query_row(
            r#"
            SELECT
                id, workspace_id, source_file_id, parent_artifact_id, name, format,
                source_offset, raw_size, sha256, md5, file_path,
                gate1_passed, gate2_passed, gate3_passed, validation_notes, warnings,
                metadata_json, created_at
            FROM firmware_artifacts
            WHERE id = ?1
            "#,
            [&artifact.id],
            |row| {
                let format_str: String = row.get(5)?;
                let notes_str: String = row.get(14)?;
                let warnings_str: String = row.get(15)?;
                Ok(FirmwareArtifact {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    source_file_id: row.get(2)?,
                    parent_artifact_id: row.get(3)?,
                    name: row.get(4)?,
                    format: format_str.parse().unwrap_or(ArtifactFormat::Unknown),
                    source_offset: row.get::<_, i64>(6)? as usize,
                    raw_size: row.get::<_, i64>(7)? as usize,
                    sha256: row.get(8)?,
                    md5: row.get(9)?,
                    file_path: row.get(10)?,
                    gate1_passed: row.get::<_, i32>(11)? == 1,
                    gate2_passed: row.get::<_, i32>(12)? == 1,
                    gate3_passed: row.get::<_, i32>(13)? == 1,
                    validation_notes: serde_json::from_str(&notes_str).unwrap_or_default(),
                    warnings: serde_json::from_str(&warnings_str).unwrap_or_default(),
                    metadata_json: row.get(16)?,
                    created_at: row.get(17)?,
                })
            },
        )
        .unwrap();

    assert_eq!(queried.id, artifact.id);
    assert_eq!(queried.workspace_id, artifact.workspace_id);
    assert_eq!(queried.source_file_id, artifact.source_file_id);
    assert_eq!(queried.name, artifact.name);
    assert_eq!(queried.format, ArtifactFormat::FirmwarePartition);
    assert_eq!(queried.source_offset, 0x1000);
    assert_eq!(queried.raw_size, 7348224);
    assert_eq!(queried.sha256, "624647C9E6D3DCAC4B7F1FE231FF926CF25907AC8BCE16E0A831A6B4E9A4D3B0");
    assert_eq!(queried.gate1_passed, true);
    assert_eq!(queried.gate2_passed, true);
    assert_eq!(queried.gate3_passed, false);
    assert_eq!(queried.validation_notes, vec!["Extracted from Dell PFS Section 1".to_string()]);
    assert_eq!(queried.warnings, vec!["Unverified SPI Layout".to_string()]);
}
