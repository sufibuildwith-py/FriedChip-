import React, { useState } from "react";
import {
  ShieldCheck,
  FileSearch,
  Copy,
  Check,
  AlertCircle,
  HardDriveDownload,
  Info,
  X,
  FileCode,
  FolderOpen,
  RefreshCw,
  Search,
  Cpu,
} from "lucide-react";
import { FirmwareFile, Workspace } from "../types";
import { api } from "../api/tauriBridge";

interface WorkspaceViewProps {
  activeWorkspace: Workspace | null;
  files: FirmwareFile[];
  onFileImported: () => void;
}

export const WorkspaceView: React.FC<WorkspaceViewProps> = ({
  activeWorkspace,
  files,
  onFileImported,
}) => {
  const [importPath, setImportPath] = useState("");
  const [isImporting, setIsImporting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [copiedHash, setCopiedHash] = useState<string | null>(null);
  const [selectedFile, setSelectedFile] = useState<FirmwareFile | null>(null);
  const [verificationStatus, setVerificationStatus] = useState<string | null>(null);
  const [isVerifying, setIsVerifying] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");

  const handleBrowse = async () => {
    try {
      const path = await api.pickFirmwareFile();
      if (path) {
        setImportPath(path);
        setImportError(null);
      }
    } catch (err) {
      console.error("Browse failed:", err);
    }
  };

  const handleVerifySource = async (fileId: string) => {
    setIsVerifying(true);
    setVerificationStatus(null);
    try {
      await api.verifySourceFile(fileId);
      setVerificationStatus("SUCCESS: Source file verified on disk. SHA-256 checksum matches database record.");
    } catch (err: unknown) {
      setVerificationStatus(`FAILED: ${typeof err === "string" ? err : "Integrity check failed"}`);
    } finally {
      setIsVerifying(false);
    }
  };

  const handleImport = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!importPath.trim() || !activeWorkspace) return;

    setIsImporting(true);
    setImportError(null);

    try {
      await api.importFirmwareFile(activeWorkspace.id, importPath.trim());
      setImportPath("");
      onFileImported();
    } catch (err: unknown) {
      setImportError(typeof err === "string" ? err : "Failed to import firmware file.");
    } finally {
      setIsImporting(false);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedHash(text);
    setTimeout(() => setCopiedHash(null), 2000);
  };

  const filteredFiles = files.filter(
    (f) =>
      f.filename.toLowerCase().includes(searchTerm.toLowerCase()) ||
      f.sha256.toLowerCase().includes(searchTerm.toLowerCase()) ||
      f.detected_format.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="content-body">
      {/* Safe Ingestion Box */}
      <div className="card">
        <div className="card-title">
          <span>Safe Firmware Ingestion</span>
          <span style={{ fontSize: "11px", color: "var(--accent-emerald)", display: "flex", alignItems: "center", gap: "5px" }}>
            <ShieldCheck size={13} /> Non-Destructive Ingestion
          </span>
        </div>

        <p style={{ color: "var(--text-muted)", fontSize: "12px", marginBottom: "14px" }}>
          Select or specify a laptop firmware image or OEM update executable.
          The engine computes the primary SHA-256 checksum in a read-only streaming buffer and parses structural signatures.
        </p>

        <form onSubmit={handleImport} style={{ display: "flex", gap: "10px", flexWrap: "wrap" }}>
          <button
            type="button"
            className="btn btn-secondary"
            onClick={handleBrowse}
            disabled={isImporting}
          >
            <FolderOpen size={14} color="var(--accent-primary)" /> Browse File...
          </button>
          <input
            type="text"
            className="input-text"
            placeholder="C:\Path\To\firmware_dump.bin or select using Browse..."
            value={importPath}
            onChange={(e) => setImportPath(e.target.value)}
            disabled={isImporting}
          />
          <button type="submit" className="btn btn-primary" disabled={isImporting || !importPath.trim()}>
            <HardDriveDownload size={14} />
            {isImporting ? "Inspecting & Ingesting..." : "Ingest Dump"}
          </button>
        </form>

        {importError && (
          <div style={{ marginTop: "12px", padding: "10px 14px", background: "var(--accent-red-subtle)", border: "1px solid rgba(239,68,68,0.25)", borderRadius: "var(--radius-md)", color: "var(--accent-red)", fontSize: "12px", display: "flex", alignItems: "center", gap: "8px" }}>
            <AlertCircle size={15} />
            <span>{importError}</span>
          </div>
        )}
      </div>

      {/* Files Table Section */}
      <div className="table-container">
        <div className="table-header-bar">
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <span className="table-header-title">Ingested Firmware Dumps ({files.length})</span>
            <span style={{ fontSize: "11px", color: "var(--text-dim)", fontFamily: "var(--font-mono)" }}>
              Stored in SQLite metadata journal
            </span>
          </div>

          <div style={{ position: "relative", width: "260px" }}>
            <Search size={13} style={{ position: "absolute", left: "10px", top: "50%", transform: "translateY(-50%)", color: "var(--text-dim)" }} />
            <input
              type="text"
              placeholder="Search by name, hash, or format..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              style={{
                width: "100%",
                padding: "6px 10px 6px 30px",
                fontSize: "11.5px",
                borderRadius: "var(--radius-sm)",
                border: "1px solid var(--border-subtle)",
                background: "var(--bg-subtle)",
                color: "var(--text-main)",
                outline: "none",
              }}
            />
          </div>
        </div>

        {files.length === 0 ? (
          <div className="empty-box">
            <FileSearch size={32} className="empty-icon" />
            <div style={{ fontWeight: 600, color: "var(--text-main)" }}>No firmware dumps in this workspace yet.</div>
            <div style={{ fontSize: "11.5px", marginTop: "4px" }}>
              Click "Browse File..." above to safely ingest and hash your first SPI dump.
            </div>
          </div>
        ) : filteredFiles.length === 0 ? (
          <div className="empty-box">
            <Search size={24} className="empty-icon" />
            <div>No matching firmware files found for "{searchTerm}".</div>
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Artifact Name</th>
                <th>Capacity / Size</th>
                <th>Primary SHA-256</th>
                <th>Detected Format</th>
                <th>Confidence</th>
                <th>Ingested At</th>
                <th style={{ textAlign: "right" }}>Action</th>
              </tr>
            </thead>
            <tbody>
              {filteredFiles.map((file) => (
                <tr key={file.id}>
                  <td>
                    <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                      <div style={{
                        width: "30px",
                        height: "30px",
                        borderRadius: "6px",
                        background: "var(--bg-subtle)",
                        border: "1px solid var(--border-subtle)",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        color: "var(--accent-primary)"
                      }}>
                        <Cpu size={15} />
                      </div>
                      <div>
                        <div style={{ fontWeight: 600, color: "var(--text-main)" }}>{file.filename}</div>
                        <div style={{ fontSize: "10.5px", color: "var(--text-dim)" }}>
                          .{file.extension || "bin"} &bull; {file.is_read_only ? "Source Read-Only" : "Preserved"}
                        </div>
                      </div>
                    </div>
                  </td>
                  <td style={{ fontFamily: "var(--font-mono)", fontSize: "11.5px" }}>
                    {(file.size_bytes / 1_048_576).toFixed(2)} MB
                    <span style={{ color: "var(--text-dim)", marginLeft: "4px", fontSize: "10px" }}>
                      ({file.size_bytes.toLocaleString()} B)
                    </span>
                  </td>
                  <td>
                    <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
                      <span className="mono-hash">{file.sha256.substring(0, 16)}...</span>
                      <button
                        className="btn btn-secondary"
                        style={{ padding: "2px 6px" }}
                        title="Copy SHA-256"
                        onClick={() => copyToClipboard(file.sha256)}
                      >
                        {copiedHash === file.sha256 ? <Check size={11} color="var(--accent-emerald)" /> : <Copy size={11} />}
                      </button>
                    </div>
                  </td>
                  <td>
                    <span style={{ fontWeight: 600, fontSize: "11.5px" }}>
                      {file.detected_format}
                    </span>
                  </td>
                  <td>
                    <span className={`badge ${file.format_confidence === "Confirmed" ? "badge-confirmed" : "badge-uncertain"}`}>
                      {file.format_confidence}
                    </span>
                  </td>
                  <td style={{ fontSize: "11px", color: "var(--text-muted)" }}>
                    {new Date(file.import_timestamp).toLocaleDateString()} {new Date(file.import_timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                  </td>
                  <td style={{ textAlign: "right" }}>
                    <button
                      className="btn btn-secondary"
                      style={{ padding: "4px 9px", fontSize: "11px" }}
                      onClick={() => setSelectedFile(file)}
                    >
                      <Info size={12} /> Inspect
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Detail Drawer Modal */}
      {selectedFile && (
        <div className="modal-backdrop" onClick={() => { setSelectedFile(null); setVerificationStatus(null); }}>
          <div className="modal-window" onClick={(e) => e.stopPropagation()}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "18px", borderBottom: "1px solid var(--border-subtle)", paddingBottom: "14px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                <FileCode size={20} color="var(--accent-primary)" />
                <div>
                  <h3 style={{ fontSize: "15px", fontWeight: 700, color: "var(--text-main)" }}>Firmware Artifact Inspector</h3>
                  <div style={{ fontSize: "11px", color: "var(--text-dim)" }}>FriedChip+ Safe Analysis Engine</div>
                </div>
              </div>
              <button
                className="btn btn-outline"
                style={{ padding: "4px 8px" }}
                onClick={() => { setSelectedFile(null); setVerificationStatus(null); }}
              >
                <X size={15} />
              </button>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: "14px" }}>
              <div>
                <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>Filename</div>
                <div style={{ fontWeight: 600, fontSize: "13px", color: "var(--text-main)", marginTop: "2px" }}>{selectedFile.filename}</div>
              </div>

              <div>
                <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>Original File Path (Immutable Source)</div>
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "11.5px", color: "var(--text-muted)", wordBreak: "break-all", marginTop: "2px" }}>
                  {selectedFile.original_path}
                </div>
              </div>

              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "14px" }}>
                <div>
                  <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>File Size</div>
                  <div style={{ fontFamily: "var(--font-mono)", fontSize: "12px", color: "var(--text-main)", marginTop: "2px" }}>
                    {selectedFile.size_bytes.toLocaleString()} bytes ({(selectedFile.size_bytes / 1_048_576).toFixed(4)} MB)
                  </div>
                </div>

                <div>
                  <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>Signature Classification</div>
                  <div style={{ display: "flex", alignItems: "center", gap: "6px", marginTop: "2px" }}>
                    <span style={{ fontWeight: 600 }}>{selectedFile.detected_format}</span>
                    <span className={`badge ${selectedFile.format_confidence === "Confirmed" ? "badge-confirmed" : "badge-uncertain"}`}>
                      {selectedFile.format_confidence}
                    </span>
                  </div>
                </div>
              </div>

              <div>
                <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>Primary Cryptographic Hash (SHA-256)</div>
                <div style={{ display: "flex", alignItems: "center", gap: "8px", marginTop: "4px" }}>
                  <code style={{ background: "var(--bg-subtle)", padding: "5px 10px", borderRadius: "var(--radius-sm)", fontSize: "11.5px", color: "var(--accent-primary)", flex: 1, wordBreak: "break-all", border: "1px solid var(--border-subtle)" }}>
                    {selectedFile.sha256}
                  </code>
                  <button className="btn btn-secondary" onClick={() => copyToClipboard(selectedFile.sha256)} title="Copy SHA-256">
                    {copiedHash === selectedFile.sha256 ? <Check size={12} color="var(--accent-emerald)" /> : <Copy size={12} />}
                  </button>
                </div>
              </div>

              <div>
                <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>Legacy Compatibility Hash (MD5)</div>
                <code style={{ background: "var(--bg-subtle)", padding: "5px 10px", borderRadius: "var(--radius-sm)", fontSize: "11.5px", color: "var(--text-muted)", display: "block", marginTop: "4px", border: "1px solid var(--border-subtle)" }}>
                  {selectedFile.md5}
                </code>
              </div>

              <div style={{ padding: "12px 14px", background: "var(--accent-emerald-subtle)", border: "1px solid rgba(16,185,129,0.25)", borderRadius: "var(--radius-md)", fontSize: "11.5px", color: "var(--text-main)", display: "flex", alignItems: "center", justifyContent: "space-between", gap: "10px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <ShieldCheck size={16} color="var(--accent-emerald)" />
                  <span>Streaming source validation: verifies on-disk bytes against database record.</span>
                </div>
                <button
                  type="button"
                  className="btn btn-secondary"
                  style={{ padding: "4px 10px", fontSize: "11px" }}
                  onClick={() => handleVerifySource(selectedFile.id)}
                  disabled={isVerifying}
                >
                  <RefreshCw size={11} className={isVerifying ? "animate-spin" : ""} />
                  {isVerifying ? "Verifying..." : "Verify On-Disk Integrity"}
                </button>
              </div>

              {verificationStatus && (
                <div style={{
                  padding: "10px 14px",
                  borderRadius: "var(--radius-md)",
                  fontSize: "11.5px",
                  fontFamily: "var(--font-mono)",
                  background: verificationStatus.startsWith("SUCCESS") ? "var(--accent-emerald-subtle)" : "var(--accent-red-subtle)",
                  color: verificationStatus.startsWith("SUCCESS") ? "var(--accent-emerald)" : "var(--accent-red)",
                  border: `1px solid ${verificationStatus.startsWith("SUCCESS") ? "rgba(16,185,129,0.3)" : "rgba(239,68,68,0.3)"}`
                }}>
                  {verificationStatus}
                </div>
              )}
            </div>

            <div style={{ marginTop: "22px", display: "flex", justifyContent: "flex-end" }}>
              <button className="btn btn-secondary" onClick={() => { setSelectedFile(null); setVerificationStatus(null); }}>
                Close Inspector
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
