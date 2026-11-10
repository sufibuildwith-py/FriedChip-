use serde::{Deserialize, Serialize};
use crate::engine::binary::reader::BinaryReader;
use crate::engine::binary::region::{validate_region_boundaries, BoundaryConflict, FlashRegion, RegionType};
use crate::error::AppError;

pub const IFD_SIGNATURE_OFFSET: usize = 0x10;
pub const IFD_SIGNATURE_MAGIC: u32 = 0x0FF0A55A;

/// Explicit Intel Flash Descriptor Profiles (DEF-15, DEF-27).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IfdProfile {
    /// IFD v1.0 (Legacy: ICH8 through Wildcat Point / Broadwell, 5-series through 9-series, Bay Trail)
    V1Legacy,
    /// IFD v1.5 (Provisional: Intel Wellsburg C610 enterprise server PCH; unverified against authentic dump)
    V1_5Intermediate,
    /// IFD v2.0 (Modern: Skylake / Sunrise Point 100-series through 700-series, Meteor Lake)
    V2Modern,
    /// Unknown / Unclassified profile (cannot be safely verified from available markers)
    Unknown,
}

/// Confidence level of the profile resolution (DEF-25.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfileConfidence {
    Confirmed,
    Probable,
    Unknown,
}

/// Provenance of the assigned profile (DEF-25.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfileSource {
    AutoDetected,
    UserOverride,
}

/// Detailed resolution metadata for profile assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileResolution {
    pub profile: IfdProfile,
    pub confidence: ProfileConfidence,
    pub source: ProfileSource,
    pub max_regions: usize,
}

/// SPI Clock Frequency representation supporting explicit reserved/repurposed encodings (DEF-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpiFrequency {
    Mhz(u32),
    Reserved(u8),
    RepurposedEspi(u8),
}

impl SpiFrequency {
    pub fn mhz_or_zero(&self) -> u32 {
        match self {
            SpiFrequency::Mhz(mhz) => *mhz,
            _ => 0,
        }
    }
}

/// Decodes raw 3-bit SPI frequency fields from FLCOMP into `SpiFrequency` based on descriptor profile.
pub fn decode_spi_frequency(bits: u8, profile: IfdProfile) -> SpiFrequency {
    let val = bits & 0x07;
    match profile {
        IfdProfile::V1Legacy | IfdProfile::V1_5Intermediate => match val {
            0 => SpiFrequency::Mhz(20),
            1 => SpiFrequency::Mhz(33),
            2 => SpiFrequency::Mhz(48),
            4 => SpiFrequency::Mhz(50),
            _ => SpiFrequency::Reserved(val),
        },
        IfdProfile::V2Modern => match val {
            0 => SpiFrequency::Mhz(20),
            1 => SpiFrequency::Mhz(33),
            2 => SpiFrequency::Mhz(48),
            4 => SpiFrequency::Mhz(30),
            6 => SpiFrequency::Mhz(17),
            _ => SpiFrequency::Reserved(val),
        },
        IfdProfile::Unknown => {
            // UNKNOWN profile must never claim a known frequency (DEF-26)
            SpiFrequency::Reserved(val)
        }
    }
}

/// Legacy helper for raw 3-bit SPI frequency field decoding.
pub fn decode_spi_frequency_mhz(bits: u8) -> u32 {
    decode_spi_frequency(bits, IfdProfile::V1Legacy).mhz_or_zero()
}

/// Derives the physical bounds on the region table from descriptor section geometry (DEF-25.3, coreboot max_regions_from_fdbar).
pub fn max_regions_from_geometry(fcba: usize, fmba: usize, frba: usize, fpsba: usize) -> usize {
    const FLUMAP1_OFFSET: usize = 0x0EFC; // 4096 - 260
    const MAX_REGIONS: usize = 16;

    let mut sorted = [fcba, fmba, frba, fpsba, FLUMAP1_OFFSET];
    sorted.sort_unstable();

    for i in 0..sorted.len().saturating_sub(1) {
        if sorted[i] == frba {
            let next = sorted[i + 1];
            if next > frba {
                let avail = next - frba;
                return (avail / 4).min(MAX_REGIONS);
            }
        }
    }
    0
}

/// Profile-aware FLREG register decoding (DEF-16, DEF-26).
///
/// Returns `(base_address, limit_address, is_enabled)`.
/// Inclusive limit semantics: limit address includes 0xFFF offset.
/// All address calculations use checked/saturating arithmetic.
/// If profile is `Unknown`, returns `(0, 0, false)` to avoid false positive address mapping.
pub fn decode_flreg_with_profile(val: u32, profile: IfdProfile) -> (usize, usize, bool) {
    let (base_mask, limit_mask) = match profile {
        IfdProfile::V1Legacy | IfdProfile::V1_5Intermediate => (0x0FFF, 0x0FFF),
        IfdProfile::V2Modern => (0x7FFF, 0x7FFF),
        IfdProfile::Unknown => {
            // UNKNOWN profile must not assume 12-bit or 15-bit address shifts (DEF-26)
            return (0, 0, false);
        }
    };

    let base_raw = (val & base_mask) as usize;
    let limit_raw = ((val >> 16) & limit_mask) as usize;

    let base = base_raw.checked_shl(12).unwrap_or(0);
    let limit = limit_raw.checked_shl(12).map(|l| l | 0xFFF).unwrap_or(0);

    // A region is disabled if limit_raw < base_raw
    let is_disabled = limit_raw < base_raw;
    (base, limit, !is_disabled)
}

