import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { AppSetting, DellPfsExtractionReport, FirmwareFile, IfdAnalysisReport, Operation, OperationLog, SystemInfo, Workspace } from "../types";

const isTauri = typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

// In-memory fallback state for browser preview / headless capture
const previewWorkspace: Workspace = {
  id: "ed717331-ceb1-433c-9917-2dbff7f9aafd",
  name: "Default Workspace",
  root_path: "C:\\Users\\xtrar\\AppData\\Roaming\\com.friedchip.workstation",
  created_at: new Date().toISOString(),
  updated_at: new Date().toISOString(),
  is_default: true,
};

const previewFiles: FirmwareFile[] = [
  {
    id: "f1a2b3c4-0001-4000-8000-000000000001",
    workspace_id: "ed717331-ceb1-433c-9917-2dbff7f9aafd",
    original_path: "C:\\FirmwareDumps\\Latitude_5490_SPI_Dump.bin",
    filename: "Latitude_5490_SPI_Dump.bin",
    extension: "bin",
    size_bytes: 16777216,
    sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
    md5: "5eb63bbbe01eeed093cb22bb8f5acdc3",
    detected_format: "IntelFlashDescriptor",
    format_confidence: "Confirmed",
    import_timestamp: new Date(Date.now() - 3600000).toISOString(),
    is_read_only: true,
  },
  {
    id: "f1a2b3c4-0002-4000-8000-000000000002",
    workspace_id: "ed717331-ceb1-433c-9917-2dbff7f9aafd",
    original_path: "C:\\FirmwareDumps\\EliteBook_840G5_Clean.bin",
    filename: "EliteBook_840G5_Clean.bin",
    extension: "bin",
    size_bytes: 16777216,
    sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    md5: "b10a8db164e0754105b7a99be72e3fe5",
    detected_format: "UefiFirmwareVolume",
    format_confidence: "Confirmed",
    import_timestamp: new Date(Date.now() - 7200000).toISOString(),
    is_read_only: true,
  },
  {
    id: "f1a2b3c4-0003-4000-8000-000000000003",
    workspace_id: "ed717331-ceb1-433c-9917-2dbff7f9aafd",
    original_path: "C:\\FirmwareDumps\\EC_Dump_IT8586E.bin",
    filename: "EC_Dump_IT8586E.bin",
    extension: "bin",
    size_bytes: 131072,
    sha256: "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e",
    md5: "827ccb0eea8a706c4c34a16891f84e7b",
    detected_format: "Unclassified",
    format_confidence: "Uncertain",
    import_timestamp: new Date(Date.now() - 10800000).toISOString(),
    is_read_only: true,
  },
];

const previewOperations: Operation[] = [
  {
    id: "op-001-import-latitude",
    workspace_id: "ed717331-ceb1-433c-9917-2dbff7f9aafd",
    operation_type: "FILE_IMPORT",
    input_file_id: "f1a2b3c4-0001-4000-8000-000000000001",
    input_sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
    status: "COMPLETED",
    created_at: new Date(Date.now() - 3600000).toISOString(),
    completed_at: new Date(Date.now() - 3598500).toISOString(),
    error_message: null,
  },
  {
    id: "op-002-import-elitebook",
    workspace_id: "ed717331-ceb1-433c-9917-2dbff7f9aafd",
    operation_type: "FILE_IMPORT",
    input_file_id: "f1a2b3c4-0002-4000-8000-000000000002",
    input_sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    status: "COMPLETED",
    created_at: new Date(Date.now() - 7200000).toISOString(),
    completed_at: new Date(Date.now() - 7198200).toISOString(),
    error_message: null,
  },
  {
    id: "op-003-import-ec-dump",
    workspace_id: "ed717331-ceb1-433c-9917-2dbff7f9aafd",
    operation_type: "FILE_IMPORT",
    input_file_id: "f1a2b3c4-0003-4000-8000-000000000003",
    input_sha256: "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e",
    status: "COMPLETED",
    created_at: new Date(Date.now() - 10800000).toISOString(),
    completed_at: new Date(Date.now() - 10798900).toISOString(),
    error_message: null,
  },
];

const previewLogs: Record<string, OperationLog[]> = {
  "op-001-import-latitude": [
    { id: 1, operation_id: "op-001-import-latitude", timestamp: new Date(Date.now() - 3600000).toISOString(), level: "INFO", message: "Initiating safe read-only import of file: C:\\FirmwareDumps\\Latitude_5490_SPI_Dump.bin" },
    { id: 2, operation_id: "op-001-import-latitude", timestamp: new Date(Date.now() - 3599500).toISOString(), level: "INFO", message: "Computed primary SHA-256: 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" },
    { id: 3, operation_id: "op-001-import-latitude", timestamp: new Date(Date.now() - 3599200).toISOString(), level: "INFO", message: "Computed legacy MD5: 5eb63bbbe01eeed093cb22bb8f5acdc3" },
    { id: 4, operation_id: "op-001-import-latitude", timestamp: new Date(Date.now() - 3599000).toISOString(), level: "INFO", message: "File size: 16777216 bytes (16.00 MB)" },
    { id: 5, operation_id: "op-001-import-latitude", timestamp: new Date(Date.now() - 3598800).toISOString(), level: "INFO", message: "Signature Analysis: Intel Flash Descriptor (SPI) [Confidence: CONFIRMED]" },
    { id: 6, operation_id: "op-001-import-latitude", timestamp: new Date(Date.now() - 3598500).toISOString(), level: "INFO", message: "Successfully registered firmware file in workspace metadata store (ID: f1a2b3c4-0001-4000-8000-000000000001)" },
  ],
};

