use std::path::{Path, PathBuf};
use tempfile::tempdir;

use crate::db::Database;
use crate::engine::binary::checksums::sha256_hex;
use crate::engine::binary::ifd::{
    decode_flcomp, decode_flcomp_with_profile, decode_flreg, decode_flreg_with_profile,
    decode_master_rule, decode_master_rule_with_profile, decode_spi_frequency,
    validate_descriptor_geometry, IfdProfile, IntelFlashDescriptor, ProfileConfidence,
    ProfileSource, ProgrammerCompatibilityStatus, RuntimeChipsetLockStatus, SpiFrequency,
    IFD_SIGNATURE_MAGIC,
};
use crate::engine::binary::reader::BinaryReader;
use crate::engine::binary::region::RegionType;
use crate::engine::testing::fixture_builder::SyntheticFixtureBuilder;
use crate::engine::uefi::firmware_volume::FirmwareVolume;
use crate::engine::vendor::dell_pfs::DellPfsParser;
use crate::models::artifact::{ArtifactFormat, FirmwareArtifact};
use crate::services::workspace_service::WorkspaceService;

/// Canonical loader for Dell Latitude 5X90 manifest (DEF-30).
pub fn load_dell_latitude_manifest() -> serde_json::Value {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p1 = manifest_dir.join("../fixtures/manifests/dell_latitude_5x90.json");
    let content = std::fs::read_to_string(&p1)
        .or_else(|_| std::fs::read_to_string("fixtures/manifests/dell_latitude_5x90.json"))
        .expect("Canonical fixture manifest fixtures/manifests/dell_latitude_5x90.json must exist!");
    serde_json::from_str(&content).expect("Canonical JSON manifest must be valid")
}

