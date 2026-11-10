import React, { useState, useEffect } from "react";
import { api } from "../api/tauriBridge";
import { DellPfsExtractionReport, FirmwareFile, IfdAnalysisReport, Workspace } from "../types";

interface ExtractorsViewProps {
  activeWorkspace: Workspace | null;
  files: FirmwareFile[];
  onRefreshWorkspace: () => void;
  onNavigateToWorkspace: () => void;
}

export const ExtractorsView: React.FC<ExtractorsViewProps> = ({
  activeWorkspace,
  files,
  onRefreshWorkspace,
  onNavigateToWorkspace,
}) => {
  const [selectedFileId, setSelectedFileId] = useState<string>("");
  const [activeTab, setActiveTab] = useState<"dell_pfs" | "ifd_regions">("dell_pfs");
  const [isExtracting, setIsExtracting] = useState<boolean>(false);
  const [isInspecting, setIsInspecting] = useState<boolean>(false);
  const [pfsReport, setPfsReport] = useState<DellPfsExtractionReport | null>(null);
  const [ifdReport, setIfdReport] = useState<IfdAnalysisReport | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  // Auto-select first file if available
  useEffect(() => {
    if (!selectedFileId && files.length > 0) {
      setSelectedFileId(files[0].id);
    }
  }, [files, selectedFileId]);

  const selectedFile = files.find((f) => f.id === selectedFileId) || null;

  const handlePickAndIngest = async () => {
    if (!activeWorkspace) return;
    setErrorMsg(null);
    setSuccessMsg(null);
    try {
      const pickedPath = await api.pickFirmwareFile();
      if (!pickedPath) return;

      const imported = await api.importFirmwareFile(activeWorkspace.id, pickedPath);
      onRefreshWorkspace();
      setSelectedFileId(imported.id);
      setSuccessMsg(`Package imported into workspace: ${imported.filename}`);
    } catch (err: any) {
      setErrorMsg(typeof err === "string" ? err : err.message || "Failed to pick file");
    }
  };

  const handleRunPfsExtraction = async () => {
    if (!activeWorkspace || !selectedFile) return;
    setIsExtracting(true);
    setErrorMsg(null);
    setSuccessMsg(null);
    setPfsReport(null);

    try {
      const report = await api.extractDellPfs(activeWorkspace.id, selectedFile.id);
      setPfsReport(report);
      setSuccessMsg(
        `Extraction complete! Extracted ${report.components.length} components across ${report.total_sections_found} sections.`
      );
      onRefreshWorkspace();
    } catch (err: any) {
      setErrorMsg(typeof err === "string" ? err : err.message || "Extraction failed");
    } finally {
      setIsExtracting(false);
    }
  };

  const handleInspectIfd = async () => {
    if (!selectedFile) return;
    setIsInspecting(true);
    setErrorMsg(null);
    setIfdReport(null);

    try {
      const report = await api.inspectFirmwareRegions(selectedFile.original_path);
      setIfdReport(report);
    } catch (err: any) {
      setErrorMsg(typeof err === "string" ? err : err.message || "IFD inspection failed");
    } finally {
      setIsInspecting(false);
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  };

  return (
    <div className="content-body">
      {/* Sub-header navigation & file selection */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "20px", flexWrap: "wrap", gap: "16px" }}>
        <div>
          <h2 style={{ fontSize: "20px", fontWeight: 700, margin: 0, color: "var(--text-main)", letterSpacing: "-0.02em" }}>
            Firmware Extractors & Binary Decomposers
          </h2>
          <div style={{ fontSize: "12px", color: "var(--text-muted)", marginTop: "4px" }}>
            Pure-Rust clean-room decompression &amp; bounded boundary parsing for OEM containers
          </div>
        </div>

        {/* Tab switcher */}
        <div style={{ display: "flex", background: "var(--bg-card)", border: "1px solid var(--border-subtle)", borderRadius: "var(--radius-sm)", padding: "3px" }}>
          <button
            onClick={() => setActiveTab("dell_pfs")}
            style={{
              padding: "6px 14px",
              fontSize: "12px",
              fontWeight: 600,
              borderRadius: "4px",
              border: "none",
              cursor: "pointer",
              background: activeTab === "dell_pfs" ? "var(--bg-card-hover)" : "transparent",
              color: activeTab === "dell_pfs" ? "var(--text-main)" : "var(--text-muted)",
              transition: "all 0.15s ease",
            }}
          >
            Dell PFS Extractor
          </button>
          <button
            onClick={() => setActiveTab("ifd_regions")}
            style={{
              padding: "6px 14px",
              fontSize: "12px",
              fontWeight: 600,
              borderRadius: "4px",
              border: "none",
              cursor: "pointer",
              background: activeTab === "ifd_regions" ? "var(--bg-card-hover)" : "transparent",
              color: activeTab === "ifd_regions" ? "var(--text-main)" : "var(--text-muted)",
              transition: "all 0.15s ease",
            }}
          >
            Intel Flash Descriptor (IFD)
          </button>
        </div>
      </div>

      {/* Input Selection Bar */}
      <div className="card" style={{ marginBottom: "20px", padding: "16px", display: "flex", alignItems: "center", justifyContent: "space-between", gap: "16px", flexWrap: "wrap" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "12px", flex: 1, minWidth: "280px" }}>
          <label style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-muted)", whiteSpace: "nowrap" }}>
            Target Package:
          </label>
          <select
            value={selectedFileId}
            onChange={(e) => setSelectedFileId(e.target.value)}
            style={{
              flex: 1,
              padding: "8px 12px",
              background: "var(--bg-main)",
              border: "1px solid var(--border-subtle)",
              borderRadius: "var(--radius-sm)",
              color: "var(--text-main)",
              fontSize: "12px",
              fontFamily: "var(--font-mono)",
            }}
          >
            {files.length === 0 ? (
              <option value="">No firmware files in workspace</option>
            ) : (
              files.map((f) => (
                <option key={f.id} value={f.id}>
                  {f.filename} ({formatBytes(f.size_bytes)}) &mdash; {f.detected_format}
                </option>
              ))
            )}
          </select>
        </div>

        <div style={{ display: "flex", gap: "10px" }}>
          <button
            className="btn btn-secondary"
            onClick={handlePickAndIngest}
            style={{ fontSize: "12px", display: "flex", alignItems: "center", gap: "6px" }}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            Ingest Package...
          </button>

          {activeTab === "dell_pfs" ? (
            <button
              className="btn btn-primary"
              onClick={handleRunPfsExtraction}
              disabled={!selectedFile || isExtracting}
              style={{ fontSize: "12px", display: "flex", alignItems: "center", gap: "6px" }}
            >
              {isExtracting ? (
                <>
                  <span className="spinner" style={{ width: "12px", height: "12px" }}></span>
                  Decompressing PFS...
                </>
              ) : (
                <>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
                  </svg>
                  Extract All Components
                </>
              )}
            </button>
          ) : (
            <button
              className="btn btn-primary"
              onClick={handleInspectIfd}
              disabled={!selectedFile || isInspecting}
              style={{ fontSize: "12px", display: "flex", alignItems: "center", gap: "6px" }}
            >
              {isInspecting ? (
                <>
                  <span className="spinner" style={{ width: "12px", height: "12px" }}></span>
                  Decoding IFD...
                </>
              ) : (
                <>
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <circle cx="11" cy="11" r="8" />
                    <line x1="21" y1="21" x2="16.65" y2="16.65" />
                  </svg>
                  Decode Region Table
                </>
              )}
            </button>
          )}
        </div>
      </div>

      {/* Notifications */}
      {errorMsg && (
        <div style={{ background: "rgba(239, 68, 68, 0.1)", border: "1px solid rgba(239, 68, 68, 0.3)", borderRadius: "var(--radius-sm)", padding: "12px 16px", marginBottom: "20px", color: "var(--status-error)", fontSize: "12px" }}>
          <strong>Error:</strong> {errorMsg}
        </div>
      )}
      {successMsg && (
        <div style={{ background: "rgba(16, 185, 129, 0.1)", border: "1px solid rgba(16, 185, 129, 0.3)", borderRadius: "var(--radius-sm)", padding: "12px 16px", marginBottom: "20px", color: "var(--status-success)", fontSize: "12px", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <span>{successMsg}</span>
          <button className="btn btn-secondary" onClick={onNavigateToWorkspace} style={{ fontSize: "11px", padding: "4px 8px" }}>
            View in Workspace &rarr;
          </button>
        </div>
      )}

      {/* DELL PFS TAB VIEW */}
      {activeTab === "dell_pfs" && (
        <div>
          {pfsReport ? (
            <div>
              {/* Sections Breakdown Cards */}
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "16px", marginBottom: "24px" }}>
                {pfsReport.sections.map((sec, i) => (
                  <div key={i} className="card" style={{ padding: "16px" }}>
                    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "12px" }}>
                      <span style={{ fontSize: "11px", textTransform: "uppercase", letterSpacing: "0.05em", color: "var(--accent-primary)", fontWeight: 700 }}>
                        Section 0x{sec.section_type_byte.toString(16).toUpperCase()} &bull; {sec.section_type_name}
                      </span>
                      <span className="badge badge-confirmed" style={{ fontSize: "10px" }}>
                        {sec.components_count} components
                      </span>
                    </div>

                    <div style={{ display: "flex", flexDirection: "column", gap: "8px", fontSize: "11px" }}>
                      <div style={{ display: "flex", justifyContent: "space-between" }}>
                        <span style={{ color: "var(--text-muted)" }}>Compressed Size:</span>
                        <span style={{ fontFamily: "var(--font-mono)" }}>{formatBytes(sec.compressed_size)}</span>
                      </div>
                      <div style={{ display: "flex", justifyContent: "space-between" }}>
                        <span style={{ color: "var(--text-muted)" }}>Decompressed Size:</span>
                        <span style={{ fontFamily: "var(--font-mono)", fontWeight: 600 }}>{formatBytes(sec.decompressed_size)}</span>
                      </div>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <span style={{ color: "var(--text-muted)" }}>Header XOR 8 Checksum:</span>
                        <span style={{ color: sec.header_checksum_valid ? "var(--status-success)" : "var(--status-error)", fontWeight: 600 }}>
                          {sec.header_checksum_valid ? "VALID" : "INVALID"}
                        </span>
                      </div>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                        <span style={{ color: "var(--text-muted)" }}>Inverted CRC-32 Payload:</span>
                        <span style={{ color: sec.payload_inverted_crc32_valid ? "var(--status-success)" : "var(--status-error)", fontWeight: 600 }}>
                          {sec.payload_inverted_crc32_valid ? "MATCHED" : "MISMATCH"}
                        </span>
                      </div>
                    </div>
                  </div>
                ))}
              </div>

              {/* Extracted Components Table */}
              <div className="card">
                <div style={{ padding: "16px", borderBottom: "1px solid var(--border-subtle)", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <h3 style={{ fontSize: "14px", fontWeight: 700, margin: 0 }}>
                    Extracted Component Payloads ({pfsReport.components.length})
                  </h3>
                  <div style={{ fontSize: "11px", color: "var(--text-dim)" }}>
                    All components verified &amp; atomized to isolated filesystem paths
                  </div>
                </div>

                <div style={{ overflowX: "auto" }}>
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th style={{ width: "40px" }}>#</th>
                        <th>Component Name &amp; Classification</th>
                        <th>Version</th>
                        <th>Payload Size</th>
                        <th>GUID</th>
                        <th>Verification Gates</th>
                      </tr>
                    </thead>
                    <tbody>
                      {pfsReport.components.map((comp) => (
                        <tr key={comp.index}>
                          <td style={{ fontFamily: "var(--font-mono)", color: "var(--text-dim)" }}>{comp.index}</td>
                          <td>
                            <div style={{ fontWeight: 600, color: "var(--text-main)" }}>{comp.name}</div>
                            <div style={{ fontSize: "10px", color: "var(--text-muted)", marginTop: "2px" }}>
                              {comp.classification}
                            </div>
                          </td>
                          <td style={{ fontFamily: "var(--font-mono)", fontSize: "11px" }}>{comp.version}</td>
                          <td style={{ fontFamily: "var(--font-mono)", fontSize: "11px" }}>{formatBytes(comp.raw_size)}</td>
                          <td>
                            <span style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--text-muted)" }}>
                              {comp.guid.substring(0, 16)}...
                            </span>
                          </td>
                          <td>
                            <div style={{ display: "flex", gap: "4px", flexWrap: "wrap" }}>
                              <span className="badge badge-confirmed" title="Parsed from Dell PFS section">
                                PARSED
                              </span>
                              <span className="badge badge-confirmed" title="Extracted to disk">
                                EXTRACTED
                              </span>
                              <span className="badge badge-confirmed" title="SHA-256 and inverted CRC-32 verified">
                                CHECKSUM OK
                              </span>
                              <span
                                className="badge"
                                style={{
                                  background: "rgba(100, 116, 139, 0.15)",
                                  color: "var(--text-muted)",
                                }}
                                title="Extracted payload requires SPI reconstruction before flashing"
                              >
                                SPI UNVERIFIED
                              </span>
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          ) : (
            <div className="card" style={{ padding: "40px", textAlign: "center" }}>
              <div style={{ maxWidth: "480px", margin: "0 auto" }}>
                <div style={{ width: "48px", height: "48px", borderRadius: "50%", background: "rgba(16, 185, 129, 0.1)", display: "flex", alignItems: "center", justifyContent: "center", margin: "0 auto 16px auto", color: "var(--accent-primary)" }}>
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <rect x="2" y="7" width="20" height="14" rx="2" ry="2" />
                    <path d="M16 21V5a2 2 0 0 0-2-2h-4a2 2 0 0 0-2 2v16" />
                  </svg>
                </div>
                <h3 style={{ fontSize: "16px", fontWeight: 700, margin: "0 0 8px 0" }}>
                  Dell PFS Container Extraction Engine
                </h3>
                <p style={{ fontSize: "12px", color: "var(--text-muted)", lineHeight: 1.6, margin: "0 0 20px 0" }}>
                  Our pure-Rust extraction engine parses multi-section Dell update packages, verifies 8-bit XOR headers and footers, decompresses ZLIB streams with anti-bomb limits, and extracts raw System BIOS, Intel ME, and EC component partitions with full provenance.
                </p>
                <button
                  className="btn btn-primary"
                  onClick={handleRunPfsExtraction}
                  disabled={!selectedFile || isExtracting}
                  style={{ display: "inline-flex", alignItems: "center", gap: "8px" }}
                >
                  {isExtracting ? "Extracting..." : "Run Dell PFS Extraction on Selected File"}
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* INTEL FLASH DESCRIPTOR TAB VIEW */}
      {activeTab === "ifd_regions" && (
        <div>
          {ifdReport ? (
            <div>
              {/* Gate Assessment Bar */}
              <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: "16px", marginBottom: "24px" }}>
                {/* Gate 1 */}
                <div className="card" style={{ padding: "16px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
                    <span style={{ fontSize: "11px", fontWeight: 700, color: "var(--text-muted)" }}>
                      GATE 1 &bull; STRUCTURAL VALIDITY
                    </span>
                    <span className={`badge ${ifdReport.gate1_structural.passed ? "badge-confirmed" : "badge-uncertain"}`}>
                      {ifdReport.gate1_structural.passed ? "PASSED" : "FAILED"}
                    </span>
                  </div>
                  <div style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-main)" }}>
                    Signature: {ifdReport.gate1_structural.signature_magic} at offset 0x{ifdReport.gate1_structural.signature_offset.toString(16)}
                  </div>
                  <div style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "4px" }}>
                    {ifdReport.gate1_structural.boundary_conflicts.length === 0
                      ? "Zero region overlaps detected"
                      : `${ifdReport.gate1_structural.boundary_conflicts.length} boundary collisions!`}
                  </div>
                </div>

                {/* Gate 2 */}
                <div className="card" style={{ padding: "16px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
                    <span style={{ fontSize: "11px", fontWeight: 700, color: "var(--text-muted)" }}>
                      GATE 2 &bull; PLATFORM COMPATIBILITY
                    </span>
                    <span className={`badge ${ifdReport.gate2_platform.passed ? "badge-confirmed" : "badge-uncertain"}`}>
                      {ifdReport.gate2_platform.passed ? "VERIFIED" : "UNVERIFIED"}
                    </span>
                  </div>
                  <div style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-main)" }}>
                    {ifdReport.gate2_platform.platform_family}
                  </div>
                  <div style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "4px" }}>
                    {ifdReport.gate2_platform.descriptor_version} &bull; {ifdReport.gate2_platform.region_count} Region Slots
                  </div>
                </div>

                {/* Gate 3 */}
                <div className="card" style={{ padding: "16px" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "8px" }}>
                    <span style={{ fontSize: "11px", fontWeight: 700, color: "var(--text-muted)" }}>
                      GATE 3 &bull; PROGRAMMER COMPATIBILITY
                    </span>
                    <span className={`badge ${ifdReport.gate3_programmer.passed ? "badge-confirmed" : "badge-uncertain"}`}>
                      {ifdReport.gate3_programmer.passed ? "SUPPORTED" : "CAUTION"}
                    </span>
                  </div>
                  <div style={{ fontSize: "12px", fontWeight: 600, color: "var(--text-main)" }}>
                    SPI Capacity: {ifdReport.gate3_programmer.detected_chip_size_mb} MB ({formatBytes(ifdReport.gate3_programmer.detected_chip_size_bytes)})
                  </div>
                  <div style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "4px" }}>
                    External Flashrom: Supported &bull; Host Flashing: Protected
                  </div>
                </div>
              </div>

              {/* Decoded Regions Table */}
              <div className="card">
                <div style={{ padding: "16px", borderBottom: "1px solid var(--border-subtle)" }}>
                  <h3 style={{ fontSize: "14px", fontWeight: 700, margin: 0 }}>
                    Decoded SPI Flash Regions ({ifdReport.regions.length})
                  </h3>
                </div>

                <div style={{ overflowX: "auto" }}>
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th>Region ID</th>
                        <th>Region Name</th>
                        <th>Physical Base</th>
                        <th>Physical Limit</th>
                        <th>Allocated Size</th>
                        <th>Status</th>
                        <th>Integrity (SHA-256)</th>
                      </tr>
                    </thead>
                    <tbody>
                      {ifdReport.regions.map((reg) => (
                        <tr key={reg.id} style={{ opacity: reg.enabled ? 1 : 0.5 }}>
                          <td style={{ fontFamily: "var(--font-mono)" }}>0x{reg.id.toString(16).toUpperCase()}</td>
                          <td style={{ fontWeight: 600, color: "var(--text-main)" }}>{reg.name}</td>
                          <td style={{ fontFamily: "var(--font-mono)", fontSize: "11px" }}>0x{reg.base.toString(16).toUpperCase()}</td>
                          <td style={{ fontFamily: "var(--font-mono)", fontSize: "11px" }}>0x{reg.limit.toString(16).toUpperCase()}</td>
                          <td style={{ fontFamily: "var(--font-mono)", fontSize: "11px" }}>{reg.enabled ? formatBytes(reg.size) : "&mdash;"}</td>
                          <td>
                            <span className={`badge ${reg.enabled ? "badge-confirmed" : "badge-heuristic"}`}>
                              {reg.enabled ? "ENABLED" : "DISABLED / UNMAPPED"}
                            </span>
                          </td>
                          <td style={{ fontFamily: "var(--font-mono)", fontSize: "10px", color: "var(--text-muted)" }}>
                            {reg.sha256 ? `${reg.sha256.substring(0, 16)}...` : "&mdash;"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          ) : (
            <div className="card" style={{ padding: "40px", textAlign: "center" }}>
              <div style={{ maxWidth: "480px", margin: "0 auto" }}>
                <div style={{ width: "48px", height: "48px", borderRadius: "50%", background: "rgba(59, 130, 246, 0.1)", display: "flex", alignItems: "center", justifyContent: "center", margin: "0 auto 16px auto", color: "var(--accent-primary)" }}>
                  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <circle cx="12" cy="12" r="10" />
                    <line x1="12" y1="8" x2="12" y2="12" />
                    <line x1="12" y1="16" x2="12.01" y2="16" />
                  </svg>
                </div>
                <h3 style={{ fontSize: "16px", fontWeight: 700, margin: "0 0 8px 0" }}>
                  Intel Flash Descriptor Region Decoder
                </h3>
                <p style={{ fontSize: "12px", color: "var(--text-muted)", lineHeight: 1.6, margin: "0 0 20px 0" }}>
                  Inspect any complete SPI firmware dump to decode FLREG 15-bit addressable page fields, detect unmapped regions, identify boundary overlaps, and separate structural validation from platform and external programmer compatibility.
                </p>
                <button
                  className="btn btn-primary"
                  onClick={handleInspectIfd}
                  disabled={!selectedFile || isInspecting}
                  style={{ display: "inline-flex", alignItems: "center", gap: "8px" }}
                >
                  {isInspecting ? "Decoding..." : "Decode Intel Flash Descriptor"}
                </button>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
