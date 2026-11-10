use serde::{Deserialize, Serialize};

use crate::engine::binary::reader::BinaryReader;
use crate::engine::binary::region::{validate_region_boundaries, BoundaryConflict, FlashRegion, RegionType};
use crate::error::AppError;

pub const IFD_SIGNATURE_OFFSET: usize = 0x10;
pub const IFD_SIGNATURE_MAGIC: u32 = 0x0FF0A55A;
pub const FLUMAP1_OFFSET: usize = 0x0EFC; // 3836 bytes (IFD_SIZE - 256 - 4)

/// Explicit Intel Flash Descriptor Profiles (DEF-15, DEF-27, DEF-34, DEF-35).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IfdProfile {
    /// IFD v1.0 (Legacy: ICH8 through Wildcat Point / Broadwell, 5-series through 9-series, Bay Trail)
    V1Legacy,
    /// IFD v1.5 (Quarantined / Unresolved server profile; unsupported pending authentic hardware dump DEF-35)
    V1_5Intermediate,
    /// IFD v2.0 (Modern: Skylake / Sunrise Point 100-series through 700-series, Meteor Lake)
    V2Modern,
    /// Unknown descriptor generation (safe zero-decoding, no speculative inferences DEF-36)
    Unknown,
}

/// Confidence level of the profile resolution (DEF-25.4, DEF-34).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfileConfidence {
    Confirmed,
    Probable,
    UserSelected,
    Unknown,
}

/// Provenance of the assigned profile (DEF-25.4, DEF-34).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfileSource {
    AutoDetected,
    UserOverride,
}

/// Detailed resolution metadata for profile assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileResolution {
    pub profile: IfdProfile,
    pub confidence: ProfileConfidence,
    pub source: ProfileSource,
    pub max_regions: usize,
    pub indicators: Vec<String>,
}

/// SPI Clock Frequency representation supporting explicit reserved/repurposed encodings (DEF-17, DEF-36).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpiFrequency {
    Mhz(u32),
    Reserved(u8),
    RepurposedEspi(u8),
}

impl SpiFrequency {
    pub fn mhz(&self) -> Option<u32> {
        match self {
            SpiFrequency::Mhz(mhz) => Some(*mhz),
            _ => None,
        }
    }

    pub fn mhz_or_zero(&self) -> u32 {
        self.mhz().unwrap_or(0)
    }
}

/// Decodes raw 3-bit SPI frequency fields from FLCOMP into `SpiFrequency` based on descriptor profile.
pub fn decode_spi_frequency(bits: u8, profile: IfdProfile) -> SpiFrequency {
    let val = bits & 0x07;
    match profile {
        IfdProfile::V1Legacy => match val {
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
        IfdProfile::V1_5Intermediate | IfdProfile::Unknown => {
            // UNKNOWN and quarantined V1.5 must never claim a known frequency (DEF-26, DEF-35, DEF-36)
            SpiFrequency::Reserved(val)
        }
    }
}

/// Legacy helper for raw 3-bit SPI frequency field decoding.
pub fn decode_spi_frequency_mhz(bits: u8) -> u32 {
    decode_spi_frequency(bits, IfdProfile::V1Legacy).mhz_or_zero()
}

/// Interpretation state for FLMAP2 register (DEF-47).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Flmap2Interpretation {
    /// Integrated Clock Controller Record Initialization (PCH 6-series through Skylake/Cannon Point era).
    /// FLMAP2[23:16] is ICCRIBA, FLMAP2[31:24] is RIL.
    IccribaRil,
    /// Tiger Point (500-series PCH) and later CPU Soft Straps.
    /// FLMAP2[11:2] is CSSO (10 bits), FLMAP2[23:16] is CSSL (8 bits).
    /// Reserved bits [1:0], [15:12], and [31:24] must be zero.
    CpuSoftStrap,
    /// Legacy ICH8-10 / Ibex Peak where bits [31:16] are reserved 0.
    LegacyMchStrap,
    /// Ambiguous, conflicting, or unknown layout.
    Unknown,
}

/// Representation of section extent length, distinguishing exact from unresolved lengths (DEF-49).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExtentLength {
    Exact(usize),
    Unknown,
}

/// Helper representing an active descriptor section interval for checked collision analysis (DEF-43, DEF-49).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionExtent {
    pub name: String,
    pub start: usize,
    pub length: ExtentLength,
    pub end: Option<usize>,
}

/// Validated descriptor geometry structure (DEF-37, DEF-42, DEF-43, DEF-47, DEF-48, DEF-49).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptorGeometry {
    pub raw_flmap0: u32,
    pub raw_flmap1: u32,
    pub raw_flmap2: u32,
    pub fcba: usize,
    pub fcba_raw: u8,
    pub nc: u8,
    pub frba: usize,
    pub frba_raw: u8,
    pub nr_raw: u8,
    pub fmba: usize,
    pub fmba_raw: u8,
    pub nm: u8,
    pub fpsba: Option<usize>,
    pub fpsba_raw: u8,
    pub isl: u8,
    pub fmsba: Option<usize>,
    pub fmsba_raw: u8,
    pub msl: u8,
    pub iccriba: Option<usize>,
    pub iccriba_raw: u8,
    pub ril: u8,
    pub csso_raw: u16,
    pub cssl_raw: u8,
    pub flmap2_interpretation: Flmap2Interpretation,
    pub max_regions_extent: usize,
    pub is_valid: bool,
    pub has_unresolved_extents: bool,
    pub section_extents: Vec<SectionExtent>,
    pub validation_errors: Vec<String>,
    pub validation_notes: Vec<String>,
}