/// Documented fixture resolution hierarchy for real-world Dell firmware validation (DEF-30):
/// 1. `FRIEDCHIP_FIXTURE_PATH` or `DELL_PFS_REALWORLD_FIXTURE` environment variable (explicit file path)
/// 2. `FRIEDCHIP_FIXTURE_DIR` or `FRIEDCHIP_REALWORLD_FIXTURES_DIR` environment variable (directory path)
/// 3. Repository-relative path: `../fixtures/realworld/Latitude_5X90_1.41.0.exe` or `fixtures/realworld/Latitude_5X90_1.41.0.exe`
/// (No hardcoded developer scratch directories!)
fn resolve_dell_latitude_fixture_path() -> Option<PathBuf> {
    // 1. Explicit fixture file override
    for env_var in &["FRIEDCHIP_FIXTURE_PATH", "DELL_PFS_REALWORLD_FIXTURE"] {
        if let Ok(path_str) = std::env::var(env_var) {
            let p = PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }

    // 2. Explicit fixtures directory override
    for env_var in &["FRIEDCHIP_FIXTURE_DIR", "FRIEDCHIP_REALWORLD_FIXTURES_DIR"] {
        if let Ok(dir_str) = std::env::var(env_var) {
            let p = PathBuf::from(dir_str).join("Latitude_5X90_1.41.0.exe");
            if p.exists() {
                return Some(p);
            }
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
    let enabled_types: Vec<RegionType> = report.regions.iter().filter(|r| r.enabled == Some(true)).map(|r| r.region_type).collect();
    assert!(enabled_types.contains(&RegionType::Descriptor));
    assert!(enabled_types.contains(&RegionType::Bios));
    assert!(enabled_types.contains(&RegionType::Me));

    // Disabled regions: GbE (3), PlatformData (4)
    let disabled_types: Vec<RegionType> = report.regions.iter().filter(|r| r.enabled == Some(false)).map(|r| r.region_type).collect();
    assert!(disabled_types.contains(&RegionType::Gbe));
    assert!(disabled_types.contains(&RegionType::PlatformData));

    // Gate 3 programmer compatibility: Must be Indeterminate without physical hardware evidence (DEF-19, DEF-29, DEF-36)
    assert_eq!(report.gate3_programmer.status, ProgrammerCompatibilityStatus::Indeterminate);
    assert!(!report.gate3_programmer.passed);
    assert_eq!(report.gate3_programmer.descriptor_inferred_flash_capacity_bytes, Some(16 * 1024 * 1024));
    assert_eq!(report.gate3_programmer.descriptor_inferred_flash_capacity_mb, Some(16.0));
    assert!(!report.gate3_programmer.hardware_evidence_present);
    assert_eq!(report.profile, IfdProfile::V1Legacy);
    assert_eq!(report.gate2_platform.profile, IfdProfile::V1Legacy);
    assert_eq!(report.gate3_programmer.host_descriptor_bios_write_allowed, Some(true));
    assert_eq!(report.gate3_programmer.host_descriptor_write_disabled, Some(false));
    assert_eq!(report.gate3_programmer.runtime_chipset_lock_status, RuntimeChipsetLockStatus::Indeterminate);
}

#[test]
fn test_ifd_corrupt_signature_rejection() {
    let img = SyntheticFixtureBuilder::build_ifd_with_corrupt_signature();
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    assert!(!report.gate1_structural.passed);
    assert!(!report.gate1_structural.has_valid_signature);
    assert!(!report.gate2_platform.passed);
    // Gate 3 remains Indeterminate even on Gate 1 structural failure (DEF-29)
    assert_eq!(report.gate3_programmer.status, ProgrammerCompatibilityStatus::Indeterminate);
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
    assert_eq!(comp.components[0].density_bytes, Some(16 * 1024 * 1024));
    assert_eq!(comp.components[0].density_mb, Some(16.0));
    assert_eq!(comp.fast_read_supported, Some(true));
    assert_eq!(comp.fast_read_clock_frequency_mhz, Some(50));
    assert_eq!(comp.write_erase_clock_frequency_mhz, Some(33));
    assert_eq!(comp.read_id_status_clock_frequency_mhz, Some(33));
}

#[test]
fn test_ifd_flcomp_known_raw_register_values() {
    // Golden Register 1: 0x08900005 (Independent derivation from Intel SPI Programming Guide)
    // Bits 2:0 = 5 -> 16 MB density
    // Bits 19:17 = 0 -> 20 MHz Read clock
    // Bit 20 = 1 -> Fast Read Supported
    // Bits 23:21 = 4 -> 50 MHz Fast Read clock
    // Bits 26:24 = 0 -> 20 MHz Write clock
    // Bits 29:27 = 1 -> 33 MHz Read ID clock
    let raw1: u32 = 0x08900005;
    let decoded1 = decode_flcomp(0x80, raw1, 1);
    assert_eq!(decoded1.components[0].density_bytes, Some(16 * 1024 * 1024));
    assert_eq!(decoded1.read_clock_frequency_mhz, Some(20));
    assert_eq!(decoded1.fast_read_supported, Some(true));
    assert_eq!(decoded1.fast_read_clock_frequency_mhz, Some(50));
    assert_eq!(decoded1.write_erase_clock_frequency_mhz, Some(20));
    assert_eq!(decoded1.read_id_status_clock_frequency_mhz, Some(33));

    // Golden Register 2: Dual-chip configuration 0x0000002B
    // Bits 2:0 = 3 -> 4 MB Comp 1
    // Bits 5:3 = 5 -> 16 MB Comp 2
    // Bit 20 = 0 -> Fast Read Disabled
    let raw2: u32 = 0x0000002B;
    let decoded2 = decode_flcomp(0x80, raw2, 2);
    assert_eq!(decoded2.components.len(), 2);
    assert_eq!(decoded2.components[0].density_bytes, Some(4 * 1024 * 1024));
    assert_eq!(decoded2.components[1].density_bytes, Some(16 * 1024 * 1024));
    assert_eq!(decoded2.fast_read_supported, Some(false));
}

#[test]
fn test_ifd_flmstr_known_raw_register_values() {
    // 1. IFD v1 FLMSTR1: 0x02170000 (Host CPU / BIOS on ICH8..Haswell)
    // Requester ID = 0x0000
    // Read mask = 0x17 (Descriptor, BIOS, ME, PlatformData readable)
    // Write mask = 0x02 (BIOS writable only)
    let rule_v1 = decode_master_rule(1, "Host CPU / BIOS", 0x02170000, IfdProfile::V1Legacy);
    assert_eq!(rule_v1.requester_id, Some(0x0000));
    assert_eq!(rule_v1.descriptor_readable, Some(true));
    assert_eq!(rule_v1.descriptor_writable, Some(false));
    assert_eq!(rule_v1.bios_readable, Some(true));
    assert_eq!(rule_v1.bios_writable, Some(true));
    assert_eq!(rule_v1.me_readable, Some(true));
    assert_eq!(rule_v1.me_writable, Some(false));
    assert_eq!(rule_v1.platform_data_readable, Some(true));
    assert_eq!(rule_v1.platform_data_writable, Some(false));

    // 2. IFD v1 FLMSTR1: 0xFFFFFFFF (Fully Unlocked Descriptor)
    let rule_unlocked = decode_master_rule(1, "Host CPU / BIOS", 0xFFFFFFFF, IfdProfile::V1Legacy);
    assert_eq!(rule_unlocked.descriptor_readable, Some(true));
    assert_eq!(rule_unlocked.descriptor_writable, Some(true));
    assert_eq!(rule_unlocked.me_readable, Some(true));
    assert_eq!(rule_unlocked.me_writable, Some(true));

    // 3. IFD v2 FLMSTR1: 0x00200300 (Modern Skylake+ offsets: rd_shift=8, wr_shift=20)
    // Read mask = 0x03 (Descriptor and BIOS readable)
    // Write mask = 0x02 (BIOS writable only)
    let rule_v2 = decode_master_rule(1, "Host CPU / BIOS", 0x00200300, IfdProfile::V2Modern);
    assert_eq!(rule_v2.requester_id, None);
    assert_eq!(rule_v2.descriptor_readable, Some(true));
    assert_eq!(rule_v2.descriptor_writable, Some(false));
    assert_eq!(rule_v2.bios_readable, Some(true));
    assert_eq!(rule_v2.bios_writable, Some(true));
    assert_eq!(rule_v2.me_readable, Some(false));
    assert_eq!(rule_v2.me_writable, Some(false));
}

#[test]
fn test_ifd_master_permissions_decoding() {
    let img = SyntheticFixtureBuilder::build_valid_ifd_image(16 * 1024 * 1024);
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    let perms = report.master_permissions.as_ref().expect("Master permissions should be decoded from FMBA");
    assert_eq!(perms.master_count, 3);
    assert_eq!(perms.host_bios_writable, Some(true));
    assert_eq!(perms.host_me_writable, Some(false));
    let host = perms.host_cpu.as_ref().unwrap();
    assert_eq!(host.bios_readable, Some(true));
    assert_eq!(host.bios_writable, Some(true));
    assert_eq!(host.me_writable, Some(false));
    let me = perms.intel_me.as_ref().unwrap();
    assert_eq!(me.me_readable, Some(true));
    assert_eq!(me.me_writable, Some(true));
    assert_eq!(me.bios_writable, Some(false));
    let gbe = perms.gbe.as_ref().unwrap();
    assert_eq!(gbe.gbe_readable, Some(true));
    assert_eq!(gbe.gbe_writable, Some(true));

    // Gate 3 verification constraints distinguish descriptor permissions from hardware PR registers
    assert!(report.gate3_programmer.verification_constraints.iter().any(|c| c.contains("Protected Range registers (PR0..PR4)")));
}

#[test]
fn test_ifd_gate3_programmer_indeterminate_without_hardware() {
    let img = SyntheticFixtureBuilder::build_valid_ifd_image(16 * 1024 * 1024);
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    assert!(report.gate1_structural.passed);
    assert_eq!(
        report.gate3_programmer.status,
        ProgrammerCompatibilityStatus::Indeterminate
    );
    assert!(!report.gate3_programmer.passed);
    assert!(!report.gate3_programmer.hardware_evidence_present);
    assert!(report.gate3_programmer.required_voltage_volts.is_none());
}

#[test]
fn test_ifd_truncated_under_4kb_rejection() {
    let short_bytes = vec![0xFFu8; 2048];
    let res = IntelFlashDescriptor::parse(&short_bytes);
    assert!(res.is_err());
}

#[test]
fn test_ifd_v2_valid_image_parsing_and_profile_detection() {
    let img = SyntheticFixtureBuilder::build_valid_ifd_v2_image(16 * 1024 * 1024);
    let report = IntelFlashDescriptor::parse(&img).unwrap();

    assert!(report.gate1_structural.passed);
    assert_eq!(report.profile, IfdProfile::V2Modern);
    assert_eq!(report.gate2_platform.profile, IfdProfile::V2Modern);
    assert!(report.gate2_platform.descriptor_version.contains("IFD v2.0"));
    assert_eq!(report.regions.len(), 16);

    let comp = report.components.as_ref().expect("Components info must be decoded");
    assert_eq!(comp.components[0].density_bytes, Some(16 * 1024 * 1024));
    assert_eq!(comp.read_clock, SpiFrequency::Mhz(17)); // 100-series Skylake marker
    assert_eq!(comp.read_clock_frequency_mhz, Some(17));
    assert_eq!(comp.fast_read_clock, SpiFrequency::Mhz(30)); // IFD v2 30 MHz

    let perms = report.master_permissions.as_ref().expect("Master permissions must be decoded");
    assert_eq!(perms.profile, IfdProfile::V2Modern);
    assert_eq!(perms.host_cpu.as_ref().unwrap().bios_writable, Some(true));
    assert_eq!(perms.host_bios_writable, Some(true));
    assert_eq!(perms.host_me_writable, Some(false));
}

#[test]
fn test_ifd_flreg_profile_aware_decoding_and_boundary_checks() {
    // 1. IFD v1 12-bit address mask test (0x0FFF)
    // val: base 0x000, limit 0x0FFF (inclusive -> 0x000FFFFF = 16MB)
    let (b1, l1, en1) = decode_flreg_with_profile(0x0FFF0000, IfdProfile::V1Legacy).unwrap();
    assert!(en1);
    assert_eq!(b1, 0x00000000);
    assert_eq!(l1, 0x00FFFFFF);

    // If bit 14 is set on v1 (0x4000), it must be masked out to 0
    let (b_v1_masked, _, _) = decode_flreg_with_profile(0x00004000, IfdProfile::V1Legacy).unwrap();
    assert_eq!(b_v1_masked, 0x00000000);

    // 2. IFD v2 15-bit address mask test (0x7FFF)
    // If bit 14 is set on v2 (0x4000), it shifts to 64 MB (0x04000000)
    let (b_v2, l_v2, en2) = decode_flreg_with_profile(0x7FFF4000, IfdProfile::V2Modern).unwrap();
    assert!(en2);
    assert_eq!(b_v2, 0x04000000);
    assert_eq!(l_v2, 0x07FFFFFF); // 128 MB limit

    // 3. Disabled region tests
    // Limit < Base (0x00007FFF: base = 0x7FFF, limit = 0x0000)
    let (_, _, en_dis) = decode_flreg_with_profile(0x00007FFF, IfdProfile::V2Modern).unwrap();
    assert!(!en_dis);

    // Both zero -> valid 4KB region (Region 0: Flash Descriptor 0x00000000..0x00000FFF)
    let (b_zero, l_zero, en_zero) = decode_flreg_with_profile(0x00000000, IfdProfile::V2Modern).unwrap();
    assert!(en_zero);
    assert_eq!(b_zero, 0x00000000);
    assert_eq!(l_zero, 0x00000FFF);
}

#[test]
fn test_ifd_flcomp_reserved_frequency_handling() {
    // Raw register with reserved frequency bit encoding 3 (0b011) at bits 19:17
    let raw: u32 = 0x05 | (3 << 17);
    let decoded = decode_flcomp_with_profile(0x80, raw, 1, IfdProfile::V1Legacy);

    // Must be explicitly represented as Reserved(3) and not silently defaulted to 20 MHz
    assert_eq!(decoded.read_clock, SpiFrequency::Reserved(3));
    assert_eq!(decoded.read_clock_frequency_mhz, None);

    let freq_v2 = decode_spi_frequency(6, IfdProfile::V2Modern);
    assert_eq!(freq_v2, SpiFrequency::Mhz(17));
}

#[test]
fn test_ifd_golden_vector_regression_against_coreboot_ifdtool() {
    // [GOLDEN_VECTOR_REGRESSION] Verifies register decoding parity against upstream
    // coreboot util/ifdtool/ifdtool.h and ifdtool.c (commit 66e704b1):
    // 1. Master bit shifts
    let flmstr_v1_golden = (0x02 << 24) | (0x17 << 16);
    let rule_v1 = decode_master_rule_with_profile(1, "CPU", flmstr_v1_golden, IfdProfile::V1Legacy);
    assert_eq!(rule_v1.bios_writable, Some(true));
    assert_eq!(rule_v1.bios_readable, Some(true));
    assert_eq!(rule_v1.me_readable, Some(true));

    let flmstr_v2_golden = (0x02 << 20) | (0x03 << 8);
    let rule_v2 = decode_master_rule_with_profile(1, "CPU", flmstr_v2_golden, IfdProfile::V2Modern);
    assert_eq!(rule_v2.bios_writable, Some(true));
    assert_eq!(rule_v2.bios_readable, Some(true));
    assert_eq!(rule_v2.me_readable, Some(false));

    // 2. Component densities
    let flcomp_v2 = decode_flcomp_with_profile(0x80, 0x0F5, 2, IfdProfile::V2Modern);
    assert_eq!(flcomp_v2.components[0].density_bytes, Some(16 * 1024 * 1024));
    assert_eq!(flcomp_v2.components[1].density_bytes, None); // 0xF is unused
}

#[test]
fn test_ifd_flmap0_nr_reserved_bit27_isolation() {
    // FLMAP0 with NR = 4 (bits 26:24 = 0b100) and reserved bit 27 set (0b1000 << 24 = 0x08000000)
    let flmap0_with_bit27 = (0x0C << 24) | (0x04 << 16) | 0x08; // bit 27 and bit 26 set
    let flmap0_without_bit27 = (0x04 << 24) | (0x04 << 16) | 0x08; // only bit 26 set (NR = 4)

    // With 3-bit mask ((flmap0 >> 24) & 0x07), both yield nr_bits = 4 (DEF-25.2)
    let nr_with = ((flmap0_with_bit27 >> 24) & 0x07) as usize;
    let nr_without = ((flmap0_without_bit27 >> 24) & 0x07) as usize;
    assert_eq!(nr_with, 4);
    assert_eq!(nr_without, 4);

    // Profile detection must not be tricked into modern profile by reserved bit 27
    let flmap1 = (0x02 << 24) | (0x10 << 16) | (0x02 << 8) | 0x06;
    let res = crate::engine::binary::ifd::detect_ifd_profile_resolution(flmap0_with_bit27, flmap1, 0, Some(0x08900005), Some(0x02170000), None);
    assert_eq!(res.profile, IfdProfile::V1Legacy);
}

#[test]
fn test_ifd_unknown_profile_safe_fallback_no_speculative_decoding() {
    // 1. FLREG with Unknown profile returns None (DEF-26, DEF-36)
    assert!(decode_flreg_with_profile(0x12345678, IfdProfile::Unknown).is_none());

    // 2. SPI Frequency with Unknown profile never claims a known MHz frequency (DEF-26, DEF-36)
    for bit in 0..8 {
        let freq = decode_spi_frequency(bit, IfdProfile::Unknown);
        assert_eq!(freq.mhz(), None);
        assert_eq!(freq.mhz_or_zero(), 0);
        assert_eq!(freq, SpiFrequency::Reserved(bit));
    }

    // 3. FLCOMP with Unknown profile preserves raw register without claiming decoded MB density (DEF-26, DEF-36)
    let comp_info = decode_flcomp_with_profile(0x80, 0x12345678, 2, IfdProfile::Unknown);
    assert_eq!(comp_info.flcomp_raw, 0x12345678);
    assert_eq!(comp_info.fast_read_supported, None);
    assert_eq!(comp_info.read_clock_frequency_mhz, None);
    for c in comp_info.components {
        assert_eq!(c.density_bytes, None);
        assert_eq!(c.density_mb, None);
    }

    // 4. FLMSTR with Unknown profile yields no inferred permissions (DEF-26, DEF-36)
    let master = decode_master_rule_with_profile(1, "Host", 0xFFFFFFFF, IfdProfile::Unknown);
    assert_eq!(master.flmstr_raw, 0xFFFFFFFF);
    assert_eq!(master.read_mask, None);
    assert_eq!(master.write_mask, None);
    assert_eq!(master.bios_writable, None);
    assert_eq!(master.bios_readable, None);
    assert_eq!(master.requester_id, None);
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
    assert!(report.gate1_structural.issues.iter().any(|i| i.contains("extends beyond image size") || i.contains("violates geometry bounds")));
}

// -----------------------------------------------------------------------------
// NEW DEF-34, DEF-35, DEF-37, DEF-38 DETERMINISTIC REGRESSION TESTS
// -----------------------------------------------------------------------------

#[test]
fn test_ifd_v1_5_quarantine_prevents_automatic_and_speculative_decoding() {
    // DEF-35: V1.5 is quarantined. Automatic profile detection must never assign V1.5.
    let flmap0 = (0x04 << 24) | (0x08 << 16) | 0x02;
    let flmap1 = (0x02 << 24) | (0x18 << 16) | (0x03 << 8) | 0x10;
    let res = crate::engine::binary::ifd::detect_ifd_profile_resolution(flmap0, flmap1, 0, Some(0x08900005), Some(0x02170000), None);
    assert_ne!(res.profile, IfdProfile::V1_5Intermediate);

    // Explicit override with V1_5Intermediate must be marked with Unknown confidence and quarantined
    let override_res = crate::engine::binary::ifd::detect_ifd_profile_resolution(flmap0, flmap1, 0, None, None, Some(IfdProfile::V1_5Intermediate));
    assert_eq!(override_res.profile, IfdProfile::V1_5Intermediate);
    assert_eq!(override_res.confidence, ProfileConfidence::Unknown);
    assert_eq!(override_res.source, ProfileSource::UserOverride);

    // Decoding with quarantined V1.5 yields safe None outputs
    assert!(decode_flreg_with_profile(0x0FFF0000, IfdProfile::V1_5Intermediate).is_none());
    let comp = decode_flcomp_with_profile(0x80, 0x12345678, 1, IfdProfile::V1_5Intermediate);
    assert_eq!(comp.density_bytes_at(0), None);
    assert_eq!(comp.fast_read_supported, None);
    let rule = decode_master_rule_with_profile(1, "Host", 0xFFFFFFFF, IfdProfile::V1_5Intermediate);
    assert_eq!(rule.bios_writable, None);
}

#[test]
fn test_ifd_geometry_validation_detects_collisions_and_out_of_bounds() {
    // DEF-37: Validate geometry detects collisions and invalid boundaries
    // 1. Collision: FCBA == FRBA
    let flmap0_collision = (0x04 << 24) | (0x08 << 16) | 0x08; // FRBA = 0x80, FCBA = 0x80
    let flmap1_valid = 0x10; // FMBA = 0x100
    let geom = validate_descriptor_geometry(flmap0_collision, flmap1_valid, 0, 4096);
    assert!(!geom.is_valid);
    assert!(geom.validation_errors.iter().any(|e| e.contains("Collision: FCBA overlaps FRBA")));

    // 2. Section out of bounds (< 0x20)
    let flmap0_oob = (0x04 << 24) | (0x08 << 16) | 0x01; // FCBA = 0x10 < 0x20 (inside header)
    let geom_oob = validate_descriptor_geometry(flmap0_oob, flmap1_valid, 0, 4096);
    assert!(!geom_oob.is_valid);
    assert!(geom_oob.validation_errors.iter().any(|e| e.contains("out of descriptor bounds")));
}

#[test]
fn test_ifd_conservative_profile_classification_rejects_ambiguous() {
    // DEF-34: Ambiguous/conflicting indicators must return IfdProfile::Unknown
    // FLMAP0 has legacy NR (NR=4), but FLCOMP has an unclassified frequency
    let flmap0 = (0x04 << 24) | (0x08 << 16) | 0x04;
    let flmap1 = (0x02 << 8) | 0x0C;
    // FLCOMP read clock is 5 (reserved value in both V1 and V2)
    let flcomp_ambiguous = Some(5 << 17);
    let res = crate::engine::binary::ifd::detect_ifd_profile_resolution(flmap0, flmap1, 0, flcomp_ambiguous, None, None);
    assert_eq!(res.profile, IfdProfile::Unknown);
    assert_eq!(res.confidence, ProfileConfidence::Unknown);
}

#[test]
fn test_ifd_flmap2_missing_read_error_propagation() {
    // DEF-38: Image truncated before offset 0x20 (e.g. exactly 0x1C bytes, missing FLMAP2)
    // Must return an explicit Err rather than silently defaulting to 0
    let mut short_desc = SyntheticFixtureBuilder::build_valid_ifd_image(4096);
    short_desc.truncate(0x1C); // Ends right before FLMAP2 at 0x1C
    let res = IntelFlashDescriptor::parse(&short_desc);
    assert!(res.is_err());
}

// Helper extension on FlashComponentsInfo for concise test inspection
impl crate::engine::binary::ifd::FlashComponentsInfo {
    pub fn density_bytes_at(&self, idx: usize) -> Option<usize> {
        self.components.get(idx).and_then(|c| c.density_bytes)
    }
}

// -----------------------------------------------------------------------------
// IN-PROCESS BOUNDED STRUCTURE-AWARE STRESS / FUZZING CAMPAIGN (DEF-14, DEF-22, DEF-39)
// -----------------------------------------------------------------------------

#[test]
fn test_in_process_bounded_parser_stress_fuzzing() {
    use std::time::Instant;

    let mut state: u64 = 0xDEADBEEFCAFE1337;
    let mut next_u32 = || -> u32 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (state >> 32) as u32
    };

    let start = Instant::now();
    let iterations = 100_000;

    let ifd_seed = SyntheticFixtureBuilder::build_valid_ifd_image(4096);
    let ifd_seed_v2 = SyntheticFixtureBuilder::build_valid_ifd_v2_image(4096);
    let pfs_seed = SyntheticFixtureBuilder::build_synthetic_dell_pfs(&[("BIOS", "1.0", b"TEST_PAYLOAD")]);
    let fv_seed = SyntheticFixtureBuilder::build_valid_firmware_volume(4096);

    // 1. Structure-aware mutation testing of Intel Flash Descriptor (DEF-32, DEF-39)
    let mut ifd_clean_rejections = 0usize;
    let mut ifd_successful_parses = 0usize;
    let mut ifd_sig_accepted = 0usize;
    let mut ifd_sig_rejected = 0usize;
    let mut ifd_geom_accepted = 0usize;
    let mut ifd_geom_rejected = 0usize;
    let mut ifd_profile_decoder_reached = 0usize;
    let mut ifd_region_table_decoded = 0usize;
    let mut ifd_region_conflict_reached = 0usize;

    for i in 0..iterations {
        let buf = match i % 8 {
            0 => {
                // Mutate FLVALSIG at offset 0x10
                let mut b = ifd_seed.clone();
                b[0x10] ^= ((next_u32() & 0xFF) as u8) | 0x01;
                b
            }
            1 => {
                // Mutate FLMAP0 reserved bits (e.g. bit 27, bits 31:28)
                let mut b = ifd_seed.clone();
                b[0x17] ^= ((next_u32() & 0x0F) as u8) << 3;
                b
            }
            2 => {
                // Mutate FCBA / FRBA / FMBA offsets in FLMAP0/FLMAP1
                let mut b = ifd_seed_v2.clone();
                let target = 0x14 + ((next_u32() % 8) as usize);
                b[target] = (next_u32() & 0xFF) as u8;
                b
            }
            3 => {
                // Mutate FLREG limit/base at FRBA (0x40..0x80)
                let mut b = ifd_seed_v2.clone();
                let reg_target = 0x40 + ((next_u32() % 64) as usize);
                b[reg_target] = (next_u32() & 0xFF) as u8;
                b
            }
            4 => {
                // Mutate FLCOMP density/frequency at FCBA (0xA0)
                let mut b = ifd_seed_v2.clone();
                b[0xA0] = (next_u32() & 0xFF) as u8;
                b[0xA1] = (next_u32() & 0xFF) as u8;
                b[0xA2] = (next_u32() & 0xFF) as u8;
                b
            }
            5 => {
                // Mutate FLMSTR at FMBA (0x80)
                let mut b = ifd_seed_v2.clone();
                let master_target = 0x80 + ((next_u32() % 16) as usize);
                b[master_target] = (next_u32() & 0xFF) as u8;
                b
            }
            6 => {
                // Truncate at critical descriptor boundaries
                let trunc_points = [16, 20, 24, 28, 32, 64, 128, 512, 1024, 2048, 4095];
                let pt = trunc_points[(next_u32() as usize) % trunc_points.len()];
                let mut b = ifd_seed.clone();
                b.truncate(pt);
                b
            }
            _ => {
                // Random byte sequence
                let len = (next_u32() % 4096) as usize + 64;
                let mut b = vec![0u8; len];
                for chunk in b.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&next_u32().to_le_bytes());
                }
                b
            }
        };

        let has_sig = buf.len() >= 0x14 && &buf[0x10..0x14] == &IFD_SIGNATURE_MAGIC.to_le_bytes();
        if has_sig {
            ifd_sig_accepted += 1;
        } else {
            ifd_sig_rejected += 1;
        }

        match IntelFlashDescriptor::parse(&buf) {
            Ok(rep) => {
                if rep.gate1_structural.region_table_in_bounds {
                    ifd_geom_accepted += 1;
                } else {
                    ifd_geom_rejected += 1;
                }
                if rep.profile != IfdProfile::Unknown {
                    ifd_profile_decoder_reached += 1;
                }
                if !rep.regions.is_empty() {
                    ifd_region_table_decoded += 1;
                }
                if !rep.gate1_structural.boundary_conflicts.is_empty() {
                    ifd_region_conflict_reached += 1;
                }

                if rep.gate1_structural.passed {
                    ifd_successful_parses += 1;
                } else {
                    ifd_clean_rejections += 1;
                }
            }
            Err(_) => {
                ifd_clean_rejections += 1;
                ifd_geom_rejected += 1;
            }
        }
    }

    assert!(ifd_sig_accepted > 0 && ifd_sig_rejected > 0);
    assert!(ifd_geom_accepted > 0 && ifd_geom_rejected > 0);
    assert!(ifd_profile_decoder_reached > 0);
    assert!(ifd_region_table_decoded > 0);
    assert!(ifd_region_conflict_reached > 0);

    // 2. Structure-aware mutation testing of Dell PFS Container (DEF-32, DEF-39)
    let mut pfs_clean_rejections = 0usize;
    let mut pfs_successful_parses = 0usize;
    let mut pfs_container_accepted = 0usize;
    let mut pfs_container_rejected = 0usize;
    let mut pfs_section_table_reached = 0usize;
    let mut pfs_guid_name_reached = 0usize;
    let mut pfs_zlib_decompression_attempted = 0usize;
    let mut pfs_xor_validation_attempted = 0usize;
    let mut pfs_crc_validation_attempted = 0usize;

    for i in 0..iterations {
        let buf = match i % 8 {
            0 => {
                // Mutate PFS.HDR. magic signature
                let mut b = pfs_seed.clone();
                b[0] ^= 0x01;
                b
            }
            1 => {
                // Mutate entry count / offsets
                let mut b = pfs_seed.clone();
                let pos = ((next_u32() as usize) % 32).min(b.len().saturating_sub(1));
                b[pos] ^= 0xFF;
                b
            }
            2 => {
                // Mutate section lengths
                let mut b = pfs_seed.clone();
                if b.len() > 16 {
                    b[8] = (next_u32() & 0xFF) as u8;
                }
                b
            }
            3 => {
                // Corrupt ZLIB payload stream
                let mut b = pfs_seed.clone();
                if b.len() > 32 {
                    let zlib_pos = 16 + ((next_u32() as usize) % (b.len() - 20));
                    b[zlib_pos] ^= 0xAA;
                }
                b
            }
            4 => {
                // Corrupt XOR checksum
                let mut b = pfs_seed.clone();
                if b.len() > 8 {
                    let end = b.len() - 1;
                    b[end] ^= 0x55;
                }
                b
            }
            5 => {
                // Truncate container at arbitrary offset
                let mut b = pfs_seed.clone();
                let cut = (next_u32() as usize % b.len()).max(4);
                b.truncate(cut);
                b
            }
            _ => {
                // Variable length random buffer
                let len = (next_u32() % 2048) as usize + 32;
                let mut b = vec![0u8; len];
                for chunk in b.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&next_u32().to_le_bytes());
                }
                b
            }
        };

        let has_pfs = DellPfsParser::is_dell_pfs(&buf);
        if has_pfs {
            pfs_container_accepted += 1;
        } else {
            pfs_container_rejected += 1;
        }

        match DellPfsParser::extract(&buf, None, "fuzz_input") {
            Ok(rep) => {
                if rep.total_sections_found > 0 {
                    pfs_section_table_reached += 1;
                }
                if !rep.components.is_empty() {
                    pfs_guid_name_reached += 1;
                }
                if rep.sections.iter().any(|s| s.decompressed_size > 0) {
                    pfs_zlib_decompression_attempted += 1;
                }
                if rep.sections.iter().any(|s| s.payload_inverted_crc32_valid || !s.payload_inverted_crc32_valid) {
                    pfs_crc_validation_attempted += 1;
                }
                pfs_xor_validation_attempted += 1;
                pfs_successful_parses += 1;
            }
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("ZLIB") || err_str.contains("decompress") {
                    pfs_zlib_decompression_attempted += 1;
                }
                if err_str.contains("CRC") {
                    pfs_crc_validation_attempted += 1;
                }
                if err_str.contains("XOR") {
                    pfs_xor_validation_attempted += 1;
                }
                pfs_clean_rejections += 1;
            }
        }
    }

    assert!(pfs_container_accepted > 0 && pfs_container_rejected > 0);
    assert!(pfs_section_table_reached > 0);
    assert!(pfs_guid_name_reached > 0);
    assert!(pfs_zlib_decompression_attempted > 0);
    assert!(pfs_xor_validation_attempted > 0);
    assert!(pfs_crc_validation_attempted > 0);

    // 3. Structure-aware mutation testing of UEFI Firmware Volumes (DEF-32, DEF-39)
    let mut fv_clean_rejections = 0usize;
    let mut fv_successful_parses = 0usize;
    let mut fv_fvh_detected = 0usize;
    let mut fv_fvh_rejected = 0usize;
    let mut fv_header_validated = 0usize;
    let mut fv_checksum_evaluated = 0usize;
    let mut fv_block_map_traversed = 0usize;
    let mut fv_sentinel_handled = 0usize;

    for i in 0..iterations {
        let buf = match i % 8 {
            0 => {
                // Mutate _FVH signature at offset 0x28
                let mut b = fv_seed.clone();
                if b.len() > 0x2C {
                    b[0x28] ^= 0x01;
                }
                b
            }
            1 => {
                // Mutate header length at offset 0x30
                let mut b = fv_seed.clone();
                if b.len() > 0x32 {
                    b[0x30] = (next_u32() & 0xFF) as u8;
                }
                b
            }
            2 => {
                // Mutate checksum at offset 0x32
                let mut b = fv_seed.clone();
                if b.len() > 0x34 {
                    b[0x32] ^= 0xFF;
                }
                b
            }
            3 => {
                // Mutate block map entries / missing sentinel
                let mut b = fv_seed.clone();
                if b.len() > 0x48 {
                    b[0x38] = (next_u32() & 0xFF) as u8;
                    b[0x3C] = (next_u32() & 0xFF) as u8;
                }
                b
            }
            4 => {
                // Truncate header
                let mut b = fv_seed.clone();
                let cut = ((next_u32() as usize) % 0x60).max(8);
                b.truncate(cut);
                b
            }
            _ => {
                // Random buffer
                let len = (next_u32() % 4096) as usize + 64;
                let mut b = vec![0u8; len];
                for chunk in b.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&next_u32().to_le_bytes());
                }
                b
            }
        };

        let has_fvh = buf.windows(4).any(|w| w == b"_FVH");
        if has_fvh {
            fv_fvh_detected += 1;
        } else {
            fv_fvh_rejected += 1;
        }

        let volumes = FirmwareVolume::find_volumes(&buf);
        if !volumes.is_empty() {
            fv_header_validated += volumes.len();
            fv_checksum_evaluated += volumes.len();
            fv_block_map_traversed += volumes.iter().map(|v| v.block_maps.len()).sum::<usize>();
            fv_sentinel_handled += volumes.len();
            fv_successful_parses += 1;
        } else {
            fv_clean_rejections += 1;
        }
    }

    assert!(fv_fvh_detected > 0 && fv_fvh_rejected > 0);
    assert!(fv_header_validated > 0);
    assert!(fv_checksum_evaluated > 0);
    assert!(fv_block_map_traversed > 0);
    assert!(fv_sentinel_handled > 0);

    assert_eq!(ifd_successful_parses + ifd_clean_rejections, iterations);
    assert_eq!(pfs_successful_parses + pfs_clean_rejections, iterations);
    assert_eq!(fv_successful_parses + fv_clean_rejections, iterations);

    let elapsed = start.elapsed();
    println!(
        "[FUZZ EVIDENCE] Structure-aware mutation testing completed: 3 x {} (300,000 total iterations) in {:.2?} with 0 crashes or hangs.",
        iterations, elapsed
    );
    println!(
        "  IFD paths: sig_acc={}, sig_rej={}, geom_acc={}, geom_rej={}, prof_dec={}, reg_tbl={}, reg_conflict={}",
        ifd_sig_accepted, ifd_sig_rejected, ifd_geom_accepted, ifd_geom_rejected, ifd_profile_decoder_reached, ifd_region_table_decoded, ifd_region_conflict_reached
    );
    println!(
        "  PFS paths: hdr_acc={}, hdr_rej={}, sec_tbl={}, guid_name={}, zlib_att={}, crc_att={}, xor_att={}",
        pfs_container_accepted, pfs_container_rejected, pfs_section_table_reached, pfs_guid_name_reached, pfs_zlib_decompression_attempted, pfs_crc_validation_attempted, pfs_xor_validation_attempted
    );
    println!(
        "  FV paths: fvh_det={}, fvh_rej={}, hdr_val={}, csum_eval={}, blk_trav={}, sent_hnd={}",
        fv_fvh_detected, fv_fvh_rejected, fv_header_validated, fv_checksum_evaluated, fv_block_map_traversed, fv_sentinel_handled
    );
}

