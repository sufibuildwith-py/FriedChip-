import React, { useEffect, useState } from "react";
import {
  ShieldCheck,
  Cpu,
  Layers,
  ArrowRight,
  Database,
  FileCode2,
  HardDriveDownload,
  Activity,
  Terminal,
  Sparkles,
} from "lucide-react";
import { FirmwareFile, Operation, SystemInfo, Workspace } from "../types";
import { api } from "../api/tauriBridge";
import { FriedChipLogo } from "./FriedChipLogo";

interface OverviewViewProps {
  activeWorkspace: Workspace | null;
  files: FirmwareFile[];
  operations: Operation[];
  onNavigate: (view: "workspace" | "operation-history") => void;
}

export const OverviewView: React.FC<OverviewViewProps> = ({
  activeWorkspace,
  files,
  operations,
  onNavigate,
}) => {
  const [sysInfo, setSysInfo] = useState<SystemInfo | null>(null);

  useEffect(() => {
    api.getSystemInfo().then(setSysInfo).catch(console.error);
  }, []);

  const confirmedFiles = files.filter((f) => f.format_confidence === "Confirmed");
  const unclassifiedFiles = files.filter((f) => f.format_confidence !== "Confirmed");
  const totalBytes = files.reduce((acc, f) => acc + f.size_bytes, 0);

  return (
    <div className="content-body">
      {/* Hero Welcome Banner */}
      <div className="hero-card">
        <div style={{ display: "flex", alignItems: "center", gap: "16px" }}>
          <FriedChipLogo size={42} variant="accent" />
          <div className="hero-text">
            <h2>
              FriedChip<span style={{ color: "var(--accent-primary)" }}>+</span>
              <span style={{ fontSize: "11px", fontWeight: 500, color: "var(--text-dim)", marginLeft: "4px" }}>
                Commercial Engineering Suite
              </span>
            </h2>
            <p>
              Independent laptop firmware engineering workstation. Precision SPI inspection,
              cryptographic validation, and non-destructive analysis running locally on your hardware.
            </p>
          </div>
        </div>

        <div className="hero-actions">
          <button className="btn btn-primary" onClick={() => onNavigate("workspace")}>
            <HardDriveDownload size={14} /> Ingest Dump
          </button>
          <button className="btn btn-secondary" onClick={() => onNavigate("operation-history")}>
            Audit Journal <ArrowRight size={14} />
          </button>
        </div>
      </div>

      {/* 4-Metric Bento Grid */}
      <div className="grid-4">
        <div className="card">
          <div className="card-title">
            <span>Registered Dumps</span>
            <FileCode2 size={15} color="var(--accent-primary)" />
          </div>
          <div className="metric-val">{files.length}</div>
          <div className="metric-sub">
            {(totalBytes / 1_048_576).toFixed(2)} MB in active workspace
          </div>
        </div>

        <div className="card">
          <div className="card-title">
            <span>Confirmed Signatures</span>
            <ShieldCheck size={15} color="var(--accent-emerald)" />
          </div>
          <div className="metric-val" style={{ color: "var(--accent-emerald)" }}>
            {confirmedFiles.length}
          </div>
          <div className="metric-sub">Validated Intel IFD / UEFI structures</div>
        </div>

        <div className="card">
          <div className="card-title">
            <span>Unclassified / Generic</span>
            <Layers size={15} color="var(--accent-amber)" />
          </div>
          <div className="metric-val" style={{ color: "var(--accent-amber)" }}>
            {unclassifiedFiles.length}
          </div>
          <div className="metric-sub">Awaiting architecture match</div>
        </div>

        <div className="card">
          <div className="card-title">
            <span>Audited Operations</span>
            <Activity size={15} color="var(--accent-purple)" />
          </div>
          <div className="metric-val">{operations.length}</div>
          <div className="metric-sub">Append-only SQLite WAL journal</div>
        </div>
      </div>

      {/* Architecture & Environment Bento Grid */}
      <div className="grid-2">
        <div className="card">
          <div className="card-title">
            <span>Engine Specifications</span>
            <Cpu size={15} color="var(--accent-primary)" />
          </div>
          <p style={{ color: "var(--text-muted)", fontSize: "12px", marginBottom: "16px" }}>
            FriedChip+ utilizes an offline-first Rust firmware processing core.
            All input source files remain strictly read-only and immutable.
          </p>

          <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", borderBottom: "1px solid var(--border-subtle)", paddingBottom: "7px" }}>
              <span style={{ color: "var(--text-muted)" }}>Host Framework</span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "11.5px", color: "var(--accent-primary)", fontWeight: 600 }}>Tauri 2 (Rust MSVC)</span>
            </div>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", borderBottom: "1px solid var(--border-subtle)", paddingBottom: "7px" }}>
              <span style={{ color: "var(--text-muted)" }}>Interface Engine</span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "11.5px", color: "var(--text-main)", fontWeight: 600 }}>React 19 &bull; TypeScript</span>
            </div>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", borderBottom: "1px solid var(--border-subtle)", paddingBottom: "7px" }}>
              <span style={{ color: "var(--text-muted)" }}>Storage Engine</span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "11.5px", color: "var(--accent-emerald)", fontWeight: 600 }}>SQLite WAL (Versioned Migrations)</span>
            </div>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <span style={{ color: "var(--text-muted)" }}>Primary Hash Algorithm</span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "11.5px", color: "var(--accent-emerald)", fontWeight: 600 }}>SHA-256 (64 KB Streaming Buffer)</span>
            </div>
          </div>
        </div>

        <div className="card">
          <div className="card-title">
            <span>Storage & Persistence Environment</span>
            <Database size={15} color="var(--accent-emerald)" />
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "12px", marginTop: "4px" }}>
            <div>
              <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>Active Workspace ID</div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "11.5px", color: "var(--text-main)", marginTop: "2px" }}>
                {activeWorkspace ? activeWorkspace.id : "Loading..."}
              </div>
            </div>

            <div>
              <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>SQLite Database Path</div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--text-muted)", wordBreak: "break-all", marginTop: "2px" }}>
                {sysInfo ? sysInfo.database_path : "Loading..."}
              </div>
            </div>

            <div>
              <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>Managed Filesystem Root</div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--text-muted)", wordBreak: "break-all", marginTop: "2px" }}>
                {sysInfo ? sysInfo.storage_root : "Loading..."}
              </div>
            </div>

            <div>
              <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>Non-Destructive Invariant</div>
              <div style={{ color: "var(--accent-emerald)", fontSize: "11.5px", display: "flex", alignItems: "center", gap: "6px", marginTop: "3px" }}>
                <ShieldCheck size={14} /> Sources strictly protected; outputs written atomically with overwrite rejection.
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Recent Activity Table */}
      <div className="table-container">
        <div className="table-header-bar">
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <Terminal size={15} color="var(--accent-primary)" />
            <span className="table-header-title">Recent Operation Activity</span>
          </div>
          <span style={{ fontSize: "11px", color: "var(--text-dim)", fontFamily: "var(--font-mono)" }}>
            Showing latest {Math.min(5, operations.length)} events
          </span>
        </div>

        {operations.length === 0 ? (
          <div className="empty-box">
            <Sparkles size={28} className="empty-icon" />
            <div>No operations recorded in this session.</div>
            <div style={{ fontSize: "11.5px", marginTop: "4px" }}>
              Import a firmware file in the Workspace to begin safe inspection.
            </div>
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Operation Type</th>
                <th>Operation ID</th>
                <th>Status</th>
                <th>Target SHA-256</th>
                <th>Timestamp</th>
              </tr>
            </thead>
            <tbody>
              {operations.slice(0, 5).map((op) => (
                <tr key={op.id}>
                  <td style={{ fontWeight: 600, color: "var(--text-main)" }}>{op.operation_type}</td>
                  <td className="mono-hash">{op.id.substring(0, 8)}...</td>
                  <td>
                    <span className={`badge ${op.status === "COMPLETED" ? "badge-completed" : "badge-failed"}`}>
                      {op.status}
                    </span>
                  </td>
                  <td className="mono-hash">
                    {op.input_sha256 ? `${op.input_sha256.substring(0, 16)}...` : "—"}
                  </td>
                  <td style={{ color: "var(--text-muted)", fontSize: "11px" }}>
                    {new Date(op.created_at).toLocaleString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
};