/// Classify the platform-dependent interpretation of FLMAP2 (DEF-47, DEF-52).
pub fn determine_flmap2_interpretation(flmap2: u32) -> Flmap2Interpretation {
    if flmap2 == 0 {
        return Flmap2Interpretation::Unknown;
    }

    let upper_half = (flmap2 >> 16) as u16;
    let fmsba_raw = (flmap2 & 0xFF) as u8;

    let ril_raw = ((flmap2 >> 24) & 0xFF) as u8;
    let iccriba_raw = ((flmap2 >> 16) & 0xFF) as u8;

    let res1 = (flmap2 & 0x03) as u8;
    let csso = ((flmap2 >> 2) & 0x03FF) as u16;
    let res2 = ((flmap2 >> 12) & 0x0F) as u8;
    let cssl = iccriba_raw;

    // 1. Legacy MCH strap definition (ICH8 through Ibex Peak):
    // Upper half (bits [31:16]) strictly reserved 0.
    // Lower half must define a non-zero base address (FMSBA).
    if upper_half == 0 {
        if fmsba_raw != 0 {
            return Flmap2Interpretation::LegacyMchStrap;
        } else {
            return Flmap2Interpretation::Unknown;
        }
    }

    // 2. Tiger Point / 500-series+ CPU soft strap definition (ich_descriptors.h):
    // Bits [31:24] (res3) MUST be strictly zero.
    // Bits [1:0] (res1) and [15:12] (res2) MUST be strictly zero.
    // CSSO (offset) and CSSL (length) must be non-zero.
    if ril_raw == 0 {
        if res1 == 0 && res2 == 0 && csso != 0 && cssl != 0 {
            return Flmap2Interpretation::CpuSoftStrap;
        } else {
            return Flmap2Interpretation::Unknown;
        }
    }

    // 3. ICCRIBA / RIL layout (Cougar Point 6-series through Cannon Point / Skylake):
    // Bits [31:24] is RIL (Register Initialization Length) and must be non-zero (1..16).
    // Bits [23:16] is ICCRIBA (ICC Register Initialization Base Address) and must be non-zero (0x02..0x40).
    if ril_raw >= 1 && ril_raw <= 16 && iccriba_raw >= 0x02 && iccriba_raw <= 0x40 {
        return Flmap2Interpretation::IccribaRil;
    }

    // All other bit patterns are unverified or ambiguous
    Flmap2Interpretation::Unknown
}

/// Validates descriptor geometry before table traversal (DEF-37, DEF-42, DEF-43, DEF-47, DEF-49, DEF-52, DEF-54).
pub fn validate_descriptor_geometry(
    flmap0: u32,
    flmap1: u32,
    flmap2: u32,
    image_size: usize,
) -> DescriptorGeometry {
    let mut errors = Vec::new();
    let mut notes = Vec::new();

    // 1. Decode raw register fields per Intel SPI Programming Guides & flashrom ich_descriptors.h
    let fcba_raw = (flmap0 & 0xFF) as u8;
    let fcba = (fcba_raw as usize) << 4;
    let nc = (((flmap0 >> 8) & 0x03) as u8).saturating_add(1);

    let frba_raw = ((flmap0 >> 16) & 0xFF) as u8;
    let frba = (frba_raw as usize) << 4;
    let nr_raw = ((flmap0 >> 24) & 0x07) as u8;

    let fmba_raw = (flmap1 & 0xFF) as u8;
    let fmba = (fmba_raw as usize) << 4;
    let nm = (((flmap1 >> 8) & 0x07) as u8).saturating_add(1);

    let fpsba_raw = ((flmap1 >> 16) & 0xFF) as u8;
    let fpsba = if fpsba_raw != 0 { Some((fpsba_raw as usize) << 4) } else { None };
    let isl = ((flmap1 >> 24) & 0xFF) as u8;

    let fmsba_raw = (flmap2 & 0xFF) as u8;
    let flmap2_interpretation = determine_flmap2_interpretation(flmap2);

    // DEF-52: Unknown layouts must NEVER expose speculative FMSBA or MSL as validated metadata
    let (fmsba, msl) = match flmap2_interpretation {
        Flmap2Interpretation::LegacyMchStrap | Flmap2Interpretation::IccribaRil => {
            let fmsba = if fmsba_raw != 0 { Some((fmsba_raw as usize) << 4) } else { None };
            let msl = ((flmap2 >> 8) & 0xFF) as u8;
            (fmsba, msl)
        }
        Flmap2Interpretation::CpuSoftStrap | Flmap2Interpretation::Unknown => (None, 0),
    };

    let iccriba_raw = ((flmap2 >> 16) & 0xFF) as u8;
    let ril_raw = ((flmap2 >> 24) & 0xFF) as u8;
    let csso_raw = ((flmap2 >> 2) & 0x03FF) as u16;
    let cssl_raw = iccriba_raw;

    // DEF-52: Unknown or CpuSoftStrap layouts must NEVER expose speculative ICCRIBA or RIL
    let (iccriba, ril) = match flmap2_interpretation {
        Flmap2Interpretation::IccribaRil => {
            let addr = (iccriba_raw as usize) << 4;
            if (0x20..0x1000).contains(&addr) {
                (Some(addr), ril_raw)
            } else {
                errors.push(format!("ICCRIBA (0x{:X}) is out of descriptor bounds [0x20..0x1000]", addr));
                (None, 0)
            }
        }
        _ => (None, 0),
    };

    let mut has_unresolved_extents = false;
    let mut active_sections: Vec<SectionExtent> = Vec::new();

    // 2. Assemble active sections with checked arithmetic extents (DEF-43, DEF-49)
    // Reassess FCBA extent: 12 bytes per flashrom sizeof(struct ich_desc_component) (DEF-49)
    let fcba_len = 12usize;
    // Mandatory FRBA baseline: 4 bytes (region 0 descriptor bounds)
    let frba_len = 4usize;
    // Mandatory FMBA: nm * 4 bytes
    let fmba_len = (nm as usize).checked_mul(4).unwrap_or(4).max(4);

    let mut add_active_section = |name: &str, start: usize, len_opt: Option<usize>, errs: &mut Vec<String>, unres: &mut bool| {
        if start < 0x20 || start >= 0x1000 {
            errs.push(format!("{} (0x{:X}) is out of descriptor bounds [0x20..0x1000]", name, start));
            return;
        }
        match len_opt {
            Some(len) => {
                match start.checked_add(len) {
                    Some(end) => {
                        if end > 0x1000 {
                            errs.push(format!(
                                "{} at 0x{:X} with length {} extends beyond descriptor block boundary 0x1000 (end: 0x{:X})",
                                name, start, len, end
                            ));
                        } else {
                            active_sections.push(SectionExtent {
                                name: name.to_string(),
                                start,
                                length: ExtentLength::Exact(len),
                                end: Some(end),
                            });
                        }
                    }
                    None => {
                        errs.push(format!("{} at 0x{:X} arithmetic overflow calculating extent length {}", name, start, len));
                    }
                }
            }
            None => {
                *unres = true;
                errs.push(format!("{} at 0x{:X} has unresolved length (length field is 0); safe extent cannot be verified", name, start));
                active_sections.push(SectionExtent {
                    name: name.to_string(),
                    start,
                    length: ExtentLength::Unknown,
                    end: None,
                });
            }
        }
    };

    add_active_section("FCBA", fcba, Some(fcba_len), &mut errors, &mut has_unresolved_extents);
    add_active_section("FRBA", frba, Some(frba_len), &mut errors, &mut has_unresolved_extents);
    add_active_section("FMBA", fmba, Some(fmba_len), &mut errors, &mut has_unresolved_extents);

    if let Some(p) = fpsba {
        let len = if isl > 0 { Some((isl as usize) * 4) } else { None };
        add_active_section("FPSBA", p, len, &mut errors, &mut has_unresolved_extents);
    }
    if let Some(m) = fmsba {
        let len = if msl > 0 { Some((msl as usize) * 4) } else { None };
        add_active_section("FMSBA", m, len, &mut errors, &mut has_unresolved_extents);
    }
    if let Some(i) = iccriba {
        let len = if ril > 0 { Some((ril as usize) * 4) } else { None };
        add_active_section("ICCRIBA", i, len, &mut errors, &mut has_unresolved_extents);
    }

    if flmap2 != 0 && flmap2_interpretation == Flmap2Interpretation::Unknown {
        has_unresolved_extents = true;
        errors.push(format!(
            "FLMAP2 layout (0x{:08X}) is unknown or unverified; safe layout-dependent geometry cannot be verified",
            flmap2
        ));
    } else if flmap2 == 0 {
        notes.push("FLMAP2 is all-zero (no strap or ICC sections defined; layout unassigned)".to_string());
    }

    // 3. Pairwise interval collision check between every active section pair (DEF-43, DEF-49)
    let n_active = active_sections.len();
    for i in 0..n_active {
        for j in (i + 1)..n_active {
            let a = &active_sections[i];
            let b = &active_sections[j];
            if a.start == b.start {
                errors.push(format!(
                    "Collision: {} overlaps {} (share identical base address 0x{:X})",
                    a.name, b.name, a.start
                ));
            } else if let (Some(a_end), Some(b_end)) = (a.end, b.end) {
                if a.start < b_end && b.start < a_end {
                    errors.push(format!(
                        "Collision: {} overlaps {} (0x{:X}..0x{:X} overlaps with 0x{:X}..0x{:X})",
                        a.name, b.name, a.start, a_end, b.start, b_end
                    ));
                }
            }
        }
    }

    // 4. Derive maximum safe region table extent bounded by next succeeding section or FLUMAP1 (DEF-42, DEF-43)
    let mut succeeding_offsets = vec![FLUMAP1_OFFSET];

    for s in &active_sections {
        if s.start > frba {
            succeeding_offsets.push(s.start);
        }
    }

    let next_boundary = succeeding_offsets.into_iter().min().unwrap_or(FLUMAP1_OFFSET);
    let available_bytes = next_boundary.saturating_sub(frba);

    let max_regions_extent = if frba >= FLUMAP1_OFFSET || available_bytes < 4 {
        0
    } else {
        (available_bytes / 4).min(16)
    };

    if max_regions_extent == 0 && errors.is_empty() {
        errors.push(format!(
            "FRBA at 0x{:X} has zero available space before next boundary 0x{:X}",
            frba, next_boundary
        ));
    }

    // Document validated sections and conservative assumptions
    notes.push(format!("FCBA validated with 12-byte component header at 0x{:X}", fcba));
    notes.push(format!("FMBA validated with {} masters ({} bytes) at 0x{:X}", nm, fmba_len, fmba));
    if let Some(p) = fpsba { notes.push(format!("FPSBA present at 0x{:X} (ISL={})", p, isl)); }
    if let Some(m) = fmsba { notes.push(format!("FMSBA present at 0x{:X} (MSL={})", m, msl)); }
    if let Some(i) = iccriba { notes.push(format!("ICCRIBA present at 0x{:X} (RIL={})", i, ril)); }
    notes.push(format!("FLMAP2 interpretation: {:?}", flmap2_interpretation));
    notes.push(format!("Safe region extent bounded at {} regions before boundary 0x{:X}", max_regions_extent, next_boundary));

    let is_valid = errors.is_empty() && !has_unresolved_extents && max_regions_extent > 0 && frba < image_size;

    DescriptorGeometry {
        raw_flmap0: flmap0,
        raw_flmap1: flmap1,
        raw_flmap2: flmap2,
        fcba,
        fcba_raw,
        nc,
        frba,
        frba_raw,
        nr_raw,
        fmba,
        fmba_raw,
        nm,
        fpsba,
        fpsba_raw,
        isl,
        fmsba,
        fmsba_raw,
        msl,
        iccriba,
        iccriba_raw,
        ril,
        csso_raw,
        cssl_raw,
        flmap2_interpretation,
        max_regions_extent,
        is_valid,
        has_unresolved_extents,
        section_extents: active_sections,
        validation_errors: errors,
        validation_notes: notes,
    }
}

