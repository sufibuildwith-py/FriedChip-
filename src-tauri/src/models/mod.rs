use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FirmwareFormat {
    IntelFlashDescriptor,
    UefiFirmwareVolume,
    DellPfsContainer,
    AsusCapsule,
    HpInsydePayload,
    Unclassified,
    Unknown,
}

impl ToString for FirmwareFormat {
    fn to_string(&self) -> String {
        match self {
            FirmwareFormat::IntelFlashDescriptor => "Intel Flash Descriptor (SPI)".to_string(),
            FirmwareFormat::UefiFirmwareVolume => "UEFI Firmware Volume (FV)".to_string(),
            FirmwareFormat::DellPfsContainer => "Dell PFS Update Package".to_string(),
            FirmwareFormat::AsusCapsule => "Asus Security Capsule".to_string(),
            FirmwareFormat::HpInsydePayload => "HP Insyde Firmware Payload".to_string(),
            FirmwareFormat::Unclassified => "UNCLASSIFIED".to_string(),
            FirmwareFormat::Unknown => "UNKNOWN".to_string(),
        }
    }
}

impl From<&str> for FirmwareFormat {
    fn from(s: &str) -> Self {
        match s {
            "IntelFlashDescriptor" | "Intel Flash Descriptor (SPI)" => FirmwareFormat::IntelFlashDescriptor,
            "UefiFirmwareVolume" | "UEFI Firmware Volume (FV)" => FirmwareFormat::UefiFirmwareVolume,
            "DellPfsContainer" | "Dell PFS Update Package" => FirmwareFormat::DellPfsContainer,
            "AsusCapsule" | "Asus Security Capsule" => FirmwareFormat::AsusCapsule,
            "HpInsydePayload" | "HP Insyde Firmware Payload" => FirmwareFormat::HpInsydePayload,
            "UNCLASSIFIED" | "Unclassified" => FirmwareFormat::Unclassified,
            _ => FirmwareFormat::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FormatConfidence {
    Confirmed,
    Heuristic,
    Uncertain,
}

impl ToString for FormatConfidence {
    fn to_string(&self) -> String {
        match self {
            FormatConfidence::Confirmed => "CONFIRMED".to_string(),
            FormatConfidence::Heuristic => "HEURISTIC".to_string(),
            FormatConfidence::Uncertain => "UNCERTAIN".to_string(),
        }
    }
}

impl From<&str> for FormatConfidence {
    fn from(s: &str) -> Self {
        match s {
            "CONFIRMED" | "Confirmed" => FormatConfidence::Confirmed,
            "HEURISTIC" | "Heuristic" => FormatConfidence::Heuristic,
            _ => FormatConfidence::Uncertain,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FirmwareFile {
    pub id: String,
    pub workspace_id: String,
    pub original_path: String,
    pub filename: String,
    pub extension: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub md5: String,
    pub detected_format: FirmwareFormat,
    pub format_confidence: FormatConfidence,
    pub import_timestamp: String,
    pub is_read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl ToString for OperationStatus {
    fn to_string(&self) -> String {
        match self {
            OperationStatus::Pending => "PENDING".to_string(),
            OperationStatus::Running => "RUNNING".to_string(),
            OperationStatus::Completed => "COMPLETED".to_string(),
            OperationStatus::Failed => "FAILED".to_string(),
            OperationStatus::Cancelled => "CANCELLED".to_string(),
        }
    }
}

impl From<&str> for OperationStatus {
    fn from(s: &str) -> Self {
        match s {
            "PENDING" => OperationStatus::Pending,
            "RUNNING" => OperationStatus::Running,
            "COMPLETED" => OperationStatus::Completed,
            "FAILED" => OperationStatus::Failed,
            "CANCELLED" => OperationStatus::Cancelled,
            _ => OperationStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Operation {
    pub id: String,
    pub workspace_id: String,
    pub operation_type: String,
    pub input_file_id: Option<String>,
    pub input_sha256: Option<String>,
    pub status: OperationStatus,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationLog {
    pub id: i64,
    pub operation_id: String,
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSetting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}