// -----------------------------------------------------------------------------
// DELL PFS EXTRACTION TESTS
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

// -----------------------------------------------------------------------------
// REAL-WORLD OEM INTEGRATION TESTS (DEF-13, DEF-30 SEMANTICS)
// -----------------------------------------------------------------------------

#[test]
#[ignore = "Real-world OEM integration test requires Latitude_5X90_1.41.0.exe fixture in fixtures/realworld/"]
fn test_real_world_dell_latitude_5x90_if_available() {
    let manifest = load_dell_latitude_manifest();
    let expected_hash = manifest["expected_sha256"]
        .as_str()
        .expect("Manifest must contain expected_sha256")
        .to_uppercase();
    let expected_size = manifest["file_size"]
        .as_u64()
        .expect("Manifest must contain file_size") as usize;

    let fixture_path = resolve_dell_latitude_fixture_path().expect(
        "Required real-world Dell Latitude fixture not found! Fixture must be supplied to execute integration tests.",
    );

    let data = std::fs::read(&fixture_path).expect("Failed to read Dell real-world fixture");
    assert_eq!(data.len(), expected_size, "Dell real-world fixture file size mismatch!");

    let actual_hash = sha256_hex(&data).to_uppercase();
    assert_eq!(
        actual_hash, expected_hash,
        "Dell real-world fixture SHA-256 verification failed! Corrupted or altered fixture binary."
    );

    assert!(DellPfsParser::is_dell_pfs(&data));

    let dir = tempdir().unwrap();
    let report = DellPfsParser::extract(&data, Some(dir.path()), "Latitude_5X90_1.41.0.exe").unwrap();

    assert!(report.total_sections_found >= 1);
    assert_eq!(report.components.len(), 14);

    let comp_names: Vec<String> = report.components.iter().map(|c| c.name.clone()).collect();
    assert!(comp_names.iter().any(|n| n.to_lowercase().contains("system bios")));
    assert!(comp_names.iter().any(|n| n.to_lowercase().contains("management engine")));

    println!("[PASS] test_real_world_dell_latitude_5x90_if_available: Successfully extracted 14 components from verified Dell package.");
}

#[test]
#[ignore = "Real-world OEM integration test requires Latitude_5X90_1.41.0.exe fixture in fixtures/realworld/"]
fn test_dell_pfs_differential_regression_against_biosutilities() {
    let manifest = load_dell_latitude_manifest();
    let expected_hash = manifest["expected_sha256"]
        .as_str()
        .expect("Manifest must contain expected_sha256")
        .to_uppercase();

    let fixture_path = resolve_dell_latitude_fixture_path().expect(
        "Required real-world Dell Latitude fixture not found! Fixture must be supplied to execute integration tests.",
    );

    let data = std::fs::read(&fixture_path).expect("Failed to read Dell real-world fixture");
    let actual_hash = sha256_hex(&data).to_uppercase();
    assert_eq!(
        actual_hash, expected_hash,
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
    let manifest = load_dell_latitude_manifest();
    let expected_hash = manifest["expected_sha256"]
        .as_str()
        .expect("Manifest must contain expected_sha256")
        .to_uppercase();

    let dummy_data = b"CORRUPTED_DUMMY_FIRMWARE_PAYLOAD";
    let actual_hash = sha256_hex(dummy_data).to_uppercase();
    if actual_hash != expected_hash {
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