/// Backwards-compatible helper deriving the physical bounds on region table extent (DEF-25.3, DEF-37).
pub fn max_regions_from_geometry(fcba: usize, fmba: usize, frba: usize, fpsba: usize) -> usize {
    let flmap0 = ((fcba >> 4) & 0xFF) as u32 | ((((frba >> 4) & 0xFF) as u32) << 16);
    let flmap1 = ((fmba >> 4) & 0xFF) as u32 | ((((fpsba >> 4) & 0xFF) as u32) << 16);
    let geom = validate_descriptor_geometry(flmap0, flmap1, 0, 4096);
    geom.max_regions_extent
}

/// Profile-aware FLREG register decoding (DEF-16, DEF-26, DEF-36).
///
/// Returns `Some((base_address, limit_address, is_enabled))` if profile is known.
/// If profile is `Unknown` or quarantined `V1_5Intermediate`, returns `None` to prevent
/// claiming speculative region addresses or enabled states.
pub fn decode_flreg_with_profile(val: u32, profile: IfdProfile) -> Option<(usize, usize, bool)> {
    let (base_mask, limit_mask) = match profile {
        IfdProfile::V1Legacy => (0x0FFF, 0x0FFF),
        IfdProfile::V2Modern => (0x7FFF, 0x7FFF),
        IfdProfile::V1_5Intermediate | IfdProfile::Unknown => {
            // UNKNOWN and quarantined V1.5 must not assume address shifts (DEF-26, DEF-35, DEF-36)
            return None;
        }
    };

    let base_raw = (val & base_mask) as usize;
    let limit_raw = ((val >> 16) & limit_mask) as usize;

    let base = base_raw.checked_shl(12).unwrap_or(0);
    let limit = limit_raw.checked_shl(12).map(|l| l | 0xFFF).unwrap_or(0);

    let is_disabled = limit_raw < base_raw;
    Some((base, limit, !is_disabled))
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

/// Platform compatibility assessment results (DEF-34, DEF-36).
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

/// Programmer compatibility assessment results (DEF-28, DEF-29, DEF-36).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate3ProgrammerCompatibility {
    pub status: ProgrammerCompatibilityStatus,
    pub passed: bool,
    #[serde(alias = "detected_chip_size_bytes")]
    pub descriptor_inferred_flash_capacity_bytes: Option<usize>,
    #[serde(alias = "detected_chip_size_mb")]
    pub descriptor_inferred_flash_capacity_mb: Option<f64>,
    pub host_descriptor_bios_write_allowed: Option<bool>,
    pub host_descriptor_write_disabled: Option<bool>,
    pub runtime_chipset_lock_status: RuntimeChipsetLockStatus,
    pub hardware_evidence_present: bool,
    pub required_voltage_volts: Option<f64>,
    pub verification_constraints: Vec<String>,
}