/// Decodes an Intel Flash Descriptor FLREG value into `(base, limit)`.
pub fn decode_flreg(val: u32) -> (usize, usize) {
    let base_page = (val & 0x7FFF) as usize;
    let limit_page = ((val >> 16) & 0x7FFF) as usize;

    let base = base_page << 12;
    let limit = (limit_page << 12) | 0xFFF;

    (base, limit)
}

/// Structural validation gate results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate1StructuralValidation {
    pub passed: bool,
    pub has_valid_signature: bool,
    pub signature_offset: usize,
    pub signature_magic: String,
    pub region_table_in_bounds: bool,
    pub boundary_conflicts: Vec<BoundaryConflict>,
    pub issues: Vec<String>,
}

/// Platform compatibility assessment results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate2PlatformCompatibility {
    pub passed: bool,
    pub platform_family: String,
    pub descriptor_version: String,
    pub profile: IfdProfile,
    pub confidence: ProfileConfidence,
    pub profile_source: ProfileSource,
    pub region_count: usize,
    pub enabled_regions: Vec<String>,
    pub compatibility_notes: Vec<String>,
}

/// Programmer compatibility assessment states (DEF-19, DEF-29).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProgrammerCompatibilityStatus {
    Compatible,
    Incompatible,
    Indeterminate,
}

/// Runtime chipset lock status representation (DEF-18, DEF-28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimeChipsetLockStatus {
    Locked,
    Unlocked,
    Indeterminate,
}

/// Programmer compatibility assessment results (DEF-28, DEF-29).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate3ProgrammerCompatibility {
    pub status: ProgrammerCompatibilityStatus,
    pub passed: bool,
    #[serde(alias = "detected_chip_size_bytes")]
    pub descriptor_inferred_flash_capacity_bytes: usize,
    #[serde(alias = "detected_chip_size_mb")]
    pub descriptor_inferred_flash_capacity_mb: f64,
    pub host_descriptor_bios_write_allowed: bool,
    pub host_descriptor_write_disabled: bool,
    pub runtime_chipset_lock_status: RuntimeChipsetLockStatus,
    pub hardware_evidence_present: bool,
    pub required_voltage_volts: Option<f64>,
    pub verification_constraints: Vec<String>,
}

/// Decoded SPI flash component density information from FCBA.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashComponentDensity {
    pub component_index: usize,
    pub raw_bits: u8,
    pub density_bytes: usize,
    pub density_mb: f64,
}

/// Component Section (FCBA) decoded details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashComponentsInfo {
    pub fcba_offset: usize,
    pub flcomp_raw: u32,
    pub component_count: usize,
    pub components: Vec<FlashComponentDensity>,
    pub read_clock: SpiFrequency,
    pub read_clock_frequency_mhz: u32,
    pub fast_read_supported: bool,
    pub fast_read_clock: SpiFrequency,
    pub fast_read_clock_frequency_mhz: u32,
    pub write_erase_clock: SpiFrequency,
    pub write_erase_clock_frequency_mhz: u32,
    pub read_id_status_clock: SpiFrequency,
    pub read_id_status_clock_frequency_mhz: u32,
}

/// Decoded master access permissions for a specific execution unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterAccessRule {
    pub master_id: usize,
    pub master_name: String,
    pub flmstr_raw: u32,
    pub requester_id: Option<u16>,
    pub read_mask: u16,
    pub write_mask: u16,
    pub descriptor_readable: bool,
    pub descriptor_writable: bool,
    pub bios_readable: bool,
    pub bios_writable: bool,
    pub me_readable: bool,
    pub me_writable: bool,
    pub gbe_readable: bool,
    pub gbe_writable: bool,
    pub platform_data_readable: bool,
    pub platform_data_writable: bool,
}

/// Flash Master Access Section (FMBA) decoded permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashMasterPermissions {
    pub fmba_offset: usize,
    pub master_count: usize,
    pub descriptor_version: String,
    pub profile: IfdProfile,
    pub masters: Vec<MasterAccessRule>,
    pub host_cpu: MasterAccessRule,
    pub intel_me: MasterAccessRule,
    pub gbe: MasterAccessRule,
    pub host_bios_writable: bool,
    pub host_me_writable: bool,
    pub host_descriptor_writable: bool,
    pub flashing_constraints: Vec<String>,
}

