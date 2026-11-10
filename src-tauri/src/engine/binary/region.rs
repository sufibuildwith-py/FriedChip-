use serde::{Deserialize, Serialize};

/// Recognized Flash Descriptor Region types per Intel SPI specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionType {
    Descriptor,
    Bios,
    Me,
    Gbe,
    PlatformData,
    DeviceExpansion,
    SecondaryBios,
    Microcode,
    Ec,
    DeviceExpansion2,
    InnovationEngine,
    TenGbe,
    Reserved,
    Unknown(u8),
}

impl RegionType {
    pub fn from_id(id: u8) -> Self {
        match id {
            0 => RegionType::Descriptor,
            1 => RegionType::Bios,
            2 => RegionType::Me,
            3 => RegionType::Gbe,
            4 => RegionType::PlatformData,
            5 => RegionType::DeviceExpansion,
            6 => RegionType::SecondaryBios,
            7 => RegionType::Microcode,
            8 => RegionType::Ec,
            9 => RegionType::DeviceExpansion2,
            10 => RegionType::InnovationEngine,
            11 => RegionType::TenGbe,
            12 => RegionType::Reserved,
            other => RegionType::Unknown(other),
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            RegionType::Descriptor => "Flash Descriptor",
            RegionType::Bios => "BIOS / UEFI Region",
            RegionType::Me => "Intel ME / CSME / TXE",
            RegionType::Gbe => "Gigabit Ethernet (GbE)",
            RegionType::PlatformData => "Platform Data (PDR)",
            RegionType::DeviceExpansion => "Device Expansion (DER)",
            RegionType::SecondaryBios => "Secondary BIOS",
            RegionType::Microcode => "Microcode / PTT",
            RegionType::Ec => "Embedded Controller (EC)",
            RegionType::DeviceExpansion2 => "Device Expansion 2",
            RegionType::InnovationEngine => "Innovation Engine (IE)",
            RegionType::TenGbe => "10GbE Region",
            RegionType::Reserved => "Reserved Region",
            RegionType::Unknown(_) => "Unknown Region",
        }
    }
}

/// A parsed flash region with physical bounds and status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashRegion {
    pub id: u8,
    pub region_type: RegionType,
    pub name: String,
    pub flreg_raw: u32,
    pub base: Option<usize>,
    pub limit: Option<usize>,
    pub size: Option<usize>,
    pub enabled: Option<bool>,
    pub sha256: Option<String>,
}

impl FlashRegion {
    /// Create a new FlashRegion from decoded base and limit addresses.
    ///
    /// Per Intel specification, if `limit < base`, the region is disabled/unmapped.
    pub fn from_raw_bounds(id: u8, base: usize, limit: usize) -> Self {
        let is_enabled = limit >= base;
        let flreg_raw = ((limit as u32 >> 12) << 16) | (base as u32 >> 12);
        Self::from_decoded(id, flreg_raw, Some((base, limit, is_enabled)))
    }

    /// Create a new FlashRegion with explicit decoded bounds or None if profile is Unknown (DEF-36).
    pub fn from_decoded(id: u8, flreg_raw: u32, bounds: Option<(usize, usize, bool)>) -> Self {
        let region_type = RegionType::from_id(id);
        let name = region_type.display_name().to_string();

        match bounds {
            Some((base, limit, is_enabled)) => {
                if !is_enabled || limit < base {
                    Self {
                        id,
                        region_type,
                        name,
                        flreg_raw,
                        base: Some(base),
                        limit: Some(limit),
                        size: Some(0),
                        enabled: Some(false),
                        sha256: None,
                    }
                } else {
                    let size = limit.saturating_sub(base).saturating_add(1);
                    Self {
                        id,
                        region_type,
                        name,
                        flreg_raw,
                        base: Some(base),
                        limit: Some(limit),
                        size: Some(size),
                        enabled: Some(true),
                        sha256: None,
                    }
                }
            }
            None => {
                // Profile is Unknown: withhold speculative addresses and enabled status (DEF-36)
                Self {
                    id,
                    region_type,
                    name,
                    flreg_raw,
                    base: None,
                    limit: None,
                    size: None,
                    enabled: None,
                    sha256: None,
                }
            }
        }
    }
}

/// Boundary validation errors detected during region table analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryConflict {
    pub region1_name: String,
    pub region1_bounds: (usize, usize),
    pub region2_name: String,
    pub region2_bounds: (usize, usize),
    pub overlap_bytes: usize,
}

/// Validates boundaries across multiple parsed regions.
pub fn validate_region_boundaries(
    regions: &[FlashRegion],
    image_len: usize,
) -> (Vec<BoundaryConflict>, Vec<String>) {
    let mut conflicts = Vec::new();
    let mut warnings = Vec::new();

    let enabled: Vec<(&FlashRegion, usize, usize)> = regions
        .iter()
        .filter_map(|r| {
            if r.enabled == Some(true) {
                if let (Some(base), Some(limit)) = (r.base, r.limit) {
                    return Some((r, base, limit));
                }
            }
            None
        })
        .collect();

    // Check bounds against image length
    for (r, base, limit) in &enabled {
        if *limit >= image_len {
            warnings.push(format!(
                "Region '{}' limit (0x{:X}) exceeds total image size (0x{:X})",
                r.name, limit, image_len
            ));
        }

        // Check 4KB page alignment
        if base % 0x1000 != 0 {
            warnings.push(format!(
                "Region '{}' base address 0x{:X} is not 4KB page aligned",
                r.name, base
            ));
        }
        if (limit + 1) % 0x1000 != 0 {
            warnings.push(format!(
                "Region '{}' limit address 0x{:X} is not 4KB page aligned",
                r.name, limit
            ));
        }
    }

    // Check pairwise overlap
    for i in 0..enabled.len() {
        for j in (i + 1)..enabled.len() {
            let (r1, base1, limit1) = enabled[i];
            let (r2, base2, limit2) = enabled[j];

            // If base1 <= limit2 and base2 <= limit1 -> overlapping
            if base1 <= limit2 && base2 <= limit1 {
                let overlap_start = base1.max(base2);
                let overlap_end = limit1.min(limit2);
                let overlap_bytes = overlap_end.saturating_sub(overlap_start).saturating_add(1);

                conflicts.push(BoundaryConflict {
                    region1_name: r1.name.clone(),
                    region1_bounds: (base1, limit1),
                    region2_name: r2.name.clone(),
                    region2_bounds: (base2, limit2),
                    overlap_bytes,
                });
            }
        }
    }

    (conflicts, warnings)
}