/// Decoded SPI flash component density information from FCBA (DEF-36).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashComponentDensity {
    pub component_index: usize,
    pub raw_bits: u8,
    pub density_bytes: Option<usize>,
    pub density_mb: Option<f64>,
}

/// Component Section (FCBA) decoded details (DEF-36).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashComponentsInfo {
    pub fcba_offset: usize,
    pub flcomp_raw: u32,
    pub component_count: usize,
    pub components: Vec<FlashComponentDensity>,
    pub read_clock: SpiFrequency,
    pub read_clock_frequency_mhz: Option<u32>,
    pub fast_read_supported: Option<bool>,
    pub fast_read_clock: SpiFrequency,
    pub fast_read_clock_frequency_mhz: Option<u32>,
    pub write_erase_clock: SpiFrequency,
    pub write_erase_clock_frequency_mhz: Option<u32>,
    pub read_id_status_clock: SpiFrequency,
    pub read_id_status_clock_frequency_mhz: Option<u32>,
}

/// Decoded master access permissions for a specific execution unit (DEF-36).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterAccessRule {
    pub master_id: usize,
    pub master_name: String,
    pub flmstr_raw: u32,
    pub requester_id: Option<u16>,
    pub read_mask: Option<u16>,
    pub write_mask: Option<u16>,
    pub descriptor_readable: Option<bool>,
    pub descriptor_writable: Option<bool>,
    pub bios_readable: Option<bool>,
    pub bios_writable: Option<bool>,
    pub me_readable: Option<bool>,
    pub me_writable: Option<bool>,
    pub gbe_readable: Option<bool>,
    pub gbe_writable: Option<bool>,
    pub platform_data_readable: Option<bool>,
    pub platform_data_writable: Option<bool>,
}

/// Flash Master Access Section (FMBA) decoded permissions (DEF-36).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashMasterPermissions {
    pub fmba_offset: usize,
    pub master_count: usize,
    pub descriptor_version: String,
    pub profile: IfdProfile,
    pub masters: Vec<MasterAccessRule>,
    pub host_cpu: Option<MasterAccessRule>,
    pub intel_me: Option<MasterAccessRule>,
    pub gbe: Option<MasterAccessRule>,
    pub host_bios_writable: Option<bool>,
    pub host_me_writable: Option<bool>,
    pub host_descriptor_writable: Option<bool>,
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

/// Decodes the 32-bit FLCOMP register at FCBA into `FlashComponentsInfo` with profile awareness (DEF-17, DEF-26, DEF-36).
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
            let size1 = if raw1 <= 5 { Some((512 * 1024) << raw1) } else { None };
            comp_densities.push(FlashComponentDensity {
                component_index: 1,
                raw_bits: raw1,
                density_bytes: size1,
                density_mb: size1.map(|s| s as f64 / (1024.0 * 1024.0)),
            });

            if component_count >= 2 {
                let raw2 = ((flcomp >> 3) & 0x07) as u8;
                let size2 = if raw2 <= 5 { Some((512 * 1024) << raw2) } else { None };
                comp_densities.push(FlashComponentDensity {
                    component_index: 2,
                    raw_bits: raw2,
                    density_bytes: size2,
                    density_mb: size2.map(|s| s as f64 / (1024.0 * 1024.0)),
                });
            }

            let read_clock = decode_spi_frequency(((flcomp >> 17) & 0x07) as u8, profile);
            let fast_read_clock = decode_spi_frequency(((flcomp >> 21) & 0x07) as u8, profile);
            let write_erase_clock = decode_spi_frequency(((flcomp >> 24) & 0x07) as u8, profile);
            let read_id_status_clock = decode_spi_frequency(((flcomp >> 27) & 0x07) as u8, profile);

            FlashComponentsInfo {
                fcba_offset,
                flcomp_raw: flcomp,
                component_count,
                components: comp_densities,
                read_clock,
                read_clock_frequency_mhz: read_clock.mhz(),
                fast_read_supported: Some(((flcomp >> 20) & 0x01) != 0),
                fast_read_clock,
                fast_read_clock_frequency_mhz: fast_read_clock.mhz(),
                write_erase_clock,
                write_erase_clock_frequency_mhz: write_erase_clock.mhz(),
                read_id_status_clock,
                read_id_status_clock_frequency_mhz: read_id_status_clock.mhz(),
            }
        }
        IfdProfile::V2Modern => {
            // IFD v2: 4-bit density fields (bits 3:0 for Comp 1, bits 7:4 for Comp 2)
            let raw1 = (flcomp & 0x0F) as u8;
            let size1 = if raw1 <= 7 { Some((512 * 1024) << raw1) } else { None };
            comp_densities.push(FlashComponentDensity {
                component_index: 1,
                raw_bits: raw1,
                density_bytes: size1,
                density_mb: size1.map(|s| s as f64 / (1024.0 * 1024.0)),
            });

            if component_count >= 2 {
                let raw2 = ((flcomp >> 4) & 0x0F) as u8;
                let size2 = if raw2 <= 7 { Some((512 * 1024) << raw2) } else { None };
                comp_densities.push(FlashComponentDensity {
                    component_index: 2,
                    raw_bits: raw2,
                    density_bytes: size2,
                    density_mb: size2.map(|s| s as f64 / (1024.0 * 1024.0)),
                });
            }

            let read_clock = decode_spi_frequency(((flcomp >> 17) & 0x07) as u8, profile);
            let fast_read_clock = decode_spi_frequency(((flcomp >> 21) & 0x07) as u8, profile);
            let write_erase_clock = decode_spi_frequency(((flcomp >> 24) & 0x07) as u8, profile);
            let read_id_status_clock = decode_spi_frequency(((flcomp >> 27) & 0x07) as u8, profile);

            FlashComponentsInfo {
                fcba_offset,
                flcomp_raw: flcomp,
                component_count,
                components: comp_densities,
                read_clock,
                read_clock_frequency_mhz: read_clock.mhz(),
                fast_read_supported: Some(((flcomp >> 20) & 0x01) != 0),
                fast_read_clock,
                fast_read_clock_frequency_mhz: fast_read_clock.mhz(),
                write_erase_clock,
                write_erase_clock_frequency_mhz: write_erase_clock.mhz(),
                read_id_status_clock,
                read_id_status_clock_frequency_mhz: read_id_status_clock.mhz(),
            }
        }
        IfdProfile::V1_5Intermediate | IfdProfile::Unknown => {
            // UNKNOWN and quarantined V1.5: Withhold speculative interpretation (DEF-26, DEF-35, DEF-36)
            comp_densities.push(FlashComponentDensity {
                component_index: 1,
                raw_bits: (flcomp & 0x0F) as u8,
                density_bytes: None,
                density_mb: None,
            });
            if component_count >= 2 {
                comp_densities.push(FlashComponentDensity {
                    component_index: 2,
                    raw_bits: ((flcomp >> 4) & 0x0F) as u8,
                    density_bytes: None,
                    density_mb: None,
                });
            }

            FlashComponentsInfo {
                fcba_offset,
                flcomp_raw: flcomp,
                component_count,
                components: comp_densities,
                read_clock: SpiFrequency::Reserved(((flcomp >> 17) & 0x07) as u8),
                read_clock_frequency_mhz: None,
                fast_read_supported: None,
                fast_read_clock: SpiFrequency::Reserved(((flcomp >> 21) & 0x07) as u8),
                fast_read_clock_frequency_mhz: None,
                write_erase_clock: SpiFrequency::Reserved(((flcomp >> 24) & 0x07) as u8),
                write_erase_clock_frequency_mhz: None,
                read_id_status_clock: SpiFrequency::Reserved(((flcomp >> 27) & 0x07) as u8),
                read_id_status_clock_frequency_mhz: None,
            }
        }
    }
}