/// Full analysis report generated for an Intel Flash Descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfdAnalysisReport {
    pub image_size: usize,
    pub profile: IfdProfile,
    pub regions: Vec<FlashRegion>,
    pub components: Option<FlashComponentsInfo>,
    pub master_permissions: Option<FlashMasterPermissions>,
    pub gate1_structural: Gate1StructuralValidation,
    pub gate2_platform: Gate2PlatformCompatibility,
    pub gate3_programmer: Gate3ProgrammerCompatibility,
}

/// Decodes the 32-bit FLCOMP register at FCBA into `FlashComponentsInfo` with profile awareness (DEF-17, DEF-26).
pub fn decode_flcomp_with_profile(
    fcba_offset: usize,
    flcomp: u32,
    component_count: usize,
    profile: IfdProfile,
) -> FlashComponentsInfo {
    let mut comp_densities = Vec::new();

    match profile {
        IfdProfile::V1Legacy => {
            // IFD v1: 3-bit density fields (bits 2:0 for Comp 1, bits 5:3 for Comp 2)
            let raw1 = (flcomp & 0x07) as u8;
            let size1 = if raw1 <= 5 { (512 * 1024) << raw1 } else { 0 };
            comp_densities.push(FlashComponentDensity {
                component_index: 1,
                raw_bits: raw1,
                density_bytes: size1,
                density_mb: size1 as f64 / (1024.0 * 1024.0),
            });

            if component_count >= 2 {
                let raw2 = ((flcomp >> 3) & 0x07) as u8;
                let size2 = if raw2 <= 5 { (512 * 1024) << raw2 } else { 0 };
                comp_densities.push(FlashComponentDensity {
                    component_index: 2,
                    raw_bits: raw2,
                    density_bytes: size2,
                    density_mb: size2 as f64 / (1024.0 * 1024.0),
                });
            }
        }
        IfdProfile::V1_5Intermediate | IfdProfile::V2Modern => {
            // IFD v2 / Wellsburg: 4-bit density fields (bits 3:0 for Comp 1, bits 7:4 for Comp 2)
            let raw1 = (flcomp & 0x0F) as u8;
            let size1 = if raw1 <= 7 { (512 * 1024) << raw1 } else { 0 };
            comp_densities.push(FlashComponentDensity {
                component_index: 1,
                raw_bits: raw1,
                density_bytes: size1,
                density_mb: size1 as f64 / (1024.0 * 1024.0),
            });

            if component_count >= 2 {
                let raw2 = ((flcomp >> 4) & 0x0F) as u8;
                let size2 = if raw2 <= 7 { (512 * 1024) << raw2 } else { 0 };
                comp_densities.push(FlashComponentDensity {
                    component_index: 2,
                    raw_bits: raw2,
                    density_bytes: size2,
                    density_mb: size2 as f64 / (1024.0 * 1024.0),
                });
            }
        }
        IfdProfile::Unknown => {
            // UNKNOWN profile preserves raw register without claiming decoded MB density (DEF-26)
            for i in 1..=component_count.min(2) {
                comp_densities.push(FlashComponentDensity {
                    component_index: i,
                    raw_bits: 0,
                    density_bytes: 0,
                    density_mb: 0.0,
                });
            }
        }
    }

    let freq_read_bits = ((flcomp >> 17) & 0x07) as u8;
    let read_clock = decode_spi_frequency(freq_read_bits, profile);
    let read_clock_frequency_mhz = read_clock.mhz_or_zero();

    let fast_read_supported = ((flcomp >> 20) & 0x01) == 1;

    let freq_fastread_bits = ((flcomp >> 21) & 0x07) as u8;
    let fast_read_clock = decode_spi_frequency(freq_fastread_bits, profile);
    let fast_read_clock_frequency_mhz = fast_read_clock.mhz_or_zero();

    let freq_write_bits = ((flcomp >> 24) & 0x07) as u8;
    let write_erase_clock = decode_spi_frequency(freq_write_bits, profile);
    let write_erase_clock_frequency_mhz = write_erase_clock.mhz_or_zero();

    let freq_read_id_bits = ((flcomp >> 27) & 0x07) as u8;
    let read_id_status_clock = decode_spi_frequency(freq_read_id_bits, profile);
    let read_id_status_clock_frequency_mhz = read_id_status_clock.mhz_or_zero();

    FlashComponentsInfo {
        fcba_offset,
        flcomp_raw: flcomp,
        component_count,
        components: comp_densities,
        read_clock,
        read_clock_frequency_mhz,
        fast_read_supported,
        fast_read_clock,
        fast_read_clock_frequency_mhz,
        write_erase_clock,
        write_erase_clock_frequency_mhz,
        read_id_status_clock,
        read_id_status_clock_frequency_mhz,
    }
}

/// Backwards-compatible decode_flcomp using V1Legacy default.
pub fn decode_flcomp(fcba_offset: usize, flcomp: u32, component_count: usize) -> FlashComponentsInfo {
    decode_flcomp_with_profile(fcba_offset, flcomp, component_count, IfdProfile::V1Legacy)
}

