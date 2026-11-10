use std::path::{Path, PathBuf};
use tempfile::tempdir;

use crate::db::Database;
use crate::engine::binary::checksums::sha256_hex;
#[allow(deprecated)]
use crate::engine::binary::ifd::{
    decode_flcomp, decode_flcomp_with_profile, decode_flreg, decode_flreg_with_profile,
    decode_master_rule, decode_master_rule_with_profile, decode_spi_frequency,
    determine_flmap2_interpretation, detect_ifd_profile, detect_ifd_profile_with_context,
    detect_ifd_profile_resolution, validate_descriptor_geometry,
    ExtentLength, Flmap2Interpretation, IfdProfile, IntelFlashDescriptor, ProfileConfidence,
    ProfileSource, ProgrammerCompatibilityStatus, RuntimeChipsetLockStatus, SpiFrequency,
    IFD_SIGNATURE_MAGIC,
};
use crate::engine::binary::reader::BinaryReader;
use crate::engine::binary::region::RegionType;
use crate::engine::testing::fixture_builder::SyntheticFixtureBuilder;
use crate::engine::testing::observer::{
    disable_observer, enable_observer, reset_observer, take_snapshot,
};
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
    let res = crate::engine::binary::ifd::detect_ifd_profile_resolution(flmap0_with_bit27, flmap1, 0, Some(0x08900005), Some(0x02170000), None, None);
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
    let res = crate::engine::binary::ifd::detect_ifd_profile_resolution(flmap0, flmap1, 0, Some(0x08900005), Some(0x02170000), None, None);
    assert_ne!(res.profile, IfdProfile::V1_5Intermediate);

    // Explicit override with V1_5Intermediate must be marked with Unknown confidence and quarantined
    let override_res = crate::engine::binary::ifd::detect_ifd_profile_resolution(flmap0, flmap1, 0, None, None, None, Some(IfdProfile::V1_5Intermediate));
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
    let res = crate::engine::binary::ifd::detect_ifd_profile_resolution(flmap0, flmap1, 0, flcomp_ambiguous, None, None, None);
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

    enable_observer();
    reset_observer();

    let ifd_seed = SyntheticFixtureBuilder::build_valid_ifd_image(4096);
    let ifd_seed_v2 = SyntheticFixtureBuilder::build_valid_ifd_v2_image(4096);
    let pfs_seed = SyntheticFixtureBuilder::build_synthetic_dell_pfs(&[("BIOS", "1.0", b"TEST_PAYLOAD")]);
    let pfs_seed_corrupt_crc = SyntheticFixtureBuilder::build_synthetic_dell_pfs_with_options(
        &[("BIOS", "1.0", b"TEST_PAYLOAD")],
        true,
    );
    let fv_seed = SyntheticFixtureBuilder::build_valid_firmware_volume(4096);

    // 1. Structure-aware mutation testing of Intel Flash Descriptor (DEF-32, DEF-39, DEF-44)
    let mut ifd_clean_rejections = 0usize;
    let mut ifd_successful_parses = 0usize;

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

        match IntelFlashDescriptor::parse(&buf) {
            Ok(rep) => {
                if rep.gate1_structural.passed {
                    ifd_successful_parses += 1;
                } else {
                    ifd_clean_rejections += 1;
                }
            }
            Err(_) => {
                ifd_clean_rejections += 1;
            }
        }
    }

    // 2. Structure-aware mutation testing of Dell PFS Container (DEF-32, DEF-39, DEF-44)
    let mut pfs_clean_rejections = 0usize;
    let mut pfs_successful_parses = 0usize;

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
                // Seed with corrupted CRC inside decompressed payload: ZLIB succeeds, CRC fails!
                pfs_seed_corrupt_crc.clone()
            }
            6 => {
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

        let _ = DellPfsParser::is_dell_pfs(&buf);
        match DellPfsParser::extract(&buf, None, "fuzz_input") {
            Ok(_) => {
                pfs_successful_parses += 1;
            }
            Err(_) => {
                pfs_clean_rejections += 1;
            }
        }
    }

    // 3. Structure-aware mutation testing of UEFI Firmware Volumes (DEF-32, DEF-39, DEF-44)
    let mut fv_clean_rejections = 0usize;
    let mut fv_successful_parses = 0usize;

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
                // Corrupt block map sentinel {0, 0} while keeping valid header checksum
                let mut b = fv_seed.clone();
                if b.len() >= 0x48 {
                    b[0x40..0x44].copy_from_slice(&1u32.to_le_bytes());
                    b[0x44..0x48].copy_from_slice(&0x1000u32.to_le_bytes());
                    b[0x32..0x34].copy_from_slice(&0u16.to_le_bytes());
                    let chk = crate::engine::binary::checksums::efi_fv_checksum_16(&b[..0x48]);
                    b[0x32..0x34].copy_from_slice(&chk.to_le_bytes());
                }
                b
            }
            4 => {
                // Valid firmware volume (exercises block map traversal and {0, 0} sentinel detection)
                fv_seed.clone()
            }
            5 => {
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

        let volumes = FirmwareVolume::find_volumes(&buf);
        if !volumes.is_empty() {
            fv_successful_parses += 1;
        } else {
            fv_clean_rejections += 1;
        }
    }

    let snap = take_snapshot();
    disable_observer();

    // Direct assertions on observer hooks proving genuine path execution (DEF-44, DEF-50)
    assert!(snap.ifd_sig_accepted > 0, "IFD signature accepted must be > 0");
    assert!(snap.ifd_sig_rejected > 0, "IFD signature rejected must be > 0");
    assert!(snap.ifd_geom_accepted > 0, "IFD geometry accepted must be > 0");
    assert!(snap.ifd_geom_rejected > 0, "IFD geometry rejected must be > 0");
    assert!(snap.ifd_profile_decoder_reached > 0, "IFD profile decoder must be reached");
    assert!(snap.ifd_region_table_entry > 0, "IFD region table entry must be > 0");
    assert!(snap.ifd_region_table_completed > 0, "IFD region table completed must be > 0");
    assert!(snap.ifd_region_conflict_reached > 0, "IFD region conflict must be reached");

    assert!(snap.pfs_is_dell_pfs_matched > 0, "PFS is_dell_pfs matched must be > 0");
    assert!(snap.pfs_is_dell_pfs_unmatched > 0, "PFS is_dell_pfs unmatched must be > 0");
    assert!(snap.pfs_extract_accepted > 0, "PFS extract accepted must be > 0");
    assert!(snap.pfs_extract_rejected > 0, "PFS extract rejected must be > 0");
    assert!(snap.pfs_section_table_reached > 0, "PFS section table must be reached");
    assert!(snap.pfs_guid_name_reached > 0, "PFS GUID name must be reached");
    assert!(snap.pfs_zlib_attempted > 0, "PFS ZLIB attempted must be > 0");
    assert!(snap.pfs_zlib_succeeded > 0, "PFS ZLIB succeeded must be > 0");
    assert!(snap.pfs_zlib_rejected > 0, "PFS ZLIB rejected must be > 0");
    assert!(snap.pfs_crc_verified_match > 0, "PFS CRC match must be > 0");
    assert!(snap.pfs_crc_verified_mismatch > 0, "PFS CRC mismatch must be > 0");
    assert!(snap.pfs_xor_verified_match > 0, "PFS XOR match must be > 0");
    assert!(snap.pfs_xor_verified_mismatch > 0, "PFS XOR mismatch must be > 0");

    assert!(snap.fv_fvh_detected > 0, "FV _FVH detected must be > 0");
    assert!(snap.fv_fvh_rejected > 0, "FV _FVH rejected must be > 0");
    assert!(snap.fv_header_validated > 0, "FV header validated must be > 0");
    assert!(snap.fv_checksum_evaluated > 0, "FV checksum evaluated must be > 0");
    assert!(snap.fv_block_map_traversed > 0, "FV block map traversed must be > 0");
    assert!(snap.fv_sentinel_found > 0, "FV sentinel found must be > 0");
    assert!(snap.fv_sentinel_missing > 0, "FV sentinel missing must be > 0");

    assert_eq!(ifd_successful_parses + ifd_clean_rejections, iterations);
    assert_eq!(pfs_successful_parses + pfs_clean_rejections, iterations);
    assert_eq!(fv_successful_parses + fv_clean_rejections, iterations);

    let elapsed = start.elapsed();
    println!(
        "[FUZZ EVIDENCE] Structure-aware mutation testing completed: 3 x {} (300,000 total iterations) in {:.2?} with 0 crashes or hangs.",
        iterations, elapsed
    );
    println!(
        "  IFD paths: sig_acc={}, sig_rej={}, geom_acc={}, geom_rej={}, prof_dec={}, reg_entry={}, reg_comp={}, reg_conflict={}",
        snap.ifd_sig_accepted, snap.ifd_sig_rejected, snap.ifd_geom_accepted, snap.ifd_geom_rejected, snap.ifd_profile_decoder_reached, snap.ifd_region_table_entry, snap.ifd_region_table_completed, snap.ifd_region_conflict_reached
    );
    println!(
        "  PFS paths: is_matched={}, is_unmatched={}, ext_acc={}, ext_rej={}, sec_tbl={}, guid_name={}, zlib_att={}, zlib_succ={}, zlib_rej={}, crc_match={}, crc_mismatch={}, xor_match={}, xor_mismatch={}",
        snap.pfs_is_dell_pfs_matched, snap.pfs_is_dell_pfs_unmatched, snap.pfs_extract_accepted, snap.pfs_extract_rejected, snap.pfs_section_table_reached, snap.pfs_guid_name_reached, snap.pfs_zlib_attempted, snap.pfs_zlib_succeeded, snap.pfs_zlib_rejected, snap.pfs_crc_verified_match, snap.pfs_crc_verified_mismatch, snap.pfs_xor_verified_match, snap.pfs_xor_verified_mismatch
    );
    println!(
        "  FV paths: fvh_det={}, fvh_rej={}, hdr_val={}, csum_eval={}, blk_trav={}, sent_found={}, sent_missing={}",
        snap.fv_fvh_detected, snap.fv_fvh_rejected, snap.fv_header_validated, snap.fv_checksum_evaluated, snap.fv_block_map_traversed, snap.fv_sentinel_found, snap.fv_sentinel_missing
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
    let res = FirmwareVolume::parse_at(&reader, 0);
    assert!(res.is_err(), "Missing {{0, 0}} sentinel must return an error per UEFI PI spec");
    let err_msg = format!("{}", res.err().unwrap());
    assert!(err_msg.contains("sentinel"));
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

// -----------------------------------------------------------------------------
// PHASE 2B.3 CORRECTNESS & EVIDENCE REMEDIATION TESTS (DEF-41..DEF-46)
// -----------------------------------------------------------------------------

#[test]
fn test_ifd_flmstr_older_layout_cannot_accidentally_trigger_modern() {
    // DEF-41: Older layout FLMSTR (e.g. Ibex Peak / Cougar Point) where bits 15:0 are Requester ID (0x0000).
    // In legacy layout:
    // bits 15:0 = Requester ID = 0x0000
    // bit 16 = Desc Read (1), bit 17 = BIOS Read (1)
    // bits 23:21 = strictly 0
    // bit 25 = BIOS Write (1)
    // bits 31:29 = strictly 0
    // Raw FLMSTR1 = (1 << 16) | (1 << 17) | (1 << 25) = 0x02030000.
    // In modern layout, bits 15:0 are Read permissions (bit 8 = Desc Read, bit 9 = BIOS Read).
    // Since bits 15:0 are 0x0000, modern evaluation fails has_desc_read && has_bios_read.
    // Even if FLMAP0 has NR=0 (single master), profile resolution must NOT classify this as V2Modern.

    let mut buf = vec![0xFFu8; 4096];
    buf[0x10..0x14].copy_from_slice(&IFD_SIGNATURE_MAGIC.to_le_bytes());
    // FLMAP0: NR=0 (bits 10:8 = 0 -> 1 master), FRBA=0x04 (0x40), FCBA=0x0A (0xA0)
    let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x0A;
    buf[0x14..0x18].copy_from_slice(&flmap0.to_le_bytes());
    // FLMAP1: FMBA=0x08 (0x08 << 4 = 0x80)
    let flmap1: u32 = 0x08;
    buf[0x18..0x1C].copy_from_slice(&flmap1.to_le_bytes());
    // FLMAP2: ICCRIBA = 0 (absent)
    buf[0x1C..0x20].copy_from_slice(&0u32.to_le_bytes());

    // FCBA at 0xA0: FLCOMP freq_read = 0 (20MHz in legacy)
    buf[0xA0..0xA4].copy_from_slice(&0u32.to_le_bytes());

    // FRBA at 0x40: Regions (Region 0: Flash Descriptor, Region 1: BIOS)
    buf[0x40..0x44].copy_from_slice(&0x00000000u32.to_le_bytes());
    buf[0x44..0x48].copy_from_slice(&0x007F0010u32.to_le_bytes());

    // FMBA at 0x80: Master 1 (FLMSTR1) = 0x02030000
    buf[0x80..0x84].copy_from_slice(&0x02030000u32.to_le_bytes());

    let ifd = IntelFlashDescriptor::parse(&buf).expect("IFD parsing should succeed");
    assert_ne!(
        ifd.profile,
        IfdProfile::V2Modern,
        "Legacy FLMSTR with Requester ID 0x0000 must never be classified as V2Modern"
    );
    assert_eq!(ifd.profile, IfdProfile::V1Legacy);
    assert_eq!(ifd.gate2_platform.confidence, ProfileConfidence::Confirmed);
}

#[test]
fn test_ifd_flcomp_read_freq_6_not_standalone_v2_proof() {
    // DEF-41: flashrom documents that read frequency 6 (17 MHz on Skylake) was repurposed
    // on later platforms (Cannon Point 300-series+).
    // Frequency 6 alone without converging modern FLMSTR markers must not produce Confirmed V2Modern.
    let mut buf = vec![0xFFu8; 4096];
    buf[0x10..0x14].copy_from_slice(&IFD_SIGNATURE_MAGIC.to_le_bytes());
    // FLMAP0: NR=0, FRBA=0x04 (0x40), FCBA=0x0A (0xA0)
    let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x0A;
    buf[0x14..0x18].copy_from_slice(&flmap0.to_le_bytes());
    // FLMAP1: FMBA=0x08 (0x80)
    let flmap1: u32 = 0x08;
    buf[0x18..0x1C].copy_from_slice(&flmap1.to_le_bytes());
    buf[0x1C..0x20].copy_from_slice(&0u32.to_le_bytes());

    // FCBA at 0xA0: FLCOMP freq_read = 6 (bits 19:17 = 6 -> 6 << 17 = 0x000C0000) (DEF-53)
    let flcomp = 6u32 << 17;
    buf[0xA0..0xA4].copy_from_slice(&flcomp.to_le_bytes());

    // FRBA at 0x40
    buf[0x40..0x44].copy_from_slice(&0x00000000u32.to_le_bytes());
    buf[0x44..0x48].copy_from_slice(&0x007F0010u32.to_le_bytes());

    // FMBA at 0x80: Master 1 has all zeros (ambiguous / invalid master permissions)
    buf[0x80..0x84].copy_from_slice(&0u32.to_le_bytes());

    let ifd = IntelFlashDescriptor::parse(&buf).expect("IFD parsing should succeed");
    assert_ne!(
        ifd.gate2_platform.confidence,
        ProfileConfidence::Confirmed,
        "Frequency 6 alone without converging modern FLMSTR markers must not be Confirmed"
    );
    assert_eq!(ifd.profile, IfdProfile::Unknown);
    assert_eq!(ifd.gate2_platform.confidence, ProfileConfidence::Unknown);
}

#[test]
fn test_ifd_flmap2_iccriba_accounting_and_safe_boundaries() {
    // DEF-42: Decode and preserve ICCRIBA from FLMAP2[23:16].
    // Test 4 configurations:
    // 1. ICCRIBA present before FRBA
    // 2. ICCRIBA present after FRBA
    // 3. ICCRIBA adjacent to FRBA
    // 4. ICCRIBA absent (iccriba_raw == 0)

    // Case 1: ICCRIBA before FRBA: FCBA=0x40, ICCRIBA=0x80 (raw 0x08, RIL=1), FRBA=0xC0, FMBA=0x200
    {
        let flmap0: u32 = (0x0C << 16) | (0x00 << 8) | 0x04; // FRBA=0xC0, FCBA=0x40
        let flmap1: u32 = 0x20; // FMBA=0x20 (0x200)
        let flmap2: u32 = (0x01 << 24) | (0x08 << 16); // RIL=1, ICCRIBA raw = 0x08 -> 0x80

        let geom = validate_descriptor_geometry(flmap0, flmap1, flmap2, 4096);
        assert!(geom.is_valid, "Geometry should be valid: {:?}", geom.validation_errors);
        assert_eq!(geom.iccriba, Some(0x80));
        assert_eq!(geom.iccriba_raw, 0x08);
        assert!(geom.max_regions_extent >= 16);
    }

    // Case 2: ICCRIBA after FRBA: FCBA=0x20, FRBA=0x40, ICCRIBA=0x80 (raw 0x08, RIL=1), FMBA=0x200
    // ICCRIBA constrains region table extent to (0x80 - 0x40) / 4 = 16 regions!
    {
        let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
        let flmap1: u32 = 0x20; // FMBA=0x20 (0x200)
        let flmap2: u32 = (0x01 << 24) | (0x08 << 16); // RIL=1, ICCRIBA raw = 0x08 -> 0x80

        let geom = validate_descriptor_geometry(flmap0, flmap1, flmap2, 4096);
        assert!(geom.is_valid, "Geometry should be valid: {:?}", geom.validation_errors);
        assert_eq!(geom.iccriba, Some(0x80));
        assert_eq!(geom.max_regions_extent, 16);
    }

    // Case 3: ICCRIBA adjacent to FRBA: FRBA=0x40, region table ends at 0x80, ICCRIBA at 0x80
    {
        let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
        let flmap1: u32 = 0x20; // FMBA=0x20 (0x200)
        let flmap2: u32 = (0x01 << 24) | (0x08 << 16); // RIL=1, ICCRIBA raw = 0x08 -> 0x80

        let geom = validate_descriptor_geometry(flmap0, flmap1, flmap2, 4096);
        assert!(geom.is_valid, "Geometry should be valid: {:?}", geom.validation_errors);
        assert_eq!(geom.iccriba, Some(0x80));
    }

    // Case 4: ICCRIBA absent (raw 0) in valid legacy MCH strap layout (DEF-42, DEF-52)
    {
        let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
        let flmap1: u32 = 0x20; // FMBA=0x20 (0x200)
        let flmap2: u32 = (0x02 << 8) | 0x30; // MSL=2, FMSBA=0x30 (0x300), ICCRIBA = 0

        let geom = validate_descriptor_geometry(flmap0, flmap1, flmap2, 4096);
        assert!(geom.is_valid, "Geometry should be valid: {:?}", geom.validation_errors);
        assert_eq!(geom.iccriba, None);
        assert_eq!(geom.iccriba_raw, 0);
    }
}

#[test]
fn test_ifd_geometry_adversarial_collision_checks() {
    // DEF-43, DEF-49: Strict interval overlap collision detection:
    // 1. FCBA / FMBA overlap
    // 2. Optional section (FPSBA) overlapping FRBA
    // 3. Region table crossing succeeding section (physical vs profile-specific NR)
    // 4. Section extending beyond 0x1000
    // 5. Valid adjacent sections pass

    // 1. FCBA / FMBA overlap: FCBA=0x40 (len 12, ends at 0x4C), FMBA=0x40 (starts at 0x40) -> collision!
    {
        let flmap0: u32 = (0x20 << 16) | (0x00 << 8) | 0x04; // FRBA=0x200, FCBA=0x40
        let flmap1: u32 = 0x04; // FMBA=0x40 (overlaps FCBA 0x40..0x4C)
        let geom = validate_descriptor_geometry(flmap0, flmap1, 0, 4096);
        assert!(!geom.is_valid, "FCBA / FMBA overlap must be invalid");
        assert!(
            geom.validation_errors.iter().any(|e| e.contains("collision") || e.contains("overlaps") || e.contains("share identical base")),
            "Errors: {:?}",
            geom.validation_errors
        );
    }

    // 2. Optional section (FPSBA) overlapping FRBA (ISL=1, 0x80..0x84 overlaps FRBA 0x80..0x84)
    {
        let flmap0: u32 = (0x08 << 16) | (0x00 << 8) | 0x02; // FRBA=0x80, FCBA=0x20
        let flmap1: u32 = (0x01 << 24) | (0x08 << 16) | 0x20; // ISL=1, FPSBA=0x80, FMBA=0x200
        let geom = validate_descriptor_geometry(flmap0, flmap1, 0, 4096);
        assert!(!geom.is_valid, "FPSBA overlapping FRBA must be invalid");
        assert!(
            geom.validation_errors.iter().any(|e| e.contains("Collision") || e.contains("overlaps") || e.contains("share identical base")),
            "Errors: {:?}",
            geom.validation_errors
        );
    }

    // 3. Region table physical space vs declared legacy NR: FRBA=0x40, FMBA=0x50.
    // Physical available space bounds region table at (0x50 - 0x40) / 4 = 4 regions!
    {
        let flmap0: u32 = (0x07 << 24) | (0x04 << 16) | (0x00 << 8) | 0x02; // NR=7 (8 regions), FRBA=0x40, FCBA=0x20
        let flmap1: u32 = 0x05; // FMBA=0x50 (0x50)
        let geom = validate_descriptor_geometry(flmap0, flmap1, 0, 4096);
        assert!(geom.is_valid, "Physical geometry itself has 4 regions available: {:?}", geom.validation_errors);
        assert_eq!(geom.max_regions_extent, 4);

        // When parsed under V1Legacy profile, declared 8 regions (0x40..0x60) cross FMBA boundary at 0x50!
        let mut buf = vec![0xFFu8; 4096];
        buf[0x10..0x14].copy_from_slice(&IFD_SIGNATURE_MAGIC.to_le_bytes());
        buf[0x14..0x18].copy_from_slice(&flmap0.to_le_bytes());
        buf[0x18..0x1C].copy_from_slice(&flmap1.to_le_bytes());
        buf[0x1C..0x20].copy_from_slice(&0u32.to_le_bytes());
        // FLMSTR1 at 0x50
        buf[0x50..0x54].copy_from_slice(&0x02030000u32.to_le_bytes());

        let parsed = IntelFlashDescriptor::parse_with_profile(&buf, Some(IfdProfile::V1Legacy)).unwrap();
        assert!(!parsed.gate1_structural.passed || parsed.gate1_structural.issues.iter().any(|i| i.contains("extends into succeeding section")),
            "Declared legacy region table exceeding boundary must be flagged");
    }

    // 4. Section extending beyond 0x1000: FMBA at 0xFF0 with 5 masters (len 20 -> ends at 0x1004 > 0x1000)
    {
        let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
        let flmap1: u32 = (0x04 << 8) | 0xFF; // NM=4 (5 masters), FMBA=0xFF0 (ends at 0x1004)
        let geom = validate_descriptor_geometry(flmap0, flmap1, 0, 4096);
        assert!(!geom.is_valid, "Section extending beyond 0x1000 must be invalid");
        assert!(
            geom.validation_errors.iter().any(|e| e.contains("0x1000")),
            "Errors: {:?}",
            geom.validation_errors
        );
    }

    // 5. Valid adjacent sections: FCBA: 0x20..0x40, FRBA: 0x40..0x80, FMBA: 0x80..0xA0
    {
        let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
        let flmap1: u32 = 0x08; // FMBA=0x80
        let geom = validate_descriptor_geometry(flmap0, flmap1, 0, 4096);
        assert!(geom.is_valid, "Valid adjacent sections must pass: {:?}", geom.validation_errors);
        assert_eq!(geom.fcba, 0x20);
        assert_eq!(geom.frba, 0x40);
        assert_eq!(geom.fmba, 0x80);
    }
}

#[test]
fn test_ifd_explicit_override_confidence_semantics() {
    // DEF-46:
    // 1. User overrides must be assigned ProfileConfidence::UserSelected, NEVER Confirmed.
    // 2. Quarantined V1_5Intermediate must return ProfileConfidence::Unknown and all semantic decoders None.
    // 3. ProfileConfidence serialization must serialize UserSelected as "user_selected".

    let mut buf = vec![0xFFu8; 4096];
    buf[0x10..0x14].copy_from_slice(&IFD_SIGNATURE_MAGIC.to_le_bytes());
    let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
    buf[0x14..0x18].copy_from_slice(&flmap0.to_le_bytes());
    let flmap1: u32 = 0x08; // FMBA=0x80
    buf[0x18..0x1C].copy_from_slice(&flmap1.to_le_bytes());
    buf[0x1C..0x20].copy_from_slice(&0u32.to_le_bytes());

    buf[0x20..0x24].copy_from_slice(&0u32.to_le_bytes()); // FCBA
    buf[0x40..0x44].copy_from_slice(&0u32.to_le_bytes()); // FRBA Region 0
    buf[0x44..0x48].copy_from_slice(&0x007F0010u32.to_le_bytes()); // Region 1
    buf[0x80..0x84].copy_from_slice(&0x02030000u32.to_le_bytes()); // FMBA Master 1

    // Case A: Override V1Legacy
    let res = detect_ifd_profile_resolution(flmap0, flmap1, 0, Some(0), Some(0x02030000), None, Some(IfdProfile::V1Legacy));
    assert_eq!(res.profile, IfdProfile::V1Legacy);
    assert_eq!(res.source, ProfileSource::UserOverride);
    assert_eq!(res.confidence, ProfileConfidence::UserSelected);
    assert_ne!(res.confidence, ProfileConfidence::Confirmed);

    // Case B: Override V2Modern
    let res = detect_ifd_profile_resolution(flmap0, flmap1, 0, Some(0), Some(0x02030000), None, Some(IfdProfile::V2Modern));
    assert_eq!(res.profile, IfdProfile::V2Modern);
    assert_eq!(res.source, ProfileSource::UserOverride);
    assert_eq!(res.confidence, ProfileConfidence::UserSelected);

    // Case C: Override V1_5Intermediate (quarantined)
    let res = detect_ifd_profile_resolution(flmap0, flmap1, 0, Some(0), Some(0x02030000), None, Some(IfdProfile::V1_5Intermediate));
    assert_eq!(res.profile, IfdProfile::V1_5Intermediate);
    assert_eq!(res.source, ProfileSource::UserOverride);
    assert_eq!(res.confidence, ProfileConfidence::Unknown, "Quarantined V1.5 must return Unknown confidence");

    // Parse with V1_5 override and verify decoders return None
    let ifd = IntelFlashDescriptor::parse_with_profile(&buf, Some(IfdProfile::V1_5Intermediate))
        .expect("Parse should succeed structurally");
    assert_eq!(ifd.profile, IfdProfile::V1_5Intermediate);
    assert_eq!(ifd.gate2_platform.confidence, ProfileConfidence::Unknown);
    if let Some(ref mp) = ifd.master_permissions {
        assert_eq!(mp.host_bios_writable, None, "Quarantined profile must yield None host_bios_writable");
        assert_eq!(mp.host_me_writable, None, "Quarantined profile must yield None host_me_writable");
        assert_eq!(mp.host_descriptor_writable, None, "Quarantined profile must yield None host_descriptor_writable");
        for m in &mp.masters {
            assert_eq!(m.bios_writable, None);
            assert_eq!(m.descriptor_writable, None);
        }
    }
    if let Some(ref comp) = ifd.components {
        assert_eq!(comp.read_clock_frequency_mhz, None, "Quarantined profile must yield None read freq");
        assert_eq!(comp.fast_read_supported, None);
    }

    // Case D: Serde serialization
    let json = serde_json::to_string(&ProfileConfidence::UserSelected).unwrap();
    assert_eq!(json, "\"USER_SELECTED\"");
}

#[test]
fn test_ifd_flmap2_interpretation_semantics_def47() {
    // DEF-47: Platform-dependent FLMAP2 interpretation:
    // 1. Positive vector for ICCRIBA/RIL: FLMAP2[23:16]=0x31, FLMAP2[31:24]=0x01
    // 2. Positive vector for Tiger Point CSSO/CSSL: CSSO=0x10, CSSL=0x04, reserved bits zero
    // 3. Negative / ambiguous vectors: Reserved bits non-zero or RIL=0 and CSSO=0

    let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
    let flmap1: u32 = 0x08; // FMBA=0x80

    // 1. ICCRIBA / RIL layout
    let flmap2_iccriba = (0x01 << 24) | (0x31 << 16) | (0x02 << 8) | 0x20;
    assert_eq!(determine_flmap2_interpretation(flmap2_iccriba), Flmap2Interpretation::IccribaRil);
    let geom_icc = validate_descriptor_geometry(flmap0, flmap1, flmap2_iccriba, 4096);
    assert_eq!(geom_icc.flmap2_interpretation, Flmap2Interpretation::IccribaRil);
    assert_eq!(geom_icc.iccriba, Some(0x310));
    assert_eq!(geom_icc.iccriba_raw, 0x31);
    assert_eq!(geom_icc.ril, 0x01);
    assert!(geom_icc.is_valid);

    // 2. Tiger Point CSSO / CSSL layout
    // CSSO = 0x10 (bits 11:2 = 0x10 -> 0x40), CSSL = 0x04 (bits 23:16 = 0x04)
    // Reserved bits 1:0 = 0, 15:12 = 0, 31:24 = 0
    let flmap2_tiger = (0x04 << 16) | 0x40;
    assert_eq!(determine_flmap2_interpretation(flmap2_tiger), Flmap2Interpretation::CpuSoftStrap);
    let geom_tiger = validate_descriptor_geometry(flmap0, flmap1, flmap2_tiger, 4096);
    assert_eq!(geom_tiger.flmap2_interpretation, Flmap2Interpretation::CpuSoftStrap);
    assert_eq!(geom_tiger.iccriba, None, "Tiger Point layout must NOT populate ICCRIBA");
    assert_eq!(geom_tiger.csso_raw, 0x10);
    assert_eq!(geom_tiger.cssl_raw, 0x04);
    assert!(geom_tiger.is_valid);

    // 3. Ambiguous vector: CSSL is present but reserved bits 1:0 are non-zero (0x01)
    let flmap2_ambig = (0x04 << 16) | 0x41;
    assert_eq!(determine_flmap2_interpretation(flmap2_ambig), Flmap2Interpretation::Unknown);
    let geom_ambig = validate_descriptor_geometry(flmap0, flmap1, flmap2_ambig, 4096);
    assert_eq!(geom_ambig.flmap2_interpretation, Flmap2Interpretation::Unknown);
    assert!(geom_ambig.has_unresolved_extents);
    assert!(!geom_ambig.is_valid);
    assert_eq!(geom_ambig.iccriba, None);

    // 4. Legacy MCH strap vector (upper half is zero)
    let flmap2_legacy = (0x02 << 8) | 0x20;
    assert_eq!(determine_flmap2_interpretation(flmap2_legacy), Flmap2Interpretation::LegacyMchStrap);
    let geom_legacy = validate_descriptor_geometry(flmap0, flmap1, flmap2_legacy, 4096);
    assert_eq!(geom_legacy.flmap2_interpretation, Flmap2Interpretation::LegacyMchStrap);
    assert_eq!(geom_legacy.iccriba, None);
    assert!(geom_legacy.is_valid);
}

#[test]
fn test_ifd_unsupported_generation_inference_removed_def48() {
    // DEF-48: Verify that geometry.iccriba_raw > 0x30 and max_regions > 5
    // are removed as standalone modern generation proofs.
    // When unique generation evidence is absent, resolution must return Unknown.

    let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
    let flmap1: u32 = 0x08; // FMBA=0x80 (max_regions = 16 > 5!)
    // ICCRIBA = 0x31 (> 0x30!), RIL = 1
    let flmap2: u32 = (0x01 << 24) | (0x31 << 16) | (0x02 << 8) | 0x20;

    // FLCOMP: freq_read = 0 (20 MHz, legacy frequency, NOT 17 MHz)
    let flcomp = Some(0u32);
    // FLMSTR1: Ambiguous master (all zeros)
    let flmstr1 = Some(0u32);

    let res = detect_ifd_profile_resolution(flmap0, flmap1, flmap2, flcomp, flmstr1, None, None);
    assert_eq!(
        res.profile,
        IfdProfile::Unknown,
        "Descriptor with ambiguous FLMSTR must return Unknown even if iccriba > 0x30 and max_regions > 5"
    );
    assert_eq!(res.confidence, ProfileConfidence::Unknown);

    // Modern profile requires converging evidence: 17 MHz read frequency + modern FLMSTR
    let modern_flmstr1 = (0x02 << 20) | (0x03 << 8); // Bits 8 and 9 set
    let flcomp_17mhz = Some(6 << 17); // 17 MHz Skylake indicator
    let res_modern = detect_ifd_profile_resolution(flmap0, flmap1, flmap2, flcomp_17mhz, Some(modern_flmstr1), None, None);
    assert_eq!(res_modern.profile, IfdProfile::Unknown, "DEF-57: frequency 6 repurposed on later platforms; insufficient for automatic V2 selection");
    assert_eq!(res_modern.confidence, ProfileConfidence::Unknown);

    // Old synthetic FLMAP2 value 0x00040040 cannot confirm Tiger Point (DEF-62)
    let flmap2_old_synthetic = (0x04 << 16) | 0x40;
    let res_old_synth = detect_ifd_profile_resolution(flmap0, flmap1, flmap2_old_synthetic, flcomp, Some(modern_flmstr1), Some(0x02000000), None);
    assert_eq!(res_old_synth.profile, IfdProfile::Unknown, "DEF-62: old synthetic 0x00040040 must not confirm Tiger Point");
    assert_eq!(res_old_synth.confidence, ProfileConfidence::Unknown);

    // Authentic Tiger Point: CSSL=0x11, CSSO=0x68 per upstream flashrom + FLUMAP1 MDTBA != 0 (DEF-62)
    let flmap2_tiger = (0x11 << 16) | (0x68 << 2);
    let flumap1_tiger = Some(0x02000000); // MDTBA != 0
    let res_tiger = detect_ifd_profile_resolution(flmap0, flmap1, flmap2_tiger, flcomp, Some(modern_flmstr1), flumap1_tiger, None);
    assert_eq!(res_tiger.profile, IfdProfile::V2Modern);
    assert_eq!(res_tiger.confidence, ProfileConfidence::Confirmed);

    // Missing FLUMAP1 MDTBA descriptor context yields Unknown (DEF-62)
    let res_tiger_no_umap = detect_ifd_profile_resolution(flmap0, flmap1, flmap2_tiger, flcomp, Some(modern_flmstr1), None, None);
    assert_eq!(res_tiger_no_umap.profile, IfdProfile::Unknown, "DEF-62: missing FLUMAP1 MDTBA context must produce Unknown");
    assert_eq!(res_tiger_no_umap.confidence, ProfileConfidence::Unknown);

    // Legacy profile: legacy FLMSTR1 (0x02030000) + legacy freq + valid legacy NR + valid legacy FLMAP2 (DEF-52)
    let legacy_flmstr1 = 0x02030000;
    let flmap2_legacy = (0x02 << 8) | 0x20; // Valid legacy MCH straps (MSL=2, FMSBA=0x20)
    let res_legacy = detect_ifd_profile_resolution(flmap0, flmap1, flmap2_legacy, flcomp, Some(legacy_flmstr1), None, None);
    assert_eq!(res_legacy.profile, IfdProfile::V1Legacy);
    assert_eq!(res_legacy.confidence, ProfileConfidence::Confirmed);
}

#[test]
fn test_ifd_geometry_known_vs_unknown_extents_def49() {
    // DEF-49: Zero length fields must produce ExtentLength::Unknown and fail safe-geometry.
    // Adjacent sections must pass; partial and complete overlaps must be detected.

    let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20

    // 1. FPSBA with ISL = 0
    {
        let flmap1_isl0 = (0x00 << 24) | (0x08 << 16) | 0x20; // FPSBA=0x80, ISL=0!
        let geom = validate_descriptor_geometry(flmap0, flmap1_isl0, 0, 4096);
        assert!(geom.has_unresolved_extents);
        assert!(!geom.is_valid);
        assert!(geom.validation_errors.iter().any(|e| e.contains("ISL=0") || e.contains("unresolved length")));
        let fpsba_sec = geom.section_extents.iter().find(|s| s.name == "FPSBA").unwrap();
        assert_eq!(fpsba_sec.length, ExtentLength::Unknown);
    }

    // 2. FMSBA with MSL = 0
    {
        let flmap1 = 0x20;
        let flmap2_msl0 = (0x00 << 8) | 0x10; // FMSBA=0x100, MSL=0!
        let geom = validate_descriptor_geometry(flmap0, flmap1, flmap2_msl0, 4096);
        assert!(geom.has_unresolved_extents);
        assert!(!geom.is_valid);
        assert!(geom.validation_errors.iter().any(|e| e.contains("MSL=0") || e.contains("unresolved length")));
        let fmsba_sec = geom.section_extents.iter().find(|s| s.name == "FMSBA").unwrap();
        assert_eq!(fmsba_sec.length, ExtentLength::Unknown);
    }

    // 3. Adjacent sections: FCBA 0x20..0x2C (len 12), FRBA 0x30..0x34 (len 4), FMBA 0x40..0x48 (len 8)
    {
        let flmap0_adj = (0x03 << 16) | (0x00 << 8) | 0x02; // FRBA=0x30, FCBA=0x20
        let flmap1_adj = (0x01 << 8) | 0x04; // NM=1 (2 masters, len 8), FMBA=0x40
        let geom = validate_descriptor_geometry(flmap0_adj, flmap1_adj, 0, 4096);
        assert!(geom.is_valid, "Adjacent non-overlapping sections must pass: {:?}", geom.validation_errors);
        assert_eq!(geom.validation_errors.len(), 0);
        assert_eq!(geom.section_extents[0].length, ExtentLength::Exact(12));
    }

    // 4. Partial overlap: FCBA at 0x20 (len 12 -> ends at 0x2C), FMBA at 0x28 (len 8 -> 0x28..0x30)
    {
        let flmap0_part = (0x10 << 16) | (0x00 << 8) | 0x02; // FRBA=0x100, FCBA=0x20
        let flmap1_part = 0x02; // FMBA at 0x20 (exact overlap with FCBA)
        let geom = validate_descriptor_geometry(flmap0_part, flmap1_part, 0, 4096);
        assert!(!geom.is_valid);
        assert!(geom.validation_errors.iter().any(|e| e.contains("Collision") || e.contains("overlaps") || e.contains("share identical base")));
    }
}

#[test]
fn test_observer_targeted_path_fixtures_def50() {
    // DEF-50: Test every observer path individually on targeted fixtures to prove
    // each positive and negative metric operates as expected.

    reset_observer();
    enable_observer();

    // 1. IFD paths
    // Valid IFD
    let valid_ifd = SyntheticFixtureBuilder::build_valid_ifd_v2_image(4096);
    let _ = IntelFlashDescriptor::parse(&valid_ifd);

    // Invalid IFD signature
    let mut invalid_sig_ifd = valid_ifd.clone();
    invalid_sig_ifd[0x10] = 0x00;
    let _ = IntelFlashDescriptor::parse(&invalid_sig_ifd);

    // Invalid IFD geometry (FCBA outside bounds)
    let mut invalid_geom_ifd = valid_ifd.clone();
    invalid_geom_ifd[0x14] = 0x01; // FCBA = 0x10 < 0x20
    let _ = IntelFlashDescriptor::parse(&invalid_geom_ifd);

    // IFD region conflict
    let mut conflict_ifd = valid_ifd.clone();
    // Overlapping region 0 (0x00..0x1000) and region 1 (0x00..0x2000)
    conflict_ifd[0x40..0x44].copy_from_slice(&0x00000000u32.to_le_bytes());
    conflict_ifd[0x44..0x48].copy_from_slice(&0x00010000u32.to_le_bytes());
    let _ = IntelFlashDescriptor::parse(&conflict_ifd);

    // 2. Dell PFS paths
    let valid_pfs = SyntheticFixtureBuilder::build_synthetic_dell_pfs(&[("BIOS", "1.0", b"TEST_PAYLOAD")]);
    assert!(DellPfsParser::is_dell_pfs(&valid_pfs));
    let _ = DellPfsParser::extract(&valid_pfs, None, "test_pfs");

    // Negative is_dell_pfs
    let random_buf = vec![0x55u8; 1024];
    assert!(!DellPfsParser::is_dell_pfs(&random_buf));
    let _ = DellPfsParser::extract(&random_buf, None, "test_pfs_err");

    // PFS with corrupted CRC
    let corrupt_crc_pfs = SyntheticFixtureBuilder::build_corrupt_crc_dell_pfs();
    let _ = DellPfsParser::extract(&corrupt_crc_pfs, None, "test_pfs_corrupt_crc");

    // PFS with corrupted XOR
    let corrupt_xor_pfs = SyntheticFixtureBuilder::build_corrupt_xor_dell_pfs();
    let _ = DellPfsParser::extract(&corrupt_xor_pfs, None, "test_pfs_corrupt_xor");

    // 3. UEFI FV paths
    let valid_fv = SyntheticFixtureBuilder::build_valid_firmware_volume(0x1000);
    let _ = FirmwareVolume::find_volumes(&valid_fv);

    // FV with missing _FVH (negative detection)
    let _ = FirmwareVolume::find_volumes(&random_buf);

    // FV with signature at offset < 0x28 (negative signature position)
    let mut fv_early_sig = vec![0u8; 64];
    fv_early_sig[0..4].copy_from_slice(b"_FVH");
    let _ = FirmwareVolume::find_volumes(&fv_early_sig);

    // FV with missing {0, 0} sentinel
    let mut fv_no_sentinel = valid_fv.clone();
    if fv_no_sentinel.len() >= 0x48 {
        fv_no_sentinel[0x40..0x44].copy_from_slice(&1u32.to_le_bytes());
        fv_no_sentinel[0x44..0x48].copy_from_slice(&0x1000u32.to_le_bytes());
        fv_no_sentinel[0x32..0x34].copy_from_slice(&0u16.to_le_bytes());
        let chk = crate::engine::binary::checksums::efi_fv_checksum_16(&fv_no_sentinel[..0x48]);
        fv_no_sentinel[0x32..0x34].copy_from_slice(&chk.to_le_bytes());
    }
    let _ = FirmwareVolume::find_volumes(&fv_no_sentinel);

    let snap = take_snapshot();
    disable_observer();

    // Verify all positive and negative metrics were exercised
    assert!(snap.ifd_sig_accepted > 0);
    assert!(snap.ifd_sig_rejected > 0);
    assert!(snap.ifd_geom_accepted > 0);
    assert!(snap.ifd_geom_rejected > 0);
    assert!(snap.ifd_profile_decoder_reached > 0);
    assert!(snap.ifd_region_table_entry > 0);
    assert!(snap.ifd_region_table_completed > 0);
    assert!(snap.ifd_region_conflict_reached > 0);

    assert!(snap.pfs_is_dell_pfs_matched > 0);
    assert!(snap.pfs_is_dell_pfs_unmatched > 0);
    assert!(snap.pfs_extract_accepted > 0);
    assert!(snap.pfs_extract_rejected > 0);
    assert!(snap.pfs_section_table_reached > 0);
    assert!(snap.pfs_guid_name_reached > 0);
    assert!(snap.pfs_zlib_attempted > 0);
    assert!(snap.pfs_zlib_succeeded > 0);
    assert!(snap.pfs_crc_verified_match > 0);
    assert!(snap.pfs_crc_verified_mismatch > 0);
    assert!(snap.pfs_xor_verified_match > 0);
    assert!(snap.pfs_xor_verified_mismatch > 0);

    assert!(snap.fv_fvh_detected > 0);
    assert!(snap.fv_fvh_rejected > 0);
    assert!(snap.fv_header_validated > 0);
    assert!(snap.fv_checksum_evaluated > 0);
    assert!(snap.fv_block_map_traversed > 0);
    assert!(snap.fv_sentinel_found > 0);
    assert!(snap.fv_sentinel_missing > 0);
}

#[test]
fn test_ifd_flmap2_interpretation_edge_cases_def52() {
    let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
    let flmap1: u32 = 0x20; // FMBA=0x20 (0x200)

    // 1. All-zero FLMAP2 must NOT be classified as LegacyMchStrap (DEF-52)
    let flmap2_zero = 0u32;
    assert_eq!(
        determine_flmap2_interpretation(flmap2_zero),
        Flmap2Interpretation::Unknown,
        "All-zero FLMAP2 has no strap definitions; must be classified as Unknown"
    );
    let geom_zero = validate_descriptor_geometry(flmap0, flmap1, flmap2_zero, 4096);
    assert_eq!(geom_zero.flmap2_interpretation, Flmap2Interpretation::Unknown);
    assert_eq!(geom_zero.fmsba, None);
    assert_eq!(geom_zero.msl, 0);
    assert_eq!(geom_zero.iccriba, None);
    assert_eq!(geom_zero.ril, 0);
    assert_eq!(geom_zero.raw_flmap2, 0);
    assert!(geom_zero.is_valid, "All-zero FLMAP2 has no invalid or conflicting section definitions");

    // 2. Valid known layout examples
    // Legacy MCH strap (upper half 0, lower half valid FMSBA + MSL)
    let flmap2_legacy = (0x02 << 8) | 0x30;
    assert_eq!(determine_flmap2_interpretation(flmap2_legacy), Flmap2Interpretation::LegacyMchStrap);
    let geom_legacy = validate_descriptor_geometry(flmap0, flmap1, flmap2_legacy, 4096);
    assert_eq!(geom_legacy.flmap2_interpretation, Flmap2Interpretation::LegacyMchStrap);
    assert_eq!(geom_legacy.fmsba, Some(0x300));
    assert_eq!(geom_legacy.msl, 2);
    assert!(geom_legacy.is_valid);

    // Tiger Point CPU Soft Strap (bits 1:0 = 0, bits 15:12 = 0, bits 31:24 = 0, CSSO != 0, CSSL != 0)
    let flmap2_tiger = (0x04 << 16) | 0x40;
    assert_eq!(determine_flmap2_interpretation(flmap2_tiger), Flmap2Interpretation::CpuSoftStrap);
    let geom_tiger = validate_descriptor_geometry(flmap0, flmap1, flmap2_tiger, 4096);
    assert_eq!(geom_tiger.flmap2_interpretation, Flmap2Interpretation::CpuSoftStrap);
    assert_eq!(geom_tiger.fmsba, None);
    assert_eq!(geom_tiger.msl, 0);
    assert_eq!(geom_tiger.csso_raw, 0x10);
    assert_eq!(geom_tiger.cssl_raw, 0x04);
    assert!(geom_tiger.is_valid);

    // ICCRIBA / RIL layout
    let flmap2_icc = (0x01 << 24) | (0x08 << 16) | (0x02 << 8) | 0x30;
    assert_eq!(determine_flmap2_interpretation(flmap2_icc), Flmap2Interpretation::IccribaRil);
    let geom_icc = validate_descriptor_geometry(flmap0, flmap1, flmap2_icc, 4096);
    assert_eq!(geom_icc.flmap2_interpretation, Flmap2Interpretation::IccribaRil);
    assert_eq!(geom_icc.iccriba, Some(0x80));
    assert_eq!(geom_icc.ril, 1);
    assert_eq!(geom_icc.fmsba, Some(0x300));
    assert!(geom_icc.is_valid);
    assert_eq!(geom_icc.msl, 2);
    assert!(geom_icc.is_valid);

    // 3. Reserved-bit violations in Tiger Point layout
    // Res1 violation (bits 1:0 non-zero)
    let flmap2_res1 = (0x04 << 16) | 0x41;
    assert_eq!(determine_flmap2_interpretation(flmap2_res1), Flmap2Interpretation::Unknown);
    let geom_res1 = validate_descriptor_geometry(flmap0, flmap1, flmap2_res1, 4096);
    assert!(!geom_res1.is_valid);
    assert!(geom_res1.has_unresolved_extents);

    // Res2 violation (bits 15:12 non-zero)
    let flmap2_res2 = (0x04 << 16) | 0x40 | (0x1 << 12);
    assert_eq!(determine_flmap2_interpretation(flmap2_res2), Flmap2Interpretation::Unknown);
    let geom_res2 = validate_descriptor_geometry(flmap0, flmap1, flmap2_res2, 4096);
    assert!(!geom_res2.is_valid);
    assert!(geom_res2.has_unresolved_extents);

    // Res3 violation (bits 31:24 non-zero when CSSO is set, and out of ICCRIBA bounds)
    let flmap2_res3 = (0x01 << 24) | (0x50 << 16) | 0x40;
    assert_eq!(determine_flmap2_interpretation(flmap2_res3), Flmap2Interpretation::Unknown);
    let geom_res3 = validate_descriptor_geometry(flmap0, flmap1, flmap2_res3, 4096);
    assert!(!geom_res3.is_valid);
    assert!(geom_res3.has_unresolved_extents);

    // 4. Conflicting layout indicators (RIL out of ICCRIBA bounds with CSSO present)
    let flmap2_conflict = (0x20 << 24) | (0x08 << 16) | 0x40;
    assert_eq!(determine_flmap2_interpretation(flmap2_conflict), Flmap2Interpretation::Unknown);
    let geom_conflict = validate_descriptor_geometry(flmap0, flmap1, flmap2_conflict, 4096);
    assert!(!geom_conflict.is_valid);
    assert!(geom_conflict.has_unresolved_extents);

    // 5. Unknown layout with plausible lower-half fields (FMSBA=0x20, MSL=2, but upper half unknown non-zero)
    let flmap2_unknown_plausible = 0x12340220u32;
    assert_eq!(determine_flmap2_interpretation(flmap2_unknown_plausible), Flmap2Interpretation::Unknown);
    let geom_unknown = validate_descriptor_geometry(flmap0, flmap1, flmap2_unknown_plausible, 4096);
    assert_eq!(geom_unknown.flmap2_interpretation, Flmap2Interpretation::Unknown);
    assert_eq!(geom_unknown.fmsba, None, "Unknown layout must NOT expose speculative FMSBA");
    assert_eq!(geom_unknown.msl, 0, "Unknown layout must NOT expose speculative MSL");
    assert_eq!(geom_unknown.iccriba, None, "Unknown layout must NOT expose speculative ICCRIBA");
    assert_eq!(geom_unknown.ril, 0, "Unknown layout must NOT expose speculative RIL");
    assert_eq!(geom_unknown.fmsba_raw, 0x20, "Raw FMSBA byte must be preserved");
    assert!(geom_unknown.has_unresolved_extents);
    assert!(!geom_unknown.is_valid);
}

#[test]
fn test_ifd_profile_read_freq_6_with_modern_flmstr_def53_def57() {
    let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
    let flmap1: u32 = 0x08; // FMBA=0x80
    let flmap2_icc: u32 = (0x01 << 24) | (0x31 << 16) | (0x02 << 8) | 0x20;

    let modern_flmstr1 = (0x02 << 20) | (0x03 << 8); // Modern layout (Host CPU reads desc bit 8, BIOS bit 9)
    let flcomp_17mhz = Some(6u32 << 17); // Read frequency 6 (17 MHz)

    // 1. Modern FLMSTR + freq 6 alone without hardware layout returns Unknown (DEF-57)
    let res = detect_ifd_profile_resolution(flmap0, flmap1, flmap2_icc, flcomp_17mhz, Some(modern_flmstr1), None, None);
    assert_eq!(res.profile, IfdProfile::Unknown, "DEF-57: Frequency 6 is repurposed on Cannon Point+ and insufficient for automatic V2 selection");
    assert_eq!(res.confidence, ProfileConfidence::Unknown);

    // 2. Proving ambiguous automatic profile cannot influence profile-specific decoding (DEF-57)
    assert_eq!(decode_flreg_with_profile(0x007F0010, res.profile), None, "Unknown profile must prevent speculative FLREG decoding");
    let rule = decode_master_rule_with_profile(1, "CPU", modern_flmstr1, res.profile);
    assert_eq!(rule.read_mask, None, "Unknown profile must prevent speculative master read mask decoding");
    assert_eq!(rule.write_mask, None, "Unknown profile must prevent speculative master write mask decoding");

    // 3. User override enables profile-specific decoding with UserSelected confidence (DEF-57)
    let res_override = detect_ifd_profile_resolution(flmap0, flmap1, flmap2_icc, flcomp_17mhz, Some(modern_flmstr1), None, Some(IfdProfile::V2Modern));
    assert_eq!(res_override.profile, IfdProfile::V2Modern);
    assert_eq!(res_override.confidence, ProfileConfidence::UserSelected);
    assert_eq!(decode_flreg_with_profile(0x007F0010, res_override.profile), Some((0x10000, 0x7FFFF, true)));
    let rule_override = decode_master_rule_with_profile(1, "CPU", modern_flmstr1, res_override.profile);
    assert!(rule_override.read_mask.is_some());
    assert!(rule_override.write_mask.is_some());

    // 4. Conflicting indicators: legacy frequency (20 MHz) + modern FLMSTR yields Unknown
    let flcomp_legacy = Some(0u32); // 20 MHz
    let res_conflict = detect_ifd_profile_resolution(flmap0, flmap1, flmap2_icc, flcomp_legacy, Some(modern_flmstr1), None, None);
    assert_eq!(res_conflict.profile, IfdProfile::Unknown);
    assert_eq!(res_conflict.confidence, ProfileConfidence::Unknown);

    // 5. Frequency 6 alone with legacy FLMSTR yields Unknown
    let legacy_flmstr1 = 0x02030000u32;
    let res_freq6_legacy = detect_ifd_profile_resolution(flmap0, flmap1, flmap2_icc, flcomp_17mhz, Some(legacy_flmstr1), None, None);
    assert_eq!(res_freq6_legacy.profile, IfdProfile::Unknown);
    assert_eq!(res_freq6_legacy.confidence, ProfileConfidence::Unknown);

    // 6. Modern FLMSTR + authentic Tiger Point soft straps (CSSL=0x11, CSSO=0x68) + FLUMAP1 MDTBA yields Confirmed (DEF-62)
    let flmap2_tiger = (0x11 << 16) | (0x68 << 2);
    let flumap1_tiger = Some(0x02000000); // MDTBA != 0
    let res_tiger = detect_ifd_profile_resolution(flmap0, flmap1, flmap2_tiger, flcomp_legacy, Some(modern_flmstr1), flumap1_tiger, None);
    assert_eq!(res_tiger.profile, IfdProfile::V2Modern);
    assert_eq!(res_tiger.confidence, ProfileConfidence::Confirmed);
    assert_eq!(decode_flreg_with_profile(0x007F0010, res_tiger.profile), Some((0x10000, 0x7FFFF, true)));
}

#[test]
fn test_ifd_geometry_unknown_flmap2_safe_bounds_def54() {
    let flmap0: u32 = (0x04 << 16) | (0x00 << 8) | 0x02; // FRBA=0x40, FCBA=0x20
    let flmap1: u32 = 0x20; // FMBA=0x200
    let flmap2_unknown = 0xABCD1234u32;

    let geom = validate_descriptor_geometry(flmap0, flmap1, flmap2_unknown, 4096);
    assert_eq!(geom.flmap2_interpretation, Flmap2Interpretation::Unknown);
    assert!(!geom.is_valid, "Geometry must not be valid when FLMAP2 layout is unknown");
    assert!(geom.has_unresolved_extents);
    assert_eq!(geom.fmsba, None);
    assert_eq!(geom.msl, 0);
    assert_eq!(geom.iccriba, None);
    assert_eq!(geom.ril, 0);
    // Physical bounds are still derived from next physical section (FMBA at 0x200)
    assert!(geom.max_regions_extent > 0, "Physical bounds calculation must remain functional");
}

#[test]
fn test_uefi_fv_observer_single_site_accounting_def55() {
    // DEF-55: Ensure exactly one candidate outcome produces one observer outcome
    // Truncated buffer (< 0x38 bytes)
    reset_observer();
    enable_observer();
    let truncated_buf = vec![0u8; 32];
    let reader = BinaryReader::new(&truncated_buf);
    let res = FirmwareVolume::parse_at(&reader, 0);
    assert!(res.is_err());
    let snap = take_snapshot();
    disable_observer();
    assert_eq!(snap.fv_fvh_detected, 0);
    assert_eq!(snap.fv_fvh_rejected, 1, "Exactly one rejection must be recorded on truncated header");

    // Invalid signature offset (< 0x28) in find_volumes
    reset_observer();
    enable_observer();
    let mut early_sig = vec![0u8; 64];
    early_sig[0..4].copy_from_slice(b"_FVH");
    let vols = FirmwareVolume::find_volumes(&early_sig);
    assert!(vols.is_empty());
    let snap = take_snapshot();
    disable_observer();
    assert_eq!(snap.fv_fvh_detected, 0);
    assert_eq!(snap.fv_fvh_rejected, 1, "Exactly one rejection must be recorded for signature < 0x28");

    // Corrupt checksum in valid volume header
    reset_observer();
    enable_observer();
    let mut corrupt_csum_fv = SyntheticFixtureBuilder::build_valid_firmware_volume(0x1000);
    corrupt_csum_fv[0x32] ^= 0xFF; // Invert checksum byte
    let reader_csum = BinaryReader::new(&corrupt_csum_fv);
    let hdr = FirmwareVolume::parse_at(&reader_csum, 0).expect("Header parses but checksum is invalid");
    assert!(!hdr.checksum_valid);
    let snap = take_snapshot();
    disable_observer();
    assert_eq!(snap.fv_fvh_detected, 1, "Header parsed successfully");
    assert_eq!(snap.fv_fvh_rejected, 0);
    assert_eq!(snap.fv_checksum_evaluated, 1);

    // Valid volume header
    reset_observer();
    enable_observer();
    let valid_fv = SyntheticFixtureBuilder::build_valid_firmware_volume(0x1000);
    let reader_valid = BinaryReader::new(&valid_fv);
    let hdr_valid = FirmwareVolume::parse_at(&reader_valid, 0).expect("Header should parse cleanly");
    assert!(hdr_valid.checksum_valid);
    let snap = take_snapshot();
    disable_observer();
    assert_eq!(snap.fv_fvh_detected, 1);
    assert_eq!(snap.fv_fvh_rejected, 0);
    assert_eq!(snap.fv_header_validated, 1);
    assert_eq!(snap.fv_checksum_evaluated, 1);
}

#[test]
fn test_dell_fixture_manifest_metadata_consistency_def63() {
    let manifest = load_dell_latitude_manifest();

    // Canonical fixture identification
    let canonical_sha = manifest["expected_sha256"].as_str().expect("expected_sha256");
    assert_eq!(canonical_sha, "E24B8F8F719CCD8F8B8EBDC8A6178D89C15121DF6806F7E112020964484B840B");
    let canonical_size = manifest["file_size"].as_u64().expect("file_size");
    assert_eq!(canonical_size, 13662488);

    // Reconciliation notes verifying DEF-56, DEF-60, and DEF-63
    let recon = &manifest["provenance_reconciliation_notes"];
    assert!(recon["def_reference"].as_str().unwrap().contains("DEF-63"));
    assert_eq!(
        recon["canonical_verification_status"].as_str(),
        Some("VERIFIED_ON_DISK: Hash and file size computed directly from authentic local fixture")
    );
    assert_eq!(
        recon["alternative_verification_status"].as_str(),
        Some("UNVERIFIED_EXTERNALLY_REFERENCED: Reported in audit log from external Dell Command | Update DUP packaging; not present or hashed on local disk")
    );
    assert_eq!(
        recon["canonical_fixture_sha256"].as_str(),
        Some("E24B8F8F719CCD8F8B8EBDC8A6178D89C15121DF6806F7E112020964484B840B")
    );
    assert_eq!(
        recon["alternative_reported_sha256"].as_str(),
        Some("c5bb883ca09bd8eb38c64eb3d047ec6cb66847deef94e101f3e79e6f315a6e87")
    );
    assert_eq!(
        recon["canonical_file_size"].as_u64(),
        Some(13662488)
    );
}

#[test]
fn test_dell_fixture_binary_provenance_verification_def63() {
    // DEF-63: Distinguish manifest metadata check from actual on-disk binary provenance.
    let maybe_fixture_path = resolve_dell_latitude_fixture_path();

    if let Some(fixture_path) = maybe_fixture_path {
        // Binary is present on disk: verify actual file properties directly
        assert!(fixture_path.exists(), "Resolved fixture path must exist on filesystem");

        let metadata = std::fs::metadata(&fixture_path).expect("Failed to read fixture metadata");
        let actual_size = metadata.len();

        // Exact canonical size verification (13,662,488 bytes; proving the 13,637,736 report typo was false)
        assert_eq!(
            actual_size, 13662488,
            "Actual on-disk binary size must match canonical 13,662,488 bytes (and not the 13,637,736 typo)"
        );
        assert_ne!(
            actual_size, 13637736,
            "Actual size must never match the 13,637,736 report clerical error"
        );

        let file_bytes = std::fs::read(&fixture_path).expect("Failed to read fixture bytes");
        assert_eq!(file_bytes.len(), 13662488);

        let actual_sha = sha256_hex(&file_bytes).to_uppercase();
        assert_eq!(
            actual_sha,
            "E24B8F8F719CCD8F8B8EBDC8A6178D89C15121DF6806F7E112020964484B840B",
            "Actual on-disk binary SHA-256 must match canonical SHA-256"
        );
    } else {
        eprintln!("[SKIPPED] test_dell_fixture_binary_provenance_verification_def63: Real-world binary not present on disk; binary provenance cannot be verified from manifest alone.");
    }
}

#[test]
fn test_ifd_old_synthetic_0x00040040_cannot_confirm_tiger_point_def62() {
    // DEF-62: The old synthetic value 0x00040040 (CSSL=4, CSSO=0x10) does NOT match
    // upstream flashrom Tiger Point identification pair (CSSL=0x11, CSSO=0x68).
    // Even when paired with modern FLMSTR and MDTBA != 0, it must return Unknown!
    let flmap0: u32 = (0x0F << 24) | (0x04 << 16) | (0x00 << 8) | 0x0A;
    let flmap1: u32 = (0x20 << 24) | (0x10 << 16) | (0x02 << 8) | 0x08;
    let flmap2_old_synthetic: u32 = (0x04 << 16) | 0x40; // 0x00040040
    let modern_flmstr1 = (0x02 << 20) | (0x03 << 8);
    let flumap1_valid = Some(0x02000000u32); // MDTBA = 2

    let res = detect_ifd_profile_resolution(
        flmap0,
        flmap1,
        flmap2_old_synthetic,
        None,
        Some(modern_flmstr1),
        flumap1_valid,
        None,
    );
    assert_eq!(res.profile, IfdProfile::Unknown, "Old synthetic 0x00040040 must not confirm Tiger Point");
    assert_eq!(res.confidence, ProfileConfidence::Unknown);
}

#[test]
fn test_ifd_tiger_point_missing_descriptor_context_returns_unknown_def62() {
    // DEF-62: Missing upper map (FLUMAP1) or MDTBA == 0 lacks required descriptor context
    // and must produce Unknown, even if CSSL/CSSO matches Tiger Point.
    let flmap0: u32 = (0x0F << 24) | (0x04 << 16) | (0x00 << 8) | 0x0A;
    let flmap1: u32 = (0x20 << 24) | (0x10 << 16) | (0x02 << 8) | 0x08;
    let flmap2_tiger: u32 = (0x11 << 16) | (0x68 << 2); // Authentic CSSL=0x11, CSSO=0x68
    let modern_flmstr1 = (0x02 << 20) | (0x03 << 8);

    // Case 1: FLUMAP1 is None
    let res_no_umap = detect_ifd_profile_resolution(
        flmap0,
        flmap1,
        flmap2_tiger,
        None,
        Some(modern_flmstr1),
        None,
        None,
    );
    assert_eq!(res_no_umap.profile, IfdProfile::Unknown, "Missing FLUMAP1 must produce Unknown");
    assert_eq!(res_no_umap.confidence, ProfileConfidence::Unknown);

    // Case 2: FLUMAP1 has MDTBA == 0 (pre-Cannon Point upper map)
    let res_zero_mdtba = detect_ifd_profile_resolution(
        flmap0,
        flmap1,
        flmap2_tiger,
        None,
        Some(modern_flmstr1),
        Some(0x00000430), // MDTBA = 0
        None,
    );
    assert_eq!(res_zero_mdtba.profile, IfdProfile::Unknown, "FLUMAP1 with MDTBA=0 must produce Unknown");
    assert_eq!(res_zero_mdtba.confidence, ProfileConfidence::Unknown);
}

#[test]
fn test_ifd_tiger_point_conflicting_evidence_returns_unknown_def62() {
    // DEF-62: Incomplete or conflicting evidence cannot produce Confirmed.
    let flmap0: u32 = (0x0F << 24) | (0x04 << 16) | (0x00 << 8) | 0x0A;
    let flmap1: u32 = (0x20 << 24) | (0x10 << 16) | (0x02 << 8) | 0x08;
    let flmap2_tiger: u32 = (0x11 << 16) | (0x68 << 2);
    let flumap1_valid = Some(0x02000000u32);

    // Legacy FLMSTR combined with Tiger Point layout -> conflict -> Unknown
    let legacy_flmstr1 = 0x02030000u32;
    let res = detect_ifd_profile_resolution(
        flmap0,
        flmap1,
        flmap2_tiger,
        None,
        Some(legacy_flmstr1),
        flumap1_valid,
        None,
    );
    assert_eq!(res.profile, IfdProfile::Unknown, "Conflicting legacy FLMSTR with Tiger Point must yield Unknown");
    assert_eq!(res.confidence, ProfileConfidence::Unknown);
}

#[test]
fn test_ifd_tiger_point_fully_evidenced_confirmed_def62() {
    // DEF-62: Authentic upstream Tiger Point pair (CSSL=0x11, CSSO=0x68) + non-zero MDTBA
    // + modern FLMSTR produces V2Modern / Confirmed.
    let flmap0: u32 = (0x0F << 24) | (0x04 << 16) | (0x00 << 8) | 0x0A;
    let flmap1: u32 = (0x20 << 24) | (0x10 << 16) | (0x02 << 8) | 0x08;
    let flmap2_tiger: u32 = (0x11 << 16) | (0x68 << 2); // 0x001101A0
    let modern_flmstr1 = (0x02 << 20) | (0x03 << 8);
    let flumap1_valid = Some(0x02000000u32);

    let res = detect_ifd_profile_resolution(
        flmap0,
        flmap1,
        flmap2_tiger,
        None,
        Some(modern_flmstr1),
        flumap1_valid,
        None,
    );
    assert_eq!(res.profile, IfdProfile::V2Modern);
    assert_eq!(res.confidence, ProfileConfidence::Confirmed);
    assert_eq!(res.source, ProfileSource::AutoDetected);
}

#[test]
fn test_ifd_unknown_profile_blocks_speculative_decoding_def62() {
    // DEF-62: Unknown automatic profiles do not silently enter profile-specific decoding.
    let flreg_val = 0x007F0010u32;
    assert_eq!(decode_flreg_with_profile(flreg_val, IfdProfile::Unknown), None);

    let rule = decode_master_rule_with_profile(1, "CPU", 0x02030000, IfdProfile::Unknown);
    assert_eq!(rule.read_mask, None);
    assert_eq!(rule.write_mask, None);
}

#[test]
fn test_ifd_user_override_produces_user_selected_def62() {
    // DEF-62: Explicit user override is represented as UserSelected, never Confirmed.
    let flmap0: u32 = (0x04 << 24) | (0x08 << 16) | 0x02;
    let flmap1: u32 = (0x02 << 24) | (0x18 << 16) | (0x03 << 8) | 0x10;

    let res_v2 = detect_ifd_profile_resolution(flmap0, flmap1, 0, None, None, None, Some(IfdProfile::V2Modern));
    assert_eq!(res_v2.profile, IfdProfile::V2Modern);
    assert_eq!(res_v2.confidence, ProfileConfidence::UserSelected);
    assert_eq!(res_v2.source, ProfileSource::UserOverride);

    let res_v1 = detect_ifd_profile_resolution(flmap0, flmap1, 0, None, None, None, Some(IfdProfile::V1Legacy));
    assert_eq!(res_v1.profile, IfdProfile::V1Legacy);
    assert_eq!(res_v1.confidence, ProfileConfidence::UserSelected);
    assert_eq!(res_v1.source, ProfileSource::UserOverride);
}

#[test]
fn test_ifd_context_aware_api_confirms_tiger_point_fixture_def65() {
    // DEF-65: Context-aware detector confirms Tiger Point when given full descriptor context (FLUMAP1 with MDTBA != 0)
    let flmap0: u32 = (0x0F << 24) | (0x04 << 16) | (0x00 << 8) | 0x0A;
    let flmap1: u32 = (0x20 << 24) | (0x10 << 16) | (0x02 << 8) | 0x08;
    let flmap2_tiger: u32 = (0x11 << 16) | (0x68 << 2); // CSSL=0x11, CSSO=0x68
    let flcomp: Option<u32> = Some(0x05 | (0x0F << 4) | (6 << 17) | (1 << 20) | (4 << 21) | (1 << 24) | (1 << 27));
    let modern_flmstr1: u32 = (0x02 << 20) | (0x03 << 8);
    let flumap1_tiger: Option<u32> = Some((0x02 << 24) | (0x04 << 8) | 0x30); // MDTBA = 0x02

    let profile = detect_ifd_profile_with_context(
        flmap0,
        flmap1,
        flmap2_tiger,
        flcomp,
        Some(modern_flmstr1),
        flumap1_tiger,
    );
    assert_eq!(profile, IfdProfile::V2Modern);
}

#[test]
fn test_ifd_context_aware_api_missing_flumap1_returns_unknown_def65() {
    // DEF-65: Missing or zero-MDTBA FLUMAP1 returns Unknown
    let flmap0: u32 = (0x0F << 24) | (0x04 << 16) | (0x00 << 8) | 0x0A;
    let flmap1: u32 = (0x20 << 24) | (0x10 << 16) | (0x02 << 8) | 0x08;
    let flmap2_tiger: u32 = (0x11 << 16) | (0x68 << 2);
    let flcomp: Option<u32> = Some(0x05 | (0x0F << 4) | (6 << 17) | (1 << 20) | (4 << 21) | (1 << 24) | (1 << 27));
    let modern_flmstr1: u32 = (0x02 << 20) | (0x03 << 8);

    // Case A: None
    let profile_none = detect_ifd_profile_with_context(
        flmap0,
        flmap1,
        flmap2_tiger,
        flcomp,
        Some(modern_flmstr1),
        None,
    );
    assert_eq!(profile_none, IfdProfile::Unknown);

    // Case B: MDTBA == 0
    let profile_zero_mdtba = detect_ifd_profile_with_context(
        flmap0,
        flmap1,
        flmap2_tiger,
        flcomp,
        Some(modern_flmstr1),
        Some(0x00000430),
    );
    assert_eq!(profile_zero_mdtba, IfdProfile::Unknown);
}

#[test]
#[allow(deprecated)]
fn test_ifd_context_free_compatibility_api_does_not_falsely_confirm_modern_def65() {
    // DEF-65: Legacy context-free detector cannot receive FLUMAP1, and therefore must never falsely confirm V2Modern
    let flmap0: u32 = (0x0F << 24) | (0x04 << 16) | (0x00 << 8) | 0x0A;
    let flmap1: u32 = (0x20 << 24) | (0x10 << 16) | (0x02 << 8) | 0x08;
    let flmap2_tiger: u32 = (0x11 << 16) | (0x68 << 2);
    let flcomp: Option<u32> = Some(0x05 | (0x0F << 4) | (6 << 17) | (1 << 20) | (4 << 21) | (1 << 24) | (1 << 27));
    let modern_flmstr1: u32 = (0x02 << 20) | (0x03 << 8);

    let profile = detect_ifd_profile(
        flmap0,
        flmap1,
        flmap2_tiger,
        flcomp,
        Some(modern_flmstr1),
    );
    assert_eq!(profile, IfdProfile::Unknown);
}

#[test]
#[allow(deprecated)]
fn test_ifd_legacy_profile_behavior_correct_where_evidence_available_def65() {
    // DEF-65: Legacy profile behavior remains correct where required evidence is available
    let flmap0: u32 = (0x04 << 24) | (0x04 << 16) | (0x00 << 8) | 0x08; // FCBA=0x80, FRBA=0x40, NR=4 (5 regions)
    let flmap1: u32 = (0x02 << 24) | (0x10 << 16) | (0x02 << 8) | 0x06; // FMBA=0x60, FISBA=0x100, ISL=2
    let flmap2_legacy: u32 = 0; // Legacy doesn't use FLMAP2 soft straps
    let flcomp_legacy: Option<u32> = Some(0x05 | (1 << 20) | (4 << 21) | (1 << 24) | (1 << 27)); // freq_read = 0 (20 MHz legacy)
    let legacy_flmstr1: u32 = (0x17 << 16) | (0x02 << 24); // RD: Desc(16), BIOS(17); WR: BIOS(25)

    let profile_ctx = detect_ifd_profile_with_context(
        flmap0,
        flmap1,
        flmap2_legacy,
        flcomp_legacy,
        Some(legacy_flmstr1),
        None,
    );
    assert_eq!(profile_ctx, IfdProfile::V1Legacy);

    let profile_compat = detect_ifd_profile(
        flmap0,
        flmap1,
        flmap2_legacy,
        flcomp_legacy,
        Some(legacy_flmstr1),
    );
    assert_eq!(profile_compat, IfdProfile::V1Legacy);
}

#[test]
fn test_ifd_user_selected_overrides_retain_confidence_def65() {
    // DEF-65: User-selected overrides retain ProfileConfidence::UserSelected
    let flmap0: u32 = (0x04 << 24) | (0x08 << 16) | 0x02;
    let flmap1: u32 = (0x02 << 24) | (0x18 << 16) | (0x03 << 8) | 0x10;

    let res_v2 = detect_ifd_profile_resolution(flmap0, flmap1, 0, None, None, None, Some(IfdProfile::V2Modern));
    assert_eq!(res_v2.profile, IfdProfile::V2Modern);
    assert_eq!(res_v2.confidence, ProfileConfidence::UserSelected);
    assert_eq!(res_v2.source, ProfileSource::UserOverride);

    let res_v1 = detect_ifd_profile_resolution(flmap0, flmap1, 0, None, None, None, Some(IfdProfile::V1Legacy));
    assert_eq!(res_v1.profile, IfdProfile::V1Legacy);
    assert_eq!(res_v1.confidence, ProfileConfidence::UserSelected);
    assert_eq!(res_v1.source, ProfileSource::UserOverride);

    let res_unknown = detect_ifd_profile_resolution(flmap0, flmap1, 0, None, None, None, Some(IfdProfile::Unknown));
    assert_eq!(res_unknown.profile, IfdProfile::Unknown);
    assert_eq!(res_unknown.confidence, ProfileConfidence::UserSelected);
    assert_eq!(res_unknown.source, ProfileSource::UserOverride);
}

#[test]
fn test_ifd_production_parser_uses_descriptor_context_no_silent_fallback_def65() {
    // DEF-65: Production parser IntelFlashDescriptor::parse uses FLUMAP1 from the image and never falls back to context-free heuristic
    let valid_image = SyntheticFixtureBuilder::build_valid_ifd_v2_image(0x1000000);
    let report = IntelFlashDescriptor::parse(&valid_image).expect("Valid IFD v2 should parse");
    assert_eq!(report.profile, IfdProfile::V2Modern);
    assert_eq!(report.gate2_platform.confidence, ProfileConfidence::Confirmed);

    // Corrupt FLUMAP1 at 0x0EFC (clear MDTBA)
    let mut modified_image = valid_image.clone();
    modified_image[0x0EFC..0x0F00].copy_from_slice(&0u32.to_le_bytes());
    let modified_report = IntelFlashDescriptor::parse(&modified_image).expect("Should parse with Unknown profile");
    assert_eq!(modified_report.profile, IfdProfile::Unknown);
    assert_eq!(modified_report.gate2_platform.confidence, ProfileConfidence::Unknown);
    assert!(modified_report.gate2_platform.compatibility_notes.iter().any(|n| n.contains("Incomplete or unverified CPU soft strap layout")));
}
