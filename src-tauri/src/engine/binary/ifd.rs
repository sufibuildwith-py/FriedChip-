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

/// Programmer compatibility assessment states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProgrammerCompatibilityStatus {
    Compatible,
    Incompatible,
    Indeterminate,
}

/// Programmer compatibility assessment results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate3ProgrammerCompatibility {
    pub status: ProgrammerCompatibilityStatus,
    pub passed: bool,
    pub detected_chip_size_bytes: usize,
    pub detected_chip_size_mb: f64,
    pub host_flashing_locked: bool,
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
    pub read_clock_frequency_mhz: u32,
    pub fast_read_supported: bool,
    pub fast_read_clock_frequency_mhz: u32,
    pub write_erase_clock_frequency_mhz: u32,
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
    pub regions: Vec<FlashRegion>,
    pub components: Option<FlashComponentsInfo>,
    pub master_permissions: Option<FlashMasterPermissions>,
    pub gate1_structural: Gate1StructuralValidation,
    pub gate2_platform: Gate2PlatformCompatibility,
    pub gate3_programmer: Gate3ProgrammerCompatibility,
}

/// Decodes raw 3-bit SPI frequency fields from FLCOMP into clock frequency in MHz.
pub fn decode_spi_frequency_mhz(bits: u8) -> u32 {
    match bits & 0x07 {
        0 => 20,
        1 => 33,
        2 => 48,
        4 => 50,
        _ => 20,
    }
}

/// Decodes the 32-bit FLCOMP register at FCBA into `FlashComponentsInfo`.
pub fn decode_flcomp(fcba_offset: usize, flcomp: u32, component_count: usize) -> FlashComponentsInfo {
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

    // Component 2 density (bits 5:3) if component_count >= 2
    if component_count >= 2 {
        let raw2 = ((flcomp >> 3) & 0x07) as u8;
        let size2 = (512 * 1024) << raw2;
        comp_densities.push(FlashComponentDensity {
            component_index: 2,
            raw_bits: raw2,
            density_bytes: size2,
            density_mb: size2 as f64 / (1024.0 * 1024.0),
        });
    }

    let freq_read_bits = ((flcomp >> 17) & 0x07) as u8;
    let read_clock_frequency_mhz = decode_spi_frequency_mhz(freq_read_bits);

    let fast_read_supported = ((flcomp >> 20) & 0x01) == 1;

    let freq_fastread_bits = ((flcomp >> 21) & 0x07) as u8;
    let fast_read_clock_frequency_mhz = decode_spi_frequency_mhz(freq_fastread_bits);

    let freq_write_bits = ((flcomp >> 24) & 0x07) as u8;
    let write_erase_clock_frequency_mhz = decode_spi_frequency_mhz(freq_write_bits);

    let freq_read_id_bits = ((flcomp >> 27) & 0x07) as u8;
    let read_id_status_clock_frequency_mhz = decode_spi_frequency_mhz(freq_read_id_bits);

    FlashComponentsInfo {
        fcba_offset,
        flcomp_raw: flcomp,
        component_count,
        components: comp_densities,
        read_clock_frequency_mhz,
        fast_read_supported,
        fast_read_clock_frequency_mhz,
        write_erase_clock_frequency_mhz,
        read_id_status_clock_frequency_mhz,
    }
}