export const api = {
  async getActiveWorkspace(): Promise<Workspace> {
    if (isTauri) return await invoke<Workspace>("get_active_workspace");
    return previewWorkspace;
  },

  async listWorkspaces(): Promise<Workspace[]> {
    if (isTauri) return await invoke<Workspace[]>("list_workspaces");
    return [previewWorkspace];
  },

  async createWorkspace(name: string, rootPath: string): Promise<Workspace> {
    if (isTauri) return await invoke<Workspace>("create_workspace", { name, rootPath });
    return {
      id: "ws-" + Math.random().toString(36).substring(7),
      name,
      root_path: rootPath,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      is_default: false,
    };
  },

  async importFirmwareFile(workspaceId: string, filePath: string): Promise<FirmwareFile> {
    if (isTauri) {
      return await invoke<FirmwareFile>("import_firmware_file", { workspaceId, filePath });
    }
    const newFile: FirmwareFile = {
      id: "f-" + Math.random().toString(36).substring(7),
      workspace_id: workspaceId,
      original_path: filePath,
      filename: filePath.split(/[/\\]/).pop() || "firmware.bin",
      extension: filePath.split(".").pop() || "bin",
      size_bytes: 16777216,
      sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      md5: "d41d8cd98f00b204e9800998ecf8427e",
      detected_format: "IntelFlashDescriptor",
      format_confidence: "Confirmed",
      import_timestamp: new Date().toISOString(),
      is_read_only: true,
    };
    previewFiles.unshift(newFile);
    return newFile;
  },

  async pickFirmwareFile(): Promise<string | null> {
    if (isTauri) {
      try {
        const selected = await open({
          multiple: false,
          directory: false,
          filters: [
            {
              name: "Firmware Files",
              extensions: ["bin", "rom", "fd", "cap", "hdr", "exe", "dump"],
            },
            {
              name: "All Files (*.*)",
              extensions: ["*"],
            },
          ],
        });
        if (Array.isArray(selected)) {
          return selected[0] || null;
        }
        return selected;
      } catch (err) {
        console.error("Native dialog error:", err);
        return null;
      }
    }
    return prompt("Enter firmware file path to ingest:", "C:\\FirmwareDumps\\Latitude_5490_SPI_Dump.bin");
  },

  async verifySourceFile(fileId: string): Promise<FirmwareFile> {
    if (isTauri) return await invoke<FirmwareFile>("verify_source_file", { fileId });
    return previewFiles.find((f) => f.id === fileId) || previewFiles[0];
  },

  async listWorkspaceFiles(workspaceId: string): Promise<FirmwareFile[]> {
    if (isTauri) return await invoke<FirmwareFile[]>("list_workspace_files", { workspaceId });
    return previewFiles;
  },

  async listOperations(workspaceId: string): Promise<Operation[]> {
    if (isTauri) return await invoke<Operation[]>("list_operations", { workspaceId });
    return previewOperations;
  },

  async getOperationLogs(operationId: string): Promise<OperationLog[]> {
    if (isTauri) return await invoke<OperationLog[]>("get_operation_logs", { operationId });
    return previewLogs[operationId] || [
      { id: 1, operation_id: operationId, timestamp: new Date().toISOString(), level: "INFO", message: "Operation recorded in database." }
    ];
  },

  async getSettings(): Promise<AppSetting[]> {
    if (isTauri) return await invoke<AppSetting[]>("get_settings");
    return [
      { key: "journal_mode", value: "WAL", updated_at: new Date().toISOString() },
      { key: "integrity_hash", value: "SHA-256", updated_at: new Date().toISOString() },
      { key: "source_preservation", value: "STRICT_READ_ONLY", updated_at: new Date().toISOString() },
    ];
  },

  async updateSetting(key: string, value: string): Promise<void> {
    if (isTauri) await invoke<void>("update_setting", { key, value });
  },

  async inspectFirmwareRegions(filePath: string): Promise<IfdAnalysisReport> {
    if (isTauri) return await invoke<IfdAnalysisReport>("inspect_firmware_regions", { filePath });
    return {
      image_size: 16777216,
      regions: [
        { id: 0, region_type: "descriptor", name: "Flash Descriptor", base: 0, limit: 4095, size: 4096, enabled: true, sha256: "a1b2c3d4e5f6..." },
        { id: 1, region_type: "bios", name: "BIOS / UEFI Region", base: 0x600000, limit: 0xFFFFFF, size: 0xA00000, enabled: true, sha256: "b2c3d4e5f6..." },
        { id: 2, region_type: "me", name: "Intel ME / CSME", base: 0x1000, limit: 0x5FFFFF, size: 0x5FF000, enabled: true, sha256: "c3d4e5f6..." },
      ],
      gate1_structural: {
        passed: true,
        has_valid_signature: true,
        signature_offset: 16,
        signature_magic: "0x0FF0A55A",
        region_table_in_bounds: true,
        boundary_conflicts: [],
        issues: [],
      },
      gate2_platform: {
        passed: true,
        platform_family: "Intel PCH (Skylake / Kaby Lake / Coffee Lake)",
        descriptor_version: "IFD v2.0",
        region_count: 5,
        enabled_regions: ["Descriptor (0x0..0xFFF)", "ME (0x1000..0x5FFFFF)", "BIOS (0x600000..0xFFFFFF)"],
        compatibility_notes: ["Intel Management Engine region present", "BIOS/UEFI primary payload region present"],
      },
      gate3_programmer: {
        passed: true,
        detected_chip_size_bytes: 16777216,
        detected_chip_size_mb: 16.0,
        external_spi_supported: true,
        host_flashing_locked: true,
        verification_constraints: [
          "Required SPI NOR flash capacity: 16 MB (0x1000000 bytes)",
          "External hardware programmer (CH341A / Dediprog / Flashrom) supported.",
        ],
      },
    };
  },

  async isDellPfsPackage(filePath: string): Promise<boolean> {
    if (isTauri) return await invoke<boolean>("is_dell_pfs_package", { filePath });
    return filePath.toLowerCase().includes("dell") || filePath.toLowerCase().includes("latitude");
  },

  async extractDellPfs(workspaceId: string, fileId: string, customOutputDir?: string): Promise<DellPfsExtractionReport> {
    if (isTauri) {
      return await invoke<DellPfsExtractionReport>("extract_dell_pfs", {
        workspaceId,
        fileId,
        customOutputDir,
      });
    }
    return {
      total_sections_found: 2,
      sections: [
        {
          section_type_byte: 0xbb,
          section_type_name: "Utilities",
          compressed_size: 1337146,
          header_checksum_valid: true,
          footer_checksum_valid: true,
          decompressed_size: 1337146,
          payload_inverted_crc32_valid: true,
          components_count: 3,
        },
        {
          section_type_byte: 0xaa,
          section_type_name: "Firmware",
          compressed_size: 11200000,
          header_checksum_valid: true,
          footer_checksum_valid: true,
          decompressed_size: 16713066,
          payload_inverted_crc32_valid: true,
          components_count: 11,
        },
      ],
      components: [
        {
          index: 1,
          name: "System BIOS with BIOS Guard",
          version: "1.41.0.0",
          guid: "C6C5E7D17124F392406639BCF6BAA1AC",
          raw_size: 9001316,
          sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
          md5: "5eb63bbbe01eeed093cb22bb8f5acdc3",
          file_path: "C:\\Storage\\extracted\\1_System_BIOS_with_BIOS_Guard.bin",
          section_type: "Firmware",
          is_nested: false,
          classification: "Dell Extracted BIOS Payload (Unverified SPI Layout)",
          validation_notes: ["Extracted from section 'Firmware' at depth 0", "Integrity verified via SHA-256"],
        },
        {
          index: 2,
          name: "Embedded Controller",
          version: "1.0.51.0",
          guid: "E85C4BC79BBCFBB04F30C962B4CB2C6B",
          raw_size: 172864,
          sha256: "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e",
          md5: "827ccb0eea8a706c4c34a16891f84e7b",
          file_path: "C:\\Storage\\extracted\\2_Embedded_Controller.bin",
          section_type: "Firmware",
          is_nested: false,
          classification: "Embedded Controller Payload (Architecture Unclassified)",
          validation_notes: ["Extracted from section 'Firmware' at depth 0", "Integrity verified via SHA-256"],
        },
        {
          index: 3,
          name: "Intel Management Engine (VPro LP) Update",
          version: "11.8.97.4739",
          guid: "7902BBC9C72E978B4BA10686DC090EB9",
          raw_size: 7348224,
          sha256: "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
          md5: "b10a8db164e0754105b7a99be72e3fe5",
          file_path: "C:\\Storage\\extracted\\5_Intel_Management_Engine.bin",
          section_type: "Firmware",
          is_nested: false,
          classification: "Intel Management Engine Region Payload",
          validation_notes: ["Extracted from section 'Firmware' at depth 0", "Integrity verified via SHA-256"],
        },
      ],
      model_ids: "Latitude 5490, 5290, 5590",
      model_brand: "Dell Inc.",
      model_name: "Latitude",
      issues: [],
    };
  },

  async getSystemInfo(): Promise<SystemInfo> {
    if (isTauri) return await invoke<SystemInfo>("get_system_info");
    return {
      engine_version: "0.1.0",
      database_path: "C:\\Users\\xtrar\\AppData\\Roaming\\com.friedchip.workstation\\workstation.db",
      storage_root: "C:\\Users\\xtrar\\AppData\\Roaming\\com.friedchip.workstation",
      rust_version: "1.99.1 (MSVC)",
      target_os: "Windows x86_64",
    };
  },
};