/// Decodes an individual 32-bit FLMSTR register into a `MasterAccessRule` with profile awareness (DEF-18, DEF-26).
pub fn decode_master_rule_with_profile(
    master_id: usize,
    master_name: &str,
    flmstr: u32,
    profile: IfdProfile,
) -> MasterAccessRule {
    let (requester_id, read_mask, write_mask) = match profile {
        IfdProfile::V1Legacy | IfdProfile::V1_5Intermediate => {
            // IFD v1 / Wellsburg (Legacy ICH8..Wildcat Point / Broadwell, 5-series..9-series)
            // Bits 15:0 = Requester ID
            // Bits 23:16 = Read access mask (regions 0..7)
            // Bits 31:24 = Write access mask (regions 0..7)
            let req_id = (flmstr & 0xFFFF) as u16;
            let r_mask = ((flmstr >> 16) & 0xFF) as u16;
            let w_mask = ((flmstr >> 24) & 0xFF) as u16;
            (Some(req_id), r_mask, w_mask)
        }
        IfdProfile::V2Modern => {
            // IFD v2 (Modern Skylake / Sunrise Point 100-series through 700-series)
            // Bits 19:8 = Read access mask (regions 0..11)
            // Bits 31:20 = Write access mask (regions 0..11)
            let mut r_mask = ((flmstr >> 8) & 0xFFF) as u16;
            let mut w_mask = ((flmstr >> 20) & 0xFFF) as u16;
            // Extended regions 12..15 in Cannon Lake+
            r_mask |= ((flmstr & 0x0F) as u16) << 12;
            w_mask |= (((flmstr >> 4) & 0x0F) as u16) << 12;
            (None, r_mask, w_mask)
        }
        IfdProfile::Unknown => {
            // UNKNOWN profile preserves raw register without assigning permissions (DEF-26)
            (None, 0, 0)
        }
    };

    MasterAccessRule {
        master_id,
        master_name: master_name.to_string(),
        flmstr_raw: flmstr,
        requester_id,
        read_mask,
        write_mask,
        descriptor_readable: (read_mask & (1 << 0)) != 0,
        descriptor_writable: (write_mask & (1 << 0)) != 0,
        bios_readable: (read_mask & (1 << 1)) != 0,
        bios_writable: (write_mask & (1 << 1)) != 0,
        me_readable: (read_mask & (1 << 2)) != 0,
        me_writable: (write_mask & (1 << 2)) != 0,
        gbe_readable: (read_mask & (1 << 3)) != 0,
        gbe_writable: (write_mask & (1 << 3)) != 0,
        platform_data_readable: (read_mask & (1 << 4)) != 0,
        platform_data_writable: (write_mask & (1 << 4)) != 0,
    }
}

/// Backwards-compatible decode_master_rule wrapper.
pub fn decode_master_rule(
    master_id: usize,
    master_name: &str,
    flmstr: u32,
    is_v2: bool,
) -> MasterAccessRule {
    let profile = if is_v2 { IfdProfile::V2Modern } else { IfdProfile::V1Legacy };
    decode_master_rule_with_profile(master_id, master_name, flmstr, profile)
}