/// Backwards-compatible decode_flcomp wrapper.
pub fn decode_flcomp(fcba_offset: usize, flcomp: u32, component_count: usize) -> FlashComponentsInfo {
    decode_flcomp_with_profile(fcba_offset, flcomp, component_count, IfdProfile::V1Legacy)
}

/// Decodes an individual 32-bit FLMSTR register into a `MasterAccessRule` with profile awareness (DEF-18, DEF-26, DEF-36).
pub fn decode_master_rule_with_profile(
    master_id: usize,
    master_name: &str,
    flmstr: u32,
    profile: IfdProfile,
) -> MasterAccessRule {
    match profile {
        IfdProfile::V1Legacy => {
            let req_id = (flmstr & 0xFFFF) as u16;
            let rd_mask = ((flmstr >> 16) & 0xFF) as u16;
            let wr_mask = ((flmstr >> 24) & 0xFF) as u16;

            MasterAccessRule {
                master_id,
                master_name: master_name.to_string(),
                flmstr_raw: flmstr,
                requester_id: Some(req_id),
                read_mask: Some(rd_mask),
                write_mask: Some(wr_mask),
                descriptor_readable: Some((rd_mask & 0x01) != 0),
                descriptor_writable: Some((wr_mask & 0x01) != 0),
                bios_readable: Some((rd_mask & 0x02) != 0),
                bios_writable: Some((wr_mask & 0x02) != 0),
                me_readable: Some((rd_mask & 0x04) != 0),
                me_writable: Some((wr_mask & 0x04) != 0),
                gbe_readable: Some((rd_mask & 0x08) != 0),
                gbe_writable: Some((wr_mask & 0x08) != 0),
                platform_data_readable: Some((rd_mask & 0x10) != 0),
                platform_data_writable: Some((wr_mask & 0x10) != 0),
            }
        }
        IfdProfile::V2Modern => {
            let rd_mask = ((flmstr >> 8) & 0x0FFF) as u16;
            let wr_mask = ((flmstr >> 20) & 0x0FFF) as u16;

            MasterAccessRule {
                master_id,
                master_name: master_name.to_string(),
                flmstr_raw: flmstr,
                requester_id: None,
                read_mask: Some(rd_mask),
                write_mask: Some(wr_mask),
                descriptor_readable: Some((rd_mask & 0x001) != 0),
                descriptor_writable: Some((wr_mask & 0x001) != 0),
                bios_readable: Some((rd_mask & 0x002) != 0),
                bios_writable: Some((wr_mask & 0x002) != 0),
                me_readable: Some((rd_mask & 0x004) != 0),
                me_writable: Some((wr_mask & 0x004) != 0),
                gbe_readable: Some((rd_mask & 0x008) != 0),
                gbe_writable: Some((wr_mask & 0x008) != 0),
                platform_data_readable: Some((rd_mask & 0x010) != 0),
                platform_data_writable: Some((wr_mask & 0x010) != 0),
            }
        }
        IfdProfile::V1_5Intermediate | IfdProfile::Unknown => {
            // UNKNOWN and quarantined V1.5: Withhold speculative interpretation (DEF-26, DEF-35, DEF-36)
            MasterAccessRule {
                master_id,
                master_name: master_name.to_string(),
                flmstr_raw: flmstr,
                requester_id: None,
                read_mask: None,
                write_mask: None,
                descriptor_readable: None,
                descriptor_writable: None,
                bios_readable: None,
                bios_writable: None,
                me_readable: None,
                me_writable: None,
                gbe_readable: None,
                gbe_writable: None,
                platform_data_readable: None,
                platform_data_writable: None,
            }
        }
    }
}

/// Backwards-compatible decode_master_rule wrapper.
pub fn decode_master_rule(
    master_id: usize,
    master_name: &str,
    flmstr: u32,
    profile: IfdProfile,
) -> MasterAccessRule {
    decode_master_rule_with_profile(master_id, master_name, flmstr, profile)
}