/// Decodes an individual 32-bit FLMSTR register into a `MasterAccessRule`.
pub fn decode_master_rule(
    master_id: usize,
    master_name: &str,
    flmstr: u32,
    is_v2: bool,
) -> MasterAccessRule {
    let (requester_id, read_mask, write_mask) = if !is_v2 {
        // IFD v1 (Legacy ICH8..Wildcat Point / Broadwell, 5-series..9-series)
        // Bits 15:0 = Requester ID
        // Bits 23:16 = Read access mask (regions 0..7)
        // Bits 31:24 = Write access mask (regions 0..7)
        let req_id = (flmstr & 0xFFFF) as u16;
        let r_mask = ((flmstr >> 16) & 0xFF) as u16;
        let w_mask = ((flmstr >> 24) & 0xFF) as u16;
        (Some(req_id), r_mask, w_mask)
    } else {
        // IFD v2 (Modern Skylake / Sunrise Point 100-series through 700-series)
        // Bits 19:8 = Read access mask (regions 0..11)
        // Bits 31:20 = Write access mask (regions 0..11)
        let mut r_mask = ((flmstr >> 8) & 0xFFF) as u16;
        let mut w_mask = ((flmstr >> 20) & 0xFFF) as u16;
        // Extended regions 12..15 in Cannon Lake+
        r_mask |= ((flmstr & 0x0F) as u16) << 12;
        w_mask |= (((flmstr >> 4) & 0x0F) as u16) << 12;
        (None, r_mask, w_mask)
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

pub struct IntelFlashDescriptor;

impl IntelFlashDescriptor {
    /// Inspect a binary dump for an Intel Flash Descriptor and parse its region table and hardware configuration.
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
                    status: ProgrammerCompatibilityStatus::Incompatible,
                    passed: false,
                    detected_chip_size_bytes: image_size,
                    detected_chip_size_mb: image_size as f64 / (1024.0 * 1024.0),
                    host_flashing_locked: true,
                    hardware_evidence_present: false,
                    required_voltage_volts: None,
                    verification_constraints: vec!["Image is not recognized as a valid Intel SPI image".to_string()],
                },
            });
        }

        // 2. Read FLMAP0 at offset 0x14
        // Spec bitfields:
        // Bits 7:0:   FCBA (Flash Component Base Address, byte offset = FCBA << 4)
        // Bits 9:8:   NC (Number Of Components: 0 = 1 component, 1 = 2 components)
        // Bits 15:10: Reserved
        // Bits 23:16: FRBA (Flash Region Base Address, byte offset = FRBA << 4)
        // Bits 26:24 (or 27:24 in v2): NR (Number Of Regions: 0-based, total = NR + 1)
        let flmap0 = reader.read_u32_le(0x14)?;
        let fcba_field = (flmap0 & 0xFF) as usize;
        let fcba = fcba_field << 4;
        let nc = (((flmap0 >> 8) & 0x03) as u8).saturating_add(1);
        let frba_field = ((flmap0 >> 16) & 0xFF) as usize;
        let frba = frba_field << 4;
        let nr_field = ((flmap0 >> 24) & 0x0F) as u8;
        let nr = nr_field.saturating_add(1);

        // 3. Read FLMAP1 at offset 0x18
        // Spec bitfields:
        // Bits 7:0:   FMBA (Flash Master Base Address, byte offset = FMBA << 4)
        // Bits 10:8:  NM (Number Of Masters: 0-based, total = NM + 1)
        // Bits 15:11: Reserved
        // Bits 23:16: FISBA (Flash ICH/PCH Strap Base Address, byte offset = FISBA << 4)
        // Bits 31:24: ISL (ICH/PCH Strap Length in Dwords)
        let flmap1 = reader.read_u32_le(0x18)?;
        let fmba_field = (flmap1 & 0xFF) as usize;
        let fmba = fmba_field << 4;
        let nm_field = ((flmap1 >> 8) & 0x07) as u8;
        let nm = nm_field.saturating_add(1);
        let fisba_field = ((flmap1 >> 16) & 0xFF) as usize;
        let _fisba = fisba_field << 4;
        let _isl = ((flmap1 >> 24) & 0xFF) as usize;

        // 4. Read FLMAP2 at offset 0x1C (if available)
        // Spec bitfields:
        // Bits 7:0:   FMSBA (Flash MCH/Processor Strap Base Address, byte offset = FMSBA << 4)
        // Bits 15:8:  MSL (MCH/Processor Strap Length in Dwords)
        let flmap2 = reader.read_u32_le(0x1C).unwrap_or(0);
        let fmsba_field = (flmap2 & 0xFF) as usize;
        let _fmsba = fmsba_field << 4;
        let _msl = ((flmap2 >> 8) & 0xFF) as usize;

        let mut issues = Vec::new();

        // 5. Decode FCBA (Flash Components Base Address)
        let mut components_info = None;
        if fcba + 4 > image_size {
            issues.push(format!(
                "Flash Component Base Address (0x{:X}) extends beyond image size (0x{:X})",
                fcba + 4, image_size
            ));
        } else if let Ok(flcomp) = reader.read_u32_le(fcba) {
            components_info = Some(decode_flcomp(fcba, flcomp, nc as usize));
        }

        // 6. Decode FMBA (Flash Master Base Address)
        let mut master_permissions = None;
        let mut host_flashing_locked = true;

        let master_count = (nm as usize).clamp(1, 4);
        if fmba + (master_count * 4) > image_size {
            issues.push(format!(
                "Flash Master Base Address (0x{:X}) extends beyond image size (0x{:X})",
                fmba + (master_count * 4), image_size
            ));
        } else {
            let mut masters = Vec::new();
            let is_v2 = nr >= 10;

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
                    masters.push(decode_master_rule(m_idx + 1, master_name, flmstr, is_v2));
                }
            }

            let host_cpu = masters.get(0).cloned().unwrap_or_else(|| {
                decode_master_rule(1, "Host CPU / BIOS", 0, is_v2)
            });
            let intel_me = masters.get(1).cloned().unwrap_or_else(|| {
                decode_master_rule(2, "Intel Management Engine", 0, is_v2)
            });
            let gbe = masters.get(2).cloned().unwrap_or_else(|| {
                decode_master_rule(3, "Gigabit Ethernet", 0, is_v2)
            });

            host_flashing_locked = !host_cpu.bios_writable;

            let mut flashing_constraints = Vec::new();
            flashing_constraints.push("Flash Master permissions (FLMSTR1) govern in-system host controller requests only.".to_string());
            flashing_constraints.push("Descriptor permissions do NOT establish complete hardware write-protection.".to_string());
            flashing_constraints.push("Complete hardware protection requires interrogation of PCH Protected Range registers (PR0..PR4), BILD lock (BIOS Interface Lock Down), SMM TCO lock, and physical /WP pin state.".to_string());

            master_permissions = Some(FlashMasterPermissions {
                fmba_offset: fmba,
                master_count,
                descriptor_version: if is_v2 { "IFD v2.0 (Skylake+)" } else { "IFD v1.0 (Legacy ICH/PCH)" }.to_string(),
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

        // 7. Parse Flash Regions at FRBA
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

        // 8. Run boundary validation
        let (boundary_conflicts, boundary_warnings) = validate_region_boundaries(&regions, image_size);
        issues.extend(boundary_warnings);

        let structural_passed = has_valid_sig && region_table_in_bounds && boundary_conflicts.is_empty();

        let enabled_regions: Vec<String> = regions
            .iter()
            .filter(|r| r.enabled)
            .map(|r| format!("{} (0x{:X}..0x{:X})", r.name, r.base, r.limit))
            .collect();

        // 9. Platform classification
        let platform_family = format!("Intel PCH (Unclassified, Region Count: {})", nr);
        let descriptor_version = if nr >= 10 {
            "IFD v2.0 (10+ Regions)".to_string()
        } else {
            "IFD v1.0 (Legacy Layout)".to_string()
        };

        let mut compat_notes = Vec::new();
        compat_notes.push(format!("Detected Flash Components: {}", nc));
        compat_notes.push(format!("Detected Flash Masters: {}", nm));
        compat_notes.push("Chipset generation cannot be verified from region count alone; marked as UNCLASSIFIED pending PCH strap/device ID inspection.".to_string());
        if regions.iter().any(|r| r.region_type == RegionType::Me && r.enabled) {
            compat_notes.push("Intel Management Engine region present".to_string());
        }
        if regions.iter().any(|r| r.region_type == RegionType::Bios && r.enabled) {
            compat_notes.push("BIOS/UEFI primary payload region present".to_string());
        }

        // 10. Programmer Compatibility Gate (DEF-12)
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
        programmer_constraints.push("External programmer compatibility cannot be positively asserted from binary descriptor analysis alone.".to_string());
        programmer_constraints.push("Requires physical hardware interrogation: flash chip package pinout, JEDEC ID (RDID), operating voltage (1.8V vs 3.3V), and in-circuit hold-off/back-powering characteristics.".to_string());
        programmer_constraints.push("Host-based access permissions are distinct from external SPI programmer compatibility; external programmers bypass PCH master permission masks.".to_string());
        programmer_constraints.push("Descriptor master permissions do not guarantee host or in-circuit writeability: hardware Protected Range registers (PR0..PR4) and BIOS_CNTL lock bits enforce independent hardware protections.".to_string());

        let programmer_status = if !structural_passed || image_size < detected_capacity {
            ProgrammerCompatibilityStatus::Incompatible
        } else {
            // Without hardware programmer verification, status is strictly Indeterminate
            ProgrammerCompatibilityStatus::Indeterminate
        };

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
                status: programmer_status,
                passed: false, // Refuse speculative positive compatibility without hardware
                detected_chip_size_bytes: detected_capacity,
                detected_chip_size_mb: detected_capacity as f64 / (1024.0 * 1024.0),
                host_flashing_locked,
                hardware_evidence_present: false,
                required_voltage_volts: None,
                verification_constraints: programmer_constraints,
            },
        })
    }
}
