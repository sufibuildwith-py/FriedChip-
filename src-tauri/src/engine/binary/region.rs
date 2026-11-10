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
    pub base: usize,
    pub limit: usize,
    pub size: usize,
    pub enabled: bool,
    pub sha256: Option<String>,
}

impl FlashRegion {
    /// Create a new FlashRegion from decoded base and limit addresses.
    ///
    /// Per Intel specification, if `limit < base`, the region is disabled/unmapped.
    pub fn from_raw_bounds(id: u8, base: usize, limit: usize) -> Self {
        let region_type = RegionType::from_id(id);
        let name = region_type.display_name().to_string();

        if limit < base {
            Self {
                id,
                region_type,
                name,
                base,
                limit,
                size: 0,
                enabled: false,
                sha256: None,
            }
        } else {
            // Region is inclusive: size = limit - base + 1
            let size = limit.saturating_sub(base).saturating_add(1);
            Self {
                id,
                region_type,
                name,
                base,
                limit,
                size,
                enabled: true,
                sha256: None,
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

    let enabled: Vec<&FlashRegion> = regions.iter().filter(|r| r.enabled).collect();

    // Check bounds against image length
    for r in &enabled {
        if r.limit >= image_len {
            warnings.push(format!(
                "Region '{}' limit (0x{:X}) exceeds total image size (0x{:X})",
                r.name, r.limit, image_len
            ));
        }

        // Check 4KB page alignment
        if r.base % 0x1000 != 0 {
            warnings.push(format!(
                "Region '{}' base address 0x{:X} is not 4KB page aligned",
                r.name, r.base
            ));
        }
        if (r.limit + 1) % 0x1000 != 0 {
            warnings.push(format!(
                "Region '{}' limit address 0x{:X} is not 4KB page aligned",
                r.name, r.limit
            ));
        }
    }

    // Check pairwise overlap
    for i in 0..enabled.len() {
        for j in (i + 1)..enabled.len() {
            let r1 = enabled[i];
            let r2 = enabled[j];

            // If r1.base <= r2.limit and r2.base <= r1.limit -> overlapping
            if r1.base <= r2.limit && r2.base <= r1.limit {
                let overlap_start = r1.base.max(r2.base);
                let overlap_end = r1.limit.min(r2.limit);
                let overlap_bytes = overlap_end.saturating_sub(overlap_start).saturating_add(1);

                conflicts.push(BoundaryConflict {
                    region1_name: r1.name.clone(),
                    region1_bounds: (r1.base, r1.limit),
                    region2_name: r2.name.clone(),
                    region2_bounds: (r2.base, r2.limit),
                    overlap_bytes,
                });
            }
        }
    }

    (conflicts, warnings)
}
