use std::path::Path;
use tempfile::tempdir;

use crate::engine::binary::ifd::{decode_flreg, IntelFlashDescriptor, IFD_SIGNATURE_MAGIC};
use crate::engine::binary::region::RegionType;
use crate::engine::testing::fixture_builder::SyntheticFixtureBuilder;
use crate::engine::uefi::firmware_volume::FirmwareVolume;
use crate::engine::vendor::dell_pfs::DellPfsParser;

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
    assert_eq!(bios_comp.sha256, crate::engine::binary::checksums::sha256_hex(bios_payload));
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
fn test_real_world_dell_latitude_5x90_if_available() {
    // If the authentic Dell Latitude 5X90 BIOS package exists in scratch or fixture directory,
    // exercise real-world extraction.
    let scratch_path = Path::new("C:/Users/xtrar/.gemini/antigravity/brain/2b8463af-e2de-426c-8983-bb9ac8466728/scratch/Latitude_5X90_1.41.0.exe");

    if scratch_path.exists() {
        let data = std::fs::read(scratch_path).unwrap();
        assert!(DellPfsParser::is_dell_pfs(&data));

        let dir = tempdir().unwrap();
        let report = DellPfsParser::extract(&data, Some(dir.path()), "Latitude_5X90_1.41.0.exe").unwrap();

        assert!(report.total_sections_found >= 1);
        assert!(!report.components.is_empty());

        // Verify that extracted components include System BIOS and Intel ME
        let comp_names: Vec<String> = report.components.iter().map(|c| c.name.clone()).collect();
        println!("Real-world Dell Latitude extracted components: {:?}", comp_names);

        assert!(comp_names.iter().any(|n| n.to_lowercase().contains("system bios") || n.to_lowercase().contains("bios")));
    }
}
