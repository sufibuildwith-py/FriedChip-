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
    pub component_count: usize,
    pub components: Vec<FlashComponentDensity>,
    pub fast_read_supported: bool,
    pub fast_read_clock_frequency_mhz: u32,
}

/// Decoded master access permissions for a specific PCH execution unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterAccessRule {
    pub master_name: String,
    pub read_mask: u8,
    pub write_mask: u8,
    pub bios_readable: bool,
    pub bios_writable: bool,
    pub me_readable: bool,
    pub me_writable: bool,
    pub gbe_readable: bool,
    pub gbe_writable: bool,
}

/// Flash Master Access Section (FMBA / FRAP) decoded permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashMasterPermissions {
    pub fmba_offset: usize,
    pub frap_raw: u32,
    pub host_cpu: MasterAccessRule,
    pub intel_me: MasterAccessRule,
    pub gbe: MasterAccessRule,
    pub host_bios_writable: bool,
    pub host_me_writable: bool,
}

/// Full analysis report generated for an Intel Flash Descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfdAnalysisReport {
    pub image_size: usize,
    pub regions: Vec<FlashRegion>,
    pub components: Option<FlashComponentsInfo>,
    pub master_permissions: Option<FlashMasterPermissions>,
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
        let fcba = fcba_field << 4; // FCBA byte offset in descriptor
        let fmba_field = ((flmap1 >> 24) & 0xFF) as usize;
        let fmba = fmba_field << 4; // FMBA byte offset in descriptor

        let mut issues = Vec::new();

        // 4. Decode FCBA (Flash Components Base Address)
        let mut components_info = None;
        if fcba + 4 > image_size {
            issues.push(format!(
                "Flash Component Base Address (0x{:X}) extends beyond image size (0x{:X})",
                fcba + 4, image_size
            ));
        } else if let Ok(flcomp) = reader.read_u32_le(fcba) {
            let mut comp_densities = Vec::new();

            // Component 1 density (bits 2:0)
            let raw1 = (flcomp & 0x07) as u8;
            let size1 = (512 * 1024) << raw1;
            comp_densities.push(FlashComponentDensity {
                component_index: 1,
                raw_bits: raw1,
                density_bytes: size1,
                density_mb: size1 as f64 / (1024.0 * 1024.0),
            });

            // Component 2 density (bits 5:3) if dual component
            if nc >= 2 {
                let raw2 = ((flcomp >> 3) & 0x07) as u8;
                let size2 = (512 * 1024) << raw2;
                comp_densities.push(FlashComponentDensity {
                    component_index: 2,
                    raw_bits: raw2,
                    density_bytes: size2,
                    density_mb: size2 as f64 / (1024.0 * 1024.0),
                });
            }

            let fast_read_supported = ((flcomp >> 13) & 1) == 1;
            let freq_bits = (flcomp >> 14) & 0x0F;
            let fast_read_clock_frequency_mhz = match freq_bits {
                0 => 20,
                1 => 33,
                2 => 48,
                4 => 50,
                _ => 20,
            };

            components_info = Some(FlashComponentsInfo {
                fcba_offset: fcba,
                component_count: nc as usize,
                components: comp_densities,
                fast_read_supported,
                fast_read_clock_frequency_mhz,
            });
        }

        // 5. Decode FMBA / FRAP (Flash Region Access Permissions)
        let mut master_permissions = None;
        let mut host_flashing_locked = true;

        if fmba + 4 > image_size {
            issues.push(format!(
                "Flash Master Base Address (0x{:X}) extends beyond image size (0x{:X})",
                fmba + 4, image_size
            ));
        } else if let Ok(frap) = reader.read_u32_le(fmba) {
            let bios_read = (frap & 0xFF) as u8;
            let bios_write = ((frap >> 8) & 0xFF) as u8;
            let me_read = ((frap >> 16) & 0xFF) as u8;
            let me_write = ((frap >> 24) & 0xFF) as u8;

            let host_bios_writable = (bios_write & (1 << 1)) != 0;
            let host_me_writable = (bios_write & (1 << 2)) != 0;

            // Host flashing is considered locked if Host CPU cannot write to Region 1 (BIOS)
            host_flashing_locked = !host_bios_writable;

            let host_cpu = MasterAccessRule {
                master_name: "Host CPU / BIOS".to_string(),
                read_mask: bios_read,
                write_mask: bios_write,
                bios_readable: (bios_read & (1 << 1)) != 0,
                bios_writable: host_bios_writable,
                me_readable: (bios_read & (1 << 2)) != 0,
                me_writable: host_me_writable,
                gbe_readable: (bios_read & (1 << 3)) != 0,
                gbe_writable: (bios_write & (1 << 3)) != 0,
            };

            let intel_me = MasterAccessRule {
                master_name: "Intel Management Engine".to_string(),
                read_mask: me_read,
                write_mask: me_write,
                bios_readable: (me_read & (1 << 1)) != 0,
                bios_writable: (me_write & (1 << 1)) != 0,
                me_readable: (me_read & (1 << 2)) != 0,
                me_writable: (me_write & (1 << 2)) != 0,
                gbe_readable: (me_read & (1 << 3)) != 0,
                gbe_writable: (me_write & (1 << 3)) != 0,
            };

            let gbe = MasterAccessRule {
                master_name: "GbE Controller".to_string(),
                read_mask: 0,
                write_mask: 0,
                bios_readable: false,
                bios_writable: false,
                me_readable: false,
                me_writable: false,
                gbe_readable: true,
                gbe_writable: true,
            };

            master_permissions = Some(FlashMasterPermissions {
                fmba_offset: fmba,
                frap_raw: frap,
                host_cpu,
                intel_me,
                gbe,
                host_bios_writable,
                host_me_writable,
            });
        }

        // 6. Parse regions
        let max_regions = nr.clamp(5, 16) as usize;
        let mut regions = Vec::new();
        let mut region_table_in_bounds = true;

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

        // 7. Run boundary validation
        let (boundary_conflicts, boundary_warnings) = validate_region_boundaries(&regions, image_size);
        issues.extend(boundary_warnings);

        let structural_passed = has_valid_sig && region_table_in_bounds && boundary_conflicts.is_empty();

        let enabled_regions: Vec<String> = regions
            .iter()
            .filter(|r| r.enabled)
            .map(|r| format!("{} (0x{:X}..0x{:X})", r.name, r.base, r.limit))
            .collect();

        // 8. Platform classification: Refuse speculative chipset names based on region count
        let platform_family = format!("Intel PCH (Unclassified, Region Count: {})", nr);
        let descriptor_version = if nr >= 10 {
            "IFD v2.0 (10+ Regions)".to_string()
        } else {
            "IFD v1.0 (Legacy Layout)".to_string()
        };

        let mut compat_notes = Vec::new();
        compat_notes.push(format!("Detected Flash Components: {}", nc));
        compat_notes.push("Chipset generation cannot be verified from region count alone; marked as UNCLASSIFIED pending PCH strap/device ID inspection.".to_string());
        if regions.iter().any(|r| r.region_type == RegionType::Me && r.enabled) {
            compat_notes.push("Intel Management Engine region present".to_string());
        }
        if regions.iter().any(|r| r.region_type == RegionType::Bios && r.enabled) {
            compat_notes.push("BIOS/UEFI primary payload region present".to_string());
        }

        // 9. Programmer Compatibility Gate
        // Determine chip capacity based on FCBA component sizes, or highest limit of enabled regions
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
            "Required SPI NOR flash capacity: {:.2} MB (0x{:X} bytes)",
            detected_capacity as f64 / (1024.0 * 1024.0),
            detected_capacity
        ));
        programmer_constraints.push("Descriptor permissions reflect in-system host permissions only; complete hardware write-protection depends on PCH Protected Range registers (PR0..PR4), BILD lock, and physical SPI write-protect (/WP) pin state.".to_string());
        programmer_constraints.push("Programmer compatibility cannot be established solely from descriptor parsing; requires physical flash chip capacity match and external programming interface.".to_string());

        let programmer_passed = structural_passed && image_size >= detected_capacity;

        Ok(IfdAnalysisReport {
            image_size,
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
                host_flashing_locked,
                verification_constraints: programmer_constraints,
            },
        })
    }
}