/// Evidence-backed profile resolution based on verified descriptor characteristics (DEF-25).
pub fn detect_ifd_profile_resolution(
    flmap0: u32,
    flmap1: u32,
    _flmap2: u32,
    flcomp: Option<u32>,
    flmstr1: Option<u32>,
    profile_override: Option<IfdProfile>,
) -> ProfileResolution {
    let fcba = ((flmap0 & 0xFF) as usize) << 4;
    let frba = (((flmap0 >> 16) & 0xFF) as usize) << 4;
    // FLMAP0 NR field is strictly bits 26:24 (3 bits, DEF-25.2). Bit 27 is reserved!
    let nr_bits = ((flmap0 >> 24) & 0x07) as usize;
    let legacy_nr_count = nr_bits.saturating_add(1);

    let fmba = ((flmap1 & 0xFF) as usize) << 4;
    let fpsba = (((flmap1 >> 16) & 0xFF) as usize) << 4;
    let isl = ((flmap1 >> 24) & 0xFF) as usize;

    let geom_regions = max_regions_from_geometry(fcba, fmba, frba, fpsba);

    // If caller provided an explicit override, honor it with Confirmed confidence and UserOverride provenance
    if let Some(override_profile) = profile_override {
        let max_regions = match override_profile {
            IfdProfile::V1Legacy => std::cmp::min(geom_regions.max(1), 5),
            IfdProfile::V1_5Intermediate => std::cmp::min(geom_regions.max(1), 8),
            IfdProfile::V2Modern => std::cmp::min(geom_regions.max(1), 16),
            IfdProfile::Unknown => geom_regions.min(16),
        };
        return ProfileResolution {
            profile: override_profile,
            confidence: if override_profile == IfdProfile::Unknown {
                ProfileConfidence::Unknown
            } else {
                ProfileConfidence::Confirmed
            },
            source: ProfileSource::UserOverride,
            max_regions,
        };
    }

    if geom_regions == 0 {
        return ProfileResolution {
            profile: IfdProfile::Unknown,
            confidence: ProfileConfidence::Unknown,
            source: ProfileSource::AutoDetected,
            max_regions: 0,
        };
    }

    // Inspect FLMSTR1 shift evidence if available
    let (flmstr1_legacy_evidence, flmstr1_modern_evidence) = if let Some(m1) = flmstr1 {
        let v1_rd = (m1 >> 16) & 0xFF;
        let v1_wr = (m1 >> 24) & 0xFF;
        let v2_rd = (m1 >> 8) & 0xFFF;
        let v2_wr = (m1 >> 20) & 0xFFF;

        // Legacy has Write in 31:24, Read in 23:16, Requester ID in 15:0
        let legacy_match = (v1_rd != 0 || v1_wr != 0) && (m1 & 0x0000FF00) == 0;
        // Modern has Write in 31:20, Read in 19:8, Requester ID dropped (bits 7:0 are 0)
        let modern_match = (v2_rd != 0 || v2_wr != 0) && (m1 & 0xFF) == 0;
        (legacy_match, modern_match)
    } else {
        (false, false)
    };

    // Inspect read frequency: 17 MHz (value 6) is unique to modern Skylake+ (DEF-25)
    let freq_read_17mhz = if let Some(comp) = flcomp {
        ((comp >> 17) & 0x07) == 6
    } else {
        false
    };

    // Modern candidate (IFD v2): geometry fits >= 10 regions, extended straps, or modern master/frequency
    if geom_regions >= 10 {
        let mut modern_indicators = 0usize;
        if freq_read_17mhz { modern_indicators += 1; }
        if isl >= 0x30 { modern_indicators += 1; }
        if flmstr1_modern_evidence { modern_indicators += 1; }

        if modern_indicators >= 2 || freq_read_17mhz {
            return ProfileResolution {
                profile: IfdProfile::V2Modern,
                confidence: ProfileConfidence::Confirmed,
                source: ProfileSource::AutoDetected,
                max_regions: geom_regions.min(16),
            };
        } else if isl >= 0x20 || flmstr1_modern_evidence {
            return ProfileResolution {
                profile: IfdProfile::V2Modern,
                confidence: ProfileConfidence::Probable,
                source: ProfileSource::AutoDetected,
                max_regions: geom_regions.min(16),
            };
        }
    }

    // Legacy candidate (IFD v1): geometry fits <= 8 regions, NR in 26:24 is <= 4, compact straps <= 25
    if geom_regions <= 8 && nr_bits <= 4 && isl <= 25 {
        let mut legacy_indicators = 0usize;
        if isl <= 21 { legacy_indicators += 1; }
        if flmstr1_legacy_evidence { legacy_indicators += 1; }

        let max_reg = std::cmp::min(geom_regions, legacy_nr_count);
        if legacy_indicators >= 1 {
            return ProfileResolution {
                profile: IfdProfile::V1Legacy,
                confidence: ProfileConfidence::Confirmed,
                source: ProfileSource::AutoDetected,
                max_regions: max_reg,
            };
        } else {
            return ProfileResolution {
                profile: IfdProfile::V1Legacy,
                confidence: ProfileConfidence::Probable,
                source: ProfileSource::AutoDetected,
                max_regions: max_reg,
            };
        }
    }

    // Ambiguous evidence -> Unknown profile with Unknown confidence
    ProfileResolution {
        profile: IfdProfile::Unknown,
        confidence: ProfileConfidence::Unknown,
        source: ProfileSource::AutoDetected,
        max_regions: geom_regions.min(16),
    }
}

/// Backwards-compatible profile detector returning just `IfdProfile`.
pub fn detect_ifd_profile(
    flmap0: u32,
    flmap1: u32,
    flmap2: u32,
    flcomp: Option<u32>,
    flmstr1: Option<u32>,
) -> IfdProfile {
    detect_ifd_profile_resolution(flmap0, flmap1, flmap2, flcomp, flmstr1, None).profile
}

pub struct IntelFlashDescriptor;

impl IntelFlashDescriptor {
    /// Inspect a binary dump for an Intel Flash Descriptor using automatic profile detection.
    pub fn parse(image_bytes: &[u8]) -> Result<IfdAnalysisReport, AppError> {
        Self::parse_with_profile(image_bytes, None)
    }

