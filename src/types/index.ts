export type FirmwareFormat =
  | "IntelFlashDescriptor"
  | "UefiFirmwareVolume"
  | "DellPfsContainer"
  | "AsusCapsule"
  | "HpInsydePayload"
  | "Unclassified"
  | "Unknown";

export type FormatConfidence = "Confirmed" | "Heuristic" | "Uncertain";

export type OperationStatus = "PENDING" | "RUNNING" | "COMPLETED" | "FAILED" | "CANCELLED";

export interface Workspace {
  id: string;
  name: string;
  root_path: string;
  created_at: string;
  updated_at: string;
  is_default: boolean;
}

export interface FirmwareFile {
  id: string;
  workspace_id: string;
  original_path: string;
  filename: string;
  extension: string;
  size_bytes: number;
  sha256: string;
  md5: string;
  detected_format: FirmwareFormat;
  format_confidence: FormatConfidence;
  import_timestamp: string;
  is_read_only: boolean;
}

export interface Operation {
  id: string;
  workspace_id: string;
  operation_type: string;
  input_file_id: string | null;
  input_sha256: string | null;
  status: OperationStatus;
  created_at: string;
  completed_at: string | null;
  error_message: string | null;
}

export interface OperationLog {
  id: number;
  operation_id: string;
  timestamp: string;
  level: "INFO" | "WARN" | "ERROR";
  message: string;
}

export interface AppSetting {
  key: string;
  value: string;
  updated_at: string;
}

export interface SystemInfo {
  engine_version: string;
  database_path: string;
  storage_root: string;
  rust_version: string;
  target_os: string;
}

export type NavigationDestination =
  | "overview"
  | "workspace"
  | "extractors"
  | "firmware-lab"
  | "binary-studio"
  | "device-profiles"
  | "firmware-library"
  | "operation-history"
  | "settings";

export interface FlashRegion {
  id: number;
  region_type: string;
  name: string;
  base: number;
  limit: number;
  size: number;
  enabled: boolean;
  sha256: string | null;
}

export interface Gate1StructuralValidation {
  passed: boolean;
  has_valid_signature: boolean;
  signature_offset: number;
  signature_magic: string;
  region_table_in_bounds: boolean;
  boundary_conflicts: Array<{
    region1_name: string;
    region1_bounds: [number, number];
    region2_name: string;
    region2_bounds: [number, number];
    overlap_bytes: number;
  }>;
  issues: string[];
}

export interface Gate2PlatformCompatibility {
  passed: boolean;
  platform_family: string;
  descriptor_version: string;
  region_count: number;
  enabled_regions: string[];
  compatibility_notes: string[];
}

export interface Gate3ProgrammerCompatibility {
  passed: boolean;
  detected_chip_size_bytes: number;
  detected_chip_size_mb: number;
  external_spi_supported: boolean;
  host_flashing_locked: boolean;
  verification_constraints: string[];
}

export interface IfdAnalysisReport {
  image_size: number;
  regions: FlashRegion[];
  gate1_structural: Gate1StructuralValidation;
  gate2_platform: Gate2PlatformCompatibility;
  gate3_programmer: Gate3ProgrammerCompatibility;
}

export interface DellPfsExtractedComponent {
  index: number;
  name: string;
  version: string;
  guid: string;
  raw_size: number;
  sha256: string;
  md5: string;
  file_path: string | null;
  section_type: string;
  is_nested: boolean;
  classification: string;
  validation_notes: string[];
}

export interface DellPfsSectionInfo {
  section_type_byte: number;
  section_type_name: string;
  compressed_size: number;
  header_checksum_valid: boolean;
  footer_checksum_valid: boolean;
  decompressed_size: number;
  payload_inverted_crc32_valid: boolean;
  components_count: number;
}

export interface DellPfsExtractionReport {
  total_sections_found: number;
  sections: DellPfsSectionInfo[];
  components: DellPfsExtractedComponent[];
  model_ids: string | null;
  model_brand: string | null;
  model_name: string | null;
  issues: string[];
}
