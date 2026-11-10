use serde::{Deserialize, Serialize};
use crate::engine::binary::reader::BinaryReader;
use crate::engine::binary::region::{validate_region_boundaries, BoundaryConflict, FlashRegion, RegionType};
use crate::error::AppError;

pub const IFD_SIGNATURE_OFFSET: usize = 0x10;
pub const IFD_SIGNATURE_MAGIC: u32 = 0x0FF0A55A;

/// Decodes an Intel Flash Descriptor FLREG value into `(base, limit)`.
///
/// Base address: bits 14:0 shifted left by 12 (4KB pages).
/// Limit address: bits 30:16 shifted left by 12, with bottom 12 bits set to 0xFFF.
///
/// Any bits outside the 15-bit fields (bit 15 and bit 31) are strictly masked out.
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
    pub region_count: usize,
    pub enabled_regions: Vec<String>,
    pub compatibility_notes: Vec<String>,
}

/// Programmer compatibility assessment results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate3ProgrammerCompatibility {
    pub passed: bool,
    pub detected_chip_size_bytes: usize,
    pub detected_chip_size_mb: f64,
    pub external_spi_supported: bool,
    pub host_flashing_locked: bool,
    pub verification_constraints: Vec<String>,
}

/// Full analysis report generated for an Intel Flash Descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfdAnalysisReport {
    pub image_size: usize,
    pub regions: Vec<FlashRegion>,
    pub gate1_structural: Gate1StructuralValidation,
    pub gate2_platform: Gate2PlatformCompatibility,
    pub gate3_programmer: Gate3ProgrammerCompatibility,
}

pub struct IntelFlashDescriptor;

impl IntelFlashDescriptor {
    /// Inspect a binary dump for an Intel Flash Descriptor and parse its region table.
    pub fn parse(image_bytes: &[u8]) -> Result<IfdAnalysisReport, AppError> {
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
                regions: Vec::new(),
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
                    platform_family: "Unknown".to_string(),
                    descriptor_version: "Unknown".to_string(),
                    region_count: 0,
                    enabled_regions: Vec::new(),
                    compatibility_notes: vec!["Cannot evaluate platform without valid IFD".to_string()],
                },
                gate3_programmer: Gate3ProgrammerCompatibility {
                    passed: false,
                    detected_chip_size_bytes: image_size,
                    detected_chip_size_mb: image_size as f64 / (1024.0 * 1024.0),
                    external_spi_supported: false,
                    host_flashing_locked: true,
                    verification_constraints: vec!["Image is not recognized as a valid Intel SPI image".to_string()],
                },
            });
        }

        // 2. Read FLMAP0 at offset 0x14
        let flmap0 = reader.read_u32_le(0x14)?;
        let nr = ((flmap0 & 0xFF) as u8).saturating_add(1); // 0-based count of regions, max 16
        let frba_field = ((flmap0 >> 16) & 0xFF) as usize;
        let frba = frba_field << 4; // FRBA byte offset in descriptor

        // 3. Read FLMAP1 at offset 0x18
        let flmap1 = reader.read_u32_le(0x18)?;
        let nc = ((flmap1 & 0x03) as u8).saturating_add(1); // Number of flash components (1 or 2)
        let fcba_field = ((flmap1 >> 16) & 0xFF) as usize;
        let _fcba = fcba_field << 4;

        // 4. Parse regions
        let max_regions = nr.clamp(5, 16) as usize;
        let mut regions = Vec::new();
        let mut region_table_in_bounds = true;
        let mut issues = Vec::new();

        if frba + (max_regions * 4) > image_size {
            region_table_in_bounds = false;
            issues.push(format!(
                "Flash Region Base Address (0x{:X}) extends beyond image size (0x{:X})",
                frba + (max_regions * 4),
                image_size
            ));
        } else {
            for id in 0..max_regions {
                let reg_offset = frba + (id * 4);
                if let Ok(reg_val) = reader.read_u32_le(reg_offset) {
                    let (base, limit) = decode_flreg(reg_val);
                    let mut region = FlashRegion::from_raw_bounds(id as u8, base, limit);

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

        // 5. Run boundary validation
        let (boundary_conflicts, boundary_warnings) = validate_region_boundaries(&regions, image_size);
        issues.extend(boundary_warnings);

        let structural_passed = has_valid_sig && region_table_in_bounds && boundary_conflicts.is_empty();

        let enabled_regions: Vec<String> = regions
            .iter()
            .filter(|r| r.enabled)
            .map(|r| format!("{} (0x{:X}..0x{:X})", r.name, r.base, r.limit))
            .collect();

        // 6. Platform determination heuristics
        let platform_family = if nr >= 10 {
            "Intel PCH (Skylake / Comet Lake / Tiger Lake / Alder Lake / Raptor Lake)".to_string()
        } else if nr >= 5 {
            "Intel Series 6..9 PCH / Cougar Point / Lynx Point".to_string()
        } else {
            "Legacy Intel ICH (ICH7..ICH10)".to_string()
        };

        let descriptor_version = if nr >= 10 { "IFD v2.0" } else { "IFD v1.0" }.to_string();

        let mut compat_notes = Vec::new();
        compat_notes.push(format!("Detected Flash Components: {}", nc));
        if regions.iter().any(|r| r.region_type == RegionType::Me && r.enabled) {
            compat_notes.push("Intel Management Engine region present".to_string());
        }
        if regions.iter().any(|r| r.region_type == RegionType::Bios && r.enabled) {
            compat_notes.push("BIOS/UEFI primary payload region present".to_string());
        }

        // 7. Programmer Compatibility Gate
        // Determine chip capacity based on highest limit of enabled regions, or FCBA component sizes
        let mut highest_limit = 0usize;
        for r in regions.iter().filter(|r| r.enabled) {
            if r.limit > highest_limit {
                highest_limit = r.limit;
            }
        }
        let detected_capacity = if highest_limit > 0 {
            // Round up to nearest power of 2 flash size (4MB, 8MB, 16MB, 32MB)
            highest_limit.saturating_add(1).next_power_of_two()
        } else {
            image_size
        };

        let mut programmer_constraints = Vec::new();
        programmer_constraints.push(format!(
            "Required SPI NOR flash capacity: {} MB (0x{:X} bytes)",
            detected_capacity / (1024 * 1024),
            detected_capacity
        ));
        programmer_constraints.push("External hardware programmer (CH341A / Dediprog / Flashrom) can flash entire image safely if write-protect pins are handled.".to_string());
        programmer_constraints.push("In-system host flashing requires verifying Flash Descriptor Master Access permissions (FRAP/PR registers).".to_string());

        let programmer_passed = structural_passed && image_size >= detected_capacity;

        Ok(IfdAnalysisReport {
            image_size,
            regions,
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
                passed: structural_passed,
                platform_family,
                descriptor_version,
                region_count: nr as usize,
                enabled_regions,
                compatibility_notes: compat_notes,
            },
            gate3_programmer: Gate3ProgrammerCompatibility {
                passed: programmer_passed,
                detected_chip_size_bytes: detected_capacity,
                detected_chip_size_mb: detected_capacity as f64 / (1024.0 * 1024.0),
                external_spi_supported: structural_passed,
                host_flashing_locked: true, // conservative default without reading PCH master permissions
                verification_constraints: programmer_constraints,
            },
        })
    }
}