    /// Inspect a binary dump for an Intel Flash Descriptor with an explicit profile override (DEF-15).
    pub fn parse_with_profile(
        image_bytes: &[u8],
        profile_override: Option<IfdProfile>,
    ) -> Result<IfdAnalysisReport, AppError> {
        let reader = BinaryReader::new(image_bytes);
        let image_size = reader.len();

        if image_size < 0x1000 {
            return Err(AppError::Binary(format!(
                "Image size (0x{:X}) is too small to contain a valid Intel Flash Descriptor (minimum 4KB)",
                image_size
            )));
        }

        // 1. Signature check at offset 0x10
        let sig = reader.read_u32_le(IFD_SIGNATURE_OFFSET)?;
        let has_valid_sig = sig == IFD_SIGNATURE_MAGIC;

        if !has_valid_sig {
            return Ok(IfdAnalysisReport {
                image_size,
                profile: IfdProfile::Unknown,
                regions: Vec::new(),
                components: None,
                master_permissions: None,
                gate1_structural: Gate1StructuralValidation {
                    passed: false,
                    has_valid_signature: false,
                    signature_offset: IFD_SIGNATURE_OFFSET,
                    signature_magic: format!("0x{:08X}", sig),
                    region_table_in_bounds: false,
                    boundary_conflicts: Vec::new(),
                    issues: vec![format!(
                        "Invalid IFD signature at offset 0x10: expected 0x{:08X}, found 0x{:08X}",
                        IFD_SIGNATURE_MAGIC, sig
                    )],
                },
                gate2_platform: Gate2PlatformCompatibility {
                    passed: false,
                    platform_family: "UNCLASSIFIED".to_string(),
                    descriptor_version: "Unknown".to_string(),
                    profile: IfdProfile::Unknown,
                    confidence: ProfileConfidence::Unknown,
                    profile_source: ProfileSource::AutoDetected,
                    region_count: 0,
                    enabled_regions: Vec::new(),
                    compatibility_notes: vec!["Cannot evaluate platform without valid IFD".to_string()],
                },
                gate3_programmer: Gate3ProgrammerCompatibility {
                    status: ProgrammerCompatibilityStatus::Indeterminate,
                    passed: false,
                    descriptor_inferred_flash_capacity_bytes: image_size,
                    descriptor_inferred_flash_capacity_mb: image_size as f64 / (1024.0 * 1024.0),
                    host_descriptor_write_disabled: true,
                    host_descriptor_bios_write_allowed: false,
                    runtime_chipset_lock_status: RuntimeChipsetLockStatus::Indeterminate,
                    hardware_evidence_present: false,
                    required_voltage_volts: None,
                    verification_constraints: vec!["Image is not recognized as a valid Intel SPI image".to_string()],
                },
            });
        }

        // 2. Read FLMAP0 at offset 0x14
        let flmap0 = reader.read_u32_le(0x14)?;
        let fcba_field = (flmap0 & 0xFF) as usize;
        let fcba = fcba_field << 4;
        let nc = (((flmap0 >> 8) & 0x03) as u8).saturating_add(1);
        let frba_field = ((flmap0 >> 16) & 0xFF) as usize;
        let frba = frba_field << 4;
        // FLMAP0 NR field is strictly bits 26:24 (3 bits, DEF-25.2). Bit 27 is reserved!
        let _nr_field = ((flmap0 >> 24) & 0x07) as u8;

        // 3. Read FLMAP1 at offset 0x18
        let flmap1 = reader.read_u32_le(0x18)?;
        let fmba_field = (flmap1 & 0xFF) as usize;
        let fmba = fmba_field << 4;
        let nm_field = ((flmap1 >> 8) & 0x07) as u8;
        let nm = nm_field.saturating_add(1);

        // 4. Read FLMAP2 at offset 0x1C
        let flmap2 = reader.read_u32_le(0x1C).unwrap_or(0);

        // Read raw FLCOMP and FLMSTR1 for profile resolution
        let raw_flcomp = if fcba + 4 <= image_size {
            reader.read_u32_le(fcba).ok()
        } else {
            None
        };

        let raw_flmstr1 = if fmba + 4 <= image_size {
            reader.read_u32_le(fmba).ok()
        } else {
            None
        };

        // 5. Resolve Profile (DEF-15, DEF-25)
        let resolution = detect_ifd_profile_resolution(flmap0, flmap1, flmap2, raw_flcomp, raw_flmstr1, profile_override);
        let profile = resolution.profile;
        let profile_confidence = resolution.confidence;
        let profile_source = resolution.source;

        let mut issues = Vec::new();

        // 6. Decode FCBA (Flash Components Base Address)
        let mut components_info = None;
        if fcba + 4 > image_size {
            issues.push(format!(
                "Flash Component Base Address (0x{:X}) extends beyond image size (0x{:X})",
                fcba + 4, image_size
            ));
        } else if let Some(flcomp) = raw_flcomp {
            components_info = Some(decode_flcomp_with_profile(fcba, flcomp, nc as usize, profile));
        }

        // 7. Decode FMBA (Flash Master Base Address)
        let mut master_permissions = None;
        let mut host_descriptor_bios_write_allowed = false;

        let master_count = (nm as usize).clamp(1, 4);
        if fmba + (master_count * 4) > image_size {
            issues.push(format!(
                "Flash Master Base Address (0x{:X}) extends beyond image size (0x{:X})",
                fmba + (master_count * 4), image_size
            ));
        } else {
            let mut masters = Vec::new();

            for m_idx in 0..master_count {
                let reg_offset = fmba + (m_idx * 4);
                if let Ok(flmstr) = reader.read_u32_le(reg_offset) {
                    let master_name = match m_idx {
                        0 => "Host CPU / BIOS",
                        1 => "Intel Management Engine",
                        2 => "Gigabit Ethernet",
                        3 => "Embedded Controller",
                        _ => "Unknown Master",
                    };
                    masters.push(decode_master_rule_with_profile(m_idx + 1, master_name, flmstr, profile));
                }
            }

            let host_cpu = masters.get(0).cloned().unwrap_or_else(|| {
                decode_master_rule_with_profile(1, "Host CPU / BIOS", 0, profile)
            });
            let intel_me = masters.get(1).cloned().unwrap_or_else(|| {
                decode_master_rule_with_profile(2, "Intel Management Engine", 0, profile)
            });
            let gbe = masters.get(2).cloned().unwrap_or_else(|| {
                decode_master_rule_with_profile(3, "Gigabit Ethernet", 0, profile)
            });

            host_descriptor_bios_write_allowed = host_cpu.bios_writable;

            let mut flashing_constraints = Vec::new();
            flashing_constraints.push("Flash Master permissions (FLMSTR1) govern in-system host controller requests only.".to_string());
            flashing_constraints.push("Descriptor permissions do NOT establish complete hardware write-protection.".to_string());
            flashing_constraints.push("Complete hardware protection requires interrogation of PCH Protected Range registers (PR0..PR4), BILD lock (BIOS Interface Lock Down), SMM TCO lock, and physical /WP pin state.".to_string());

            let desc_version_str = match profile {
                IfdProfile::V1Legacy => "IFD v1.0 (Legacy ICH8..Wildcat Point / Broadwell)".to_string(),
                IfdProfile::V1_5Intermediate => "IFD v1.5 (Provisional / Wellsburg C610)".to_string(),
                IfdProfile::V2Modern => "IFD v2.0 (Modern Skylake 100-series..Raptor/Meteor Lake)".to_string(),
                IfdProfile::Unknown => "IFD Unknown / Unclassified Layout".to_string(),
            };

            master_permissions = Some(FlashMasterPermissions {
                fmba_offset: fmba,
                master_count,
                descriptor_version: desc_version_str,
                profile,
                masters,
                host_cpu: host_cpu.clone(),
                intel_me: intel_me.clone(),
                gbe: gbe.clone(),
                host_bios_writable: host_cpu.bios_writable,
                host_me_writable: host_cpu.me_writable,
                host_descriptor_writable: host_cpu.descriptor_writable,
                flashing_constraints,
            });
        }

        // 8. Parse Flash Regions at FRBA (Profile-aware and geometry-bounded DEF-16, DEF-25)
        let nr_bits = ((flmap0 >> 24) & 0x07) as usize;
        let legacy_nr_count = nr_bits.saturating_add(1);
        let effective_regions = if resolution.max_regions > 0 {
            resolution.max_regions
        } else if resolution.profile == IfdProfile::V1Legacy || resolution.profile == IfdProfile::Unknown {
            legacy_nr_count
        } else {
            1
        };

        let max_regions = resolution.max_regions;
        let mut regions = Vec::new();
        let mut region_table_in_bounds = true;

        if resolution.max_regions == 0 || frba + (effective_regions * 4) > image_size {
            region_table_in_bounds = false;
            issues.push(format!(
                "Flash Region Base Address (0x{:X}) extends beyond image size (0x{:X})",
                frba + (effective_regions * 4),
                image_size
            ));
        } else {
            for id in 0..resolution.max_regions {
                let reg_offset = frba + (id * 4);
                if let Ok(reg_val) = reader.read_u32_le(reg_offset) {
                    let (base, limit, is_enabled) = decode_flreg_with_profile(reg_val, profile);
                    let mut region = FlashRegion::from_raw_bounds(id as u8, base, limit);
                    region.enabled = is_enabled;

                    // Compute SHA-256 for enabled regions completely within bounds
                    if region.enabled && region.limit < image_size {
                        if let Ok(slice) = reader.slice(region.base, region.size) {
                            region.sha256 = Some(crate::engine::binary::checksums::sha256_hex(slice));
                        }
                    }

                    regions.push(region);
                }
            }
        }

        // 9. Run boundary validation
        let (boundary_conflicts, boundary_warnings) = validate_region_boundaries(&regions, image_size);
        issues.extend(boundary_warnings);

        let structural_passed = has_valid_sig && region_table_in_bounds && boundary_conflicts.is_empty();

        let enabled_regions: Vec<String> = regions
            .iter()
            .filter(|r| r.enabled)
            .map(|r| format!("{} (0x{:X}..0x{:X})", r.name, r.base, r.limit))
            .collect();

        // 10. Platform classification
        let (platform_family, descriptor_version) = match profile {
            IfdProfile::V1Legacy => (
                "Intel Legacy PCH (ICH8 through 9-Series / Bay Trail)".to_string(),
                "IFD v1.0 (Legacy Layout)".to_string(),
            ),
            IfdProfile::V1_5Intermediate => (
                "Intel Wellsburg / C610 Enterprise PCH (Provisional)".to_string(),
                "IFD v1.5 (Provisional 8-Region Layout)".to_string(),
            ),
            IfdProfile::V2Modern => (
                "Intel Modern PCH (100-Series through 700-Series / Meteor Lake)".to_string(),
                "IFD v2.0 (Modern Geometry-Bounded Layout)".to_string(),
            ),
            IfdProfile::Unknown => (
                "Intel PCH (Unclassified / Unknown Profile)".to_string(),
                "IFD Unknown / Unclassified".to_string(),
            ),
        };

        let mut compat_notes = Vec::new();
        compat_notes.push(format!("Detected Flash Components: {}", nc));
        compat_notes.push(format!("Detected Flash Masters: {}", nm));
        compat_notes.push(format!("Assigned Profile: {:?}", profile));
        compat_notes.push(format!("Profile Confidence: {:?}", profile_confidence));
        compat_notes.push(format!("Profile Source: {:?}", profile_source));
        compat_notes.push(format!("Geometry Bounds Max Regions: {}", max_regions));
        if regions.iter().any(|r| r.region_type == RegionType::Me && r.enabled) {
            compat_notes.push("Intel Management Engine region present".to_string());
        }
        if regions.iter().any(|r| r.region_type == RegionType::Bios && r.enabled) {
            compat_notes.push("BIOS/UEFI primary payload region present".to_string());
        }

        let platform_passed = structural_passed && profile != IfdProfile::Unknown && profile_confidence != ProfileConfidence::Unknown;

        // 11. Programmer Compatibility Gate (DEF-19, DEF-28, DEF-29)
        let fcba_total_capacity: usize = components_info
            .as_ref()
            .map(|c| c.components.iter().map(|d| d.density_bytes).sum())
            .unwrap_or(0);

        let mut highest_limit = 0usize;
        for r in regions.iter().filter(|r| r.enabled) {
            if r.limit > highest_limit {
                highest_limit = r.limit;
            }
        }

        let detected_capacity = if fcba_total_capacity > 0 {
            fcba_total_capacity
        } else if highest_limit > 0 {
            highest_limit.saturating_add(1).next_power_of_two()
        } else {
            image_size
        };

        let mut programmer_constraints = Vec::new();
        programmer_constraints.push(format!(
            "Descriptor-inferred SPI flash capacity estimate: {:.2} MB (0x{:X} bytes)",
            detected_capacity as f64 / (1024.0 * 1024.0),
            detected_capacity
        ));
        programmer_constraints.push("External programmer compatibility cannot be positively asserted from binary descriptor analysis alone.".to_string());
        programmer_constraints.push("Requires physical hardware interrogation: flash chip package pinout, JEDEC ID (RDID), operating voltage (1.8V vs 3.3V), and in-circuit hold-off/back-powering characteristics.".to_string());
        programmer_constraints.push("Host-based access permissions are distinct from external SPI programmer compatibility; external programmers bypass PCH master permission masks.".to_string());
        programmer_constraints.push("Descriptor master permissions do not guarantee host or in-circuit writeability: hardware Protected Range registers (PR0..PR4) and BIOS_CNTL lock bits enforce independent hardware protections.".to_string());

        // Gate 3 status remains strictly Indeterminate without physical hardware telemetry (DEF-29)
        let programmer_status = ProgrammerCompatibilityStatus::Indeterminate;

        Ok(IfdAnalysisReport {
            image_size,
            profile,
            regions,
            components: components_info,
            master_permissions,
            gate1_structural: Gate1StructuralValidation {
                passed: structural_passed,
                has_valid_signature: true,
                signature_offset: IFD_SIGNATURE_OFFSET,
                signature_magic: format!("0x{:08X}", sig),
                region_table_in_bounds,
                boundary_conflicts,
                issues,
            },
            gate2_platform: Gate2PlatformCompatibility {
                passed: platform_passed,
                platform_family,
                descriptor_version,
                profile,
                confidence: profile_confidence,
                profile_source,
                region_count: max_regions,
                enabled_regions,
                compatibility_notes: compat_notes,
            },
            gate3_programmer: Gate3ProgrammerCompatibility {
                status: programmer_status,
                passed: false, // Refuse speculative positive compatibility without hardware
                descriptor_inferred_flash_capacity_bytes: detected_capacity,
                descriptor_inferred_flash_capacity_mb: detected_capacity as f64 / (1024.0 * 1024.0),
                host_descriptor_bios_write_allowed,
                host_descriptor_write_disabled: !host_descriptor_bios_write_allowed,
                runtime_chipset_lock_status: RuntimeChipsetLockStatus::Indeterminate,
                hardware_evidence_present: false,
                required_voltage_volts: None,
                verification_constraints: programmer_constraints,
            },
        })
    }
}