/// Conservative, evidence-backed Intel Flash Descriptor profile detector (DEF-15, DEF-25, DEF-34, DEF-35, DEF-57, DEF-62).
pub fn detect_ifd_profile_resolution(
    flmap0: u32,
    flmap1: u32,
    flmap2: u32,
    flcomp: Option<u32>,
    flmstr1: Option<u32>,
    flumap1: Option<u32>,
    profile_override: Option<IfdProfile>,
) -> ProfileResolution {
    let mut indicators = Vec::new();

    // 1. Validate geometry independently from generation (DEF-34, DEF-37)
    let geometry = validate_descriptor_geometry(flmap0, flmap1, flmap2, 4096);
    let max_regions = geometry.max_regions_extent;

    // 2. Caller explicit profile override
    if let Some(override_profile) = profile_override {
        match override_profile {
            IfdProfile::V1_5Intermediate => {
                // DEF-35: V1.5 is quarantined and unverified
                indicators.push("Explicit user override requested V1.5, which is quarantined/unsupported pending authentic fixture verification".to_string());
                return ProfileResolution {
                    profile: IfdProfile::V1_5Intermediate,
                    confidence: ProfileConfidence::Unknown,
                    source: ProfileSource::UserOverride,
                    max_regions: max_regions.min(8),
                    indicators,
                };
            }
            IfdProfile::V1Legacy => {
                let nr_count = (geometry.nr_raw as usize).saturating_add(1);
                indicators.push("Explicit user override: V1Legacy".to_string());
                return ProfileResolution {
                    profile: IfdProfile::V1Legacy,
                    confidence: ProfileConfidence::UserSelected, // DEF-46: never Confirmed without independent evidence
                    source: ProfileSource::UserOverride,
                    max_regions: std::cmp::min(max_regions.max(1), nr_count.clamp(1, 5)),
                    indicators,
                };
            }
            IfdProfile::V2Modern => {
                indicators.push("Explicit user override: V2Modern".to_string());
                return ProfileResolution {
                    profile: IfdProfile::V2Modern,
                    confidence: ProfileConfidence::UserSelected, // DEF-46: never Confirmed without independent evidence
                    source: ProfileSource::UserOverride,
                    max_regions: max_regions.min(16),
                    indicators,
                };
            }
            IfdProfile::Unknown => {
                indicators.push("Explicit user override: Unknown".to_string());
                return ProfileResolution {
                    profile: IfdProfile::Unknown,
                    confidence: ProfileConfidence::UserSelected, // DEF-46
                    source: ProfileSource::UserOverride,
                    max_regions,
                    indicators,
                };
            }
        }
    }

    if !geometry.is_valid {
        indicators.push(format!("Descriptor geometry is malformed: {:?}", geometry.validation_errors));
        return ProfileResolution {
            profile: IfdProfile::Unknown,
            confidence: ProfileConfidence::Unknown,
            source: ProfileSource::AutoDetected,
            max_regions: 0,
            indicators,
        };
    }

    // 3. Inspect binary indicators for IFD v2 (Modern) vs IFD v1 (Legacy) (DEF-41)
    // Upstream flashrom & coreboot evidence:
    // - freq_read == 6 (17 MHz) was reserved on pre-Skylake and used on Skylake 100-series,
    //   but is repurposed on 300-series Cannon Point and newer. It is NOT standalone V2 proof.
    // - NR field in FLMAP0[26:24]: In legacy, 0..4 defines 1..5 regions. In modern, NR is dropped/reserved (often 0).
    //   A zero field independently proves neither generation.
    let freq_read = flcomp.map(|comp| ((comp >> 17) & 0x07) as u8);
    let is_17mhz_read = freq_read == Some(6);
    let is_legacy_freq = match freq_read {
        Some(0) | Some(1) | Some(2) | Some(4) => true,
        _ => false,
    };

    let nr_bits = geometry.nr_raw;
    let is_valid_legacy_nr = nr_bits <= 4; // 0..4 indicates 1..5 regions

    // FLMSTR1 format evidence:
    // In legacy FLMSTR1:
    //   - Host CPU reads Descriptor at bit 16 and BIOS at bit 17
    //   - Bits [15:0] is Requester ID (e.g. 0x0000, 0x0002)
    //   - Bits [23:21] and [31:29] are strictly reserved zero (0xE0E00000)
    // In modern FLMSTR1:
    //   - Host CPU reads Descriptor at bit 8 and BIOS at bit 9
    //   - Bits [15:0] is the 16-bit read mask (not Requester ID)
    let (is_modern_flmstr, is_legacy_flmstr) = if let Some(m1) = flmstr1 {
        let modern_rd_desc = (m1 & (1 << 8)) != 0;
        let modern_rd_bios = (m1 & (1 << 9)) != 0;
        let modern_match = modern_rd_desc && modern_rd_bios;

        let legacy_rd_desc = (m1 & (1 << 16)) != 0;
        let legacy_rd_bios = (m1 & (1 << 17)) != 0;
        let legacy_reserved_zero = (m1 & 0xE0E00000) == 0;
        let legacy_match = legacy_rd_desc && legacy_rd_bios && legacy_reserved_zero;

        (modern_match, legacy_match)
    } else {
        (false, false)
    };

    // Modern profile criteria (Tiger Point / 500-series+) (DEF-41, DEF-53, DEF-57, DEF-62):
    // Requires verified modern FLMSTR layout AND authentic Tiger Point identification pair
    // (CSSL == 0x11, CSSO == 0x68 per upstream flashrom ich_descriptors.c) AND verified FLUMAP1
    // descriptor context with non-zero MDTBA (MIP Descr. Table Base Addr., required by flashrom
    // guess_ich_chipset_from_content to enter modern 300/500/600 series classification)
    // AND must NOT match legacy layout.
    let is_tiger_strap_format = matches!(geometry.flmap2_interpretation, Flmap2Interpretation::CpuSoftStrap);
    let is_upstream_tiger_point_pair = geometry.cssl_raw == 0x11 && geometry.csso_raw == 0x68;
    let mdtba = flumap1.map(|u| ((u >> 24) & 0xFF) as u8).unwrap_or(0);
    let has_valid_mdtba = mdtba != 0;

    if is_modern_flmstr && !is_legacy_flmstr && is_tiger_strap_format && is_upstream_tiger_point_pair && has_valid_mdtba {
        indicators.push("Detected modern Tiger Point indicators: authentic CPU soft strap pair (CSSL=0x11, CSSO=0x68), non-zero FLUMAP1 MDTBA, and modern FLMSTR".to_string());
        return ProfileResolution {
            profile: IfdProfile::V2Modern,
            confidence: ProfileConfidence::Confirmed,
            source: ProfileSource::AutoDetected,
            max_regions: max_regions.min(16),
            indicators,
        };
    }

    if is_tiger_strap_format {
        indicators.push(format!(
            "Incomplete or unverified CPU soft strap layout (CSSL=0x{:02X}, CSSO=0x{:03X}, MDTBA=0x{:02X}): does not match upstream Tiger Point identification pair (CSSL=0x11, CSSO=0x68) with non-zero MDTBA; assigned Unknown (DEF-62)",
            geometry.cssl_raw, geometry.csso_raw, mdtba
        ));
        return ProfileResolution {
            profile: IfdProfile::Unknown,
            confidence: ProfileConfidence::Unknown,
            source: ProfileSource::AutoDetected,
            max_regions,
            indicators,
        };
    }

    if is_modern_flmstr && is_17mhz_read && !is_legacy_flmstr {
        indicators.push("Ambiguous modern indicators: 17 MHz read clock with modern FLMSTR layout; frequency 6 is repurposed on Cannon Point+ and insufficient for automatic V2 selection; assigned Unknown pending user profile override (DEF-57)".to_string());
        return ProfileResolution {
            profile: IfdProfile::Unknown,
            confidence: ProfileConfidence::Unknown,
            source: ProfileSource::AutoDetected,
            max_regions,
            indicators,
        };
    }

    // Legacy profile criteria (ICH8 through 9-Series):
    // Requires verified legacy FLMSTR layout AND valid legacy NR (0..4) AND legacy frequency
    // AND must NOT match modern layout.
    if is_legacy_flmstr && is_valid_legacy_nr && is_legacy_freq && !is_17mhz_read && !is_modern_flmstr {
        let legacy_count = (nr_bits as usize).saturating_add(1);
        indicators.push("Detected legacy ICH8..9-Series indicators: valid legacy NR, legacy SPI frequency, legacy FLMSTR layout".to_string());
        return ProfileResolution {
            profile: IfdProfile::V1Legacy,
            confidence: ProfileConfidence::Confirmed,
            source: ProfileSource::AutoDetected,
            max_regions: std::cmp::min(max_regions, legacy_count),
            indicators,
        };
    }

    // Ambiguous evidence -> UNKNOWN (DEF-34, DEF-41: "Where binary evidence cannot establish a unique layout, return UNKNOWN")
    indicators.push("Binary evidence is ambiguous, conflicting, or unverified; speculative profile assignment withheld for safety".to_string());
    ProfileResolution {
        profile: IfdProfile::Unknown,
        confidence: ProfileConfidence::Unknown,
        source: ProfileSource::AutoDetected,
        max_regions,
        indicators,
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
    detect_ifd_profile_resolution(flmap0, flmap1, flmap2, flcomp, flmstr1, None, None).profile
}

pub struct IntelFlashDescriptor;

impl IntelFlashDescriptor {
    /// Inspect a binary dump for an Intel Flash Descriptor using automatic profile detection.
    pub fn parse(image_bytes: &[u8]) -> Result<IfdAnalysisReport, AppError> {
        Self::parse_with_profile(image_bytes, None)
    }

    /// Inspect a binary dump for an Intel Flash Descriptor with an explicit profile override (DEF-15, DEF-34).
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
        #[cfg(test)]
        crate::engine::testing::observer::record_ifd_sig(has_valid_sig);

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
                    descriptor_inferred_flash_capacity_bytes: None,
                    descriptor_inferred_flash_capacity_mb: None,
                    host_descriptor_write_disabled: None,
                    host_descriptor_bios_write_allowed: None,
                    runtime_chipset_lock_status: RuntimeChipsetLockStatus::Indeterminate,
                    hardware_evidence_present: false,
                    required_voltage_volts: None,
                    verification_constraints: vec!["Image is not recognized as a valid Intel SPI image".to_string()],
                },
            });
        }

        // 2. Read FLMAP0 at offset 0x14
        let flmap0 = reader.read_u32_le(0x14)?;

        // 3. Read FLMAP1 at offset 0x18
        let flmap1 = reader.read_u32_le(0x18)?;

        // 4. Read FLMAP2 at offset 0x1C (Explicit read without silent fallback DEF-38)
        let flmap2 = reader.read_u32_le(0x1C)?;

        // 5. Pre-traversal Geometry Validation (DEF-37)
        let geometry = validate_descriptor_geometry(flmap0, flmap1, flmap2, image_size);
        #[cfg(test)]
        crate::engine::testing::observer::record_ifd_geom(geometry.is_valid);
        let mut issues = geometry.validation_errors.clone();

        let fcba = geometry.fcba;
        let nc = geometry.nc;
        let frba = geometry.frba;
        let fmba = geometry.fmba;
        let nm = geometry.nm;

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

        // Read raw FLUMAP1 at offset 0x0EFC (3836 bytes) if within image bounds (DEF-62)
        const FLUMAP1_OFFSET: usize = 0x0EFC;
        let raw_flumap1 = if FLUMAP1_OFFSET + 4 <= image_size {
            reader.read_u32_le(FLUMAP1_OFFSET).ok()
        } else {
            None
        };

        // 6. Resolve Profile (DEF-15, DEF-25, DEF-34, DEF-35, DEF-62)
        let resolution = detect_ifd_profile_resolution(
            flmap0,
            flmap1,
            flmap2,
            raw_flcomp,
            raw_flmstr1,
            raw_flumap1,
            profile_override,
        );
        #[cfg(test)]
        crate::engine::testing::observer::record_ifd_profile_decoder();
        let profile = resolution.profile;
        let profile_confidence = resolution.confidence;
        let profile_source = resolution.source;
        let max_regions = resolution.max_regions;

        // 7. Decode FCBA (Flash Components Base Address)
        let mut components_info = None;
        if fcba + 4 > image_size {
            issues.push(format!(
                "Flash Component Base Address (0x{:X}) extends beyond image size (0x{:X})",
                fcba + 4, image_size
            ));
        } else if let Some(flcomp) = raw_flcomp {
            components_info = Some(decode_flcomp_with_profile(fcba, flcomp, nc as usize, profile));
        }

        // 8. Decode FMBA (Flash Master Base Address)
        let mut master_permissions = None;
        let mut host_descriptor_bios_write_allowed = None;

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

            let host_cpu = masters.get(0).cloned();
            let intel_me = masters.get(1).cloned();
            let gbe = masters.get(2).cloned();

            host_descriptor_bios_write_allowed = host_cpu.as_ref().and_then(|h| h.bios_writable);

            let mut flashing_constraints = Vec::new();
            flashing_constraints.push("Flash Master permissions (FLMSTR1) govern in-system host controller requests only.".to_string());
            flashing_constraints.push("Descriptor permissions do NOT establish complete hardware write-protection.".to_string());
            flashing_constraints.push("Complete hardware protection requires interrogation of PCH Protected Range registers (PR0..PR4), BILD lock (BIOS Interface Lock Down), SMM TCO lock, and physical /WP pin state.".to_string());

            let desc_version_str = match profile {
                IfdProfile::V1Legacy => "IFD v1.0 (Legacy ICH8..Wildcat Point / Broadwell)".to_string(),
                IfdProfile::V1_5Intermediate => "IFD v1.5 (Quarantined / Wellsburg C610)".to_string(),
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
                host_bios_writable: host_cpu.as_ref().and_then(|h| h.bios_writable),
                host_me_writable: host_cpu.as_ref().and_then(|h| h.me_writable),
                host_descriptor_writable: host_cpu.as_ref().and_then(|h| h.descriptor_writable),
                flashing_constraints,
            });
        }

        // 9. Parse Flash Regions at FRBA (Profile-aware and geometry-bounded DEF-16, DEF-25, DEF-36, DEF-37, DEF-49)
        let mut succeeding_offsets = vec![FLUMAP1_OFFSET];
        for s in &geometry.section_extents {
            if s.start > frba {
                succeeding_offsets.push(s.start);
            }
        }
        let next_boundary = succeeding_offsets.into_iter().min().unwrap_or(FLUMAP1_OFFSET);

        let declared_table_len = match profile {
            IfdProfile::V1Legacy => ((geometry.nr_raw as usize).saturating_add(1).min(5)) * 4,
            IfdProfile::V2Modern => max_regions.min(16) * 4,
            _ => max_regions * 4,
        };

        if frba + declared_table_len > next_boundary {
            issues.push(format!(
                "Region table with declared length {} bytes (0x{:X}..0x{:X}) extends into succeeding section at 0x{:X}",
                declared_table_len, frba, frba + declared_table_len, next_boundary
            ));
        }

        let nr_bits = geometry.nr_raw as usize;
        let legacy_nr_count = nr_bits.saturating_add(1);
        let effective_regions = if max_regions > 0 {
            max_regions
        } else if profile == IfdProfile::V1Legacy || profile == IfdProfile::Unknown {
            legacy_nr_count
        } else {
            1
        };

        let mut regions = Vec::new();
        let mut region_table_in_bounds = true;

        if !geometry.is_valid || max_regions == 0 || frba + (effective_regions * 4) > image_size {
            region_table_in_bounds = false;
            issues.push(format!(
                "Flash Region Base Address (0x{:X}) extends beyond image size (0x{:X}) or violates geometry bounds",
                frba + (effective_regions * 4),
                image_size
            ));
        } else {
            #[cfg(test)]
            crate::engine::testing::observer::record_ifd_region_table_entry();

            for id in 0..max_regions {
                let reg_offset = frba + (id * 4);
                if let Ok(reg_val) = reader.read_u32_le(reg_offset) {
                    let decoded_bounds = decode_flreg_with_profile(reg_val, profile);
                    let mut region = FlashRegion::from_decoded(id as u8, reg_val, decoded_bounds);

                    // Compute SHA-256 for enabled regions completely within bounds
                    if region.enabled == Some(true) {
                        if let (Some(base), Some(limit), Some(size)) = (region.base, region.limit, region.size) {
                            if limit < image_size {
                                if let Ok(slice) = reader.slice(base, size) {
                                    region.sha256 = Some(crate::engine::binary::checksums::sha256_hex(slice));
                                }
                            }
                        }
                    }

                    regions.push(region);
                }
            }

            #[cfg(test)]
            crate::engine::testing::observer::record_ifd_region_table_completed();
        }

        // 10. Run boundary validation
        let (boundary_conflicts, boundary_warnings) = validate_region_boundaries(&regions, image_size);
        #[cfg(test)]
        if !boundary_conflicts.is_empty() {
            crate::engine::testing::observer::record_ifd_region_conflict();
        }
        issues.extend(boundary_warnings);

        let structural_passed = has_valid_sig && region_table_in_bounds && boundary_conflicts.is_empty();

        let enabled_regions: Vec<String> = regions
            .iter()
            .filter_map(|r| {
                if r.enabled == Some(true) {
                    if let (Some(base), Some(limit)) = (r.base, r.limit) {
                        return Some(format!("{} (0x{:X}..0x{:X})", r.name, base, limit));
                    }
                }
                None
            })
            .collect();

        // 11. Platform classification
        let (platform_family, descriptor_version) = match profile {
            IfdProfile::V1Legacy => (
                "Intel Legacy PCH (ICH8 through 9-Series / Bay Trail)".to_string(),
                "IFD v1.0 (Legacy Layout)".to_string(),
            ),
            IfdProfile::V1_5Intermediate => (
                "Intel Wellsburg / C610 Enterprise PCH (Quarantined)".to_string(),
                "IFD v1.5 (Quarantined Server Layout)".to_string(),
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
        compat_notes.extend(resolution.indicators.clone());

        if regions.iter().any(|r| r.region_type == RegionType::Me && r.enabled == Some(true)) {
            compat_notes.push("Intel Management Engine region present".to_string());
        }
        if regions.iter().any(|r| r.region_type == RegionType::Bios && r.enabled == Some(true)) {
            compat_notes.push("BIOS/UEFI primary payload region present".to_string());
        }

        let platform_passed = structural_passed && profile != IfdProfile::Unknown && profile_confidence != ProfileConfidence::Unknown;

        // 12. Programmer Compatibility Gate (DEF-19, DEF-28, DEF-29, DEF-36)
        let (inferred_cap_bytes, inferred_cap_mb) = if profile == IfdProfile::Unknown || profile == IfdProfile::V1_5Intermediate {
            (None, None)
        } else {
            let fcba_total_capacity: usize = components_info
                .as_ref()
                .map(|c| c.components.iter().filter_map(|d| d.density_bytes).sum())
                .unwrap_or(0);

            let mut highest_limit = 0usize;
            for r in regions.iter().filter(|r| r.enabled == Some(true)) {
                if let Some(limit) = r.limit {
                    if limit > highest_limit {
                        highest_limit = limit;
                    }
                }
            }

            let cap = if fcba_total_capacity > 0 {
                fcba_total_capacity
            } else if highest_limit > 0 {
                highest_limit.saturating_add(1).next_power_of_two()
            } else {
                image_size
            };
            (Some(cap), Some(cap as f64 / (1024.0 * 1024.0)))
        };

        let mut programmer_constraints = Vec::new();
        if let Some(cap) = inferred_cap_bytes {
            programmer_constraints.push(format!(
                "Descriptor-inferred SPI flash capacity estimate: {:.2} MB (0x{:X} bytes)",
                cap as f64 / (1024.0 * 1024.0),
                cap
            ));
        } else {
            programmer_constraints.push("SPI flash capacity cannot be inferred from unclassified/unknown descriptor profile".to_string());
        }
        programmer_constraints.push("External programmer compatibility cannot be positively asserted from binary descriptor analysis alone.".to_string());
        programmer_constraints.push("Requires physical hardware interrogation: flash chip package pinout, JEDEC ID (RDID), operating voltage (1.8V vs 3.3V), and in-circuit hold-off/back-powering characteristics.".to_string());
        programmer_constraints.push("Host-based access permissions are distinct from external SPI programmer compatibility; external programmers bypass PCH master permission masks.".to_string());
        programmer_constraints.push("Descriptor master permissions do not guarantee host or in-circuit writeability: hardware Protected Range registers (PR0..PR4) and BIOS_CNTL lock bits enforce independent hardware protections.".to_string());

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
                passed: false,
                descriptor_inferred_flash_capacity_bytes: inferred_cap_bytes,
                descriptor_inferred_flash_capacity_mb: inferred_cap_mb,
                host_descriptor_bios_write_allowed,
                host_descriptor_write_disabled: host_descriptor_bios_write_allowed.map(|w| !w),
                runtime_chipset_lock_status: RuntimeChipsetLockStatus::Indeterminate,
                hardware_evidence_present: false,
                required_voltage_volts: None,
                verification_constraints: programmer_constraints,
            },
        })
    }
}
