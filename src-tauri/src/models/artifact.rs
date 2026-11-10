use serde::{Deserialize, Serialize};

/// Recognized format classification for firmware artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactFormat {
    RawSpiDump,
    IntelFlashDescriptor,
    DellPfsContainer,
    FirmwarePartition,
    UefiFirmwareVolume,
    EmbeddedController,
    PlatformXml,
    Unknown,
}

impl std::fmt::Display for ArtifactFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RawSpiDump => write!(f, "raw_spi_dump"),
            Self::IntelFlashDescriptor => write!(f, "intel_flash_descriptor"),
            Self::DellPfsContainer => write!(f, "dell_pfs_container"),
            Self::FirmwarePartition => write!(f, "firmware_partition"),
            Self::UefiFirmwareVolume => write!(f, "uefi_firmware_volume"),
            Self::EmbeddedController => write!(f, "embedded_controller"),
            Self::PlatformXml => write!(f, "platform_xml"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for ArtifactFormat {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "raw_spi_dump" => Self::RawSpiDump,
            "intel_flash_descriptor" => Self::IntelFlashDescriptor,
            "dell_pfs_container" => Self::DellPfsContainer,
            "firmware_partition" => Self::FirmwarePartition,
            "uefi_firmware_volume" => Self::UefiFirmwareVolume,
            "embedded_controller" => Self::EmbeddedController,
            "platform_xml" => Self::PlatformXml,
            _ => Self::Unknown,
        })
    }
}

/// A universal, vendor-agnostic representation of an extracted or analyzed firmware artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirmwareArtifact {
    pub id: String,
    pub workspace_id: String,
    pub source_file_id: String,
    pub parent_artifact_id: Option<String>,
    pub name: String,
    pub format: ArtifactFormat,
    pub source_offset: usize,
    pub raw_size: usize,
    pub sha256: String,
    pub md5: Option<String>,
    pub file_path: Option<String>,
    pub gate1_passed: bool,
    pub gate2_passed: bool,
    pub gate3_passed: bool,
    pub validation_notes: Vec<String>,
    pub warnings: Vec<String>,
    pub metadata_json: String,
    pub created_at: String,
}
