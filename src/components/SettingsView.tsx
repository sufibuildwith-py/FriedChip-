import React, { useEffect, useState } from "react";
import {
  Settings,
  ShieldCheck,
  Database,
  HardDrive,
  Sun,
  Moon,
  Monitor,
  Check,
  Copy,
  User,
  Heart,
  Sparkles,
  Award,
} from "lucide-react";
import { AppSetting, SystemInfo } from "../types";
import { api } from "../api/tauriBridge";
import { useTheme } from "../context/ThemeContext";
import { FriedChipLogo } from "./FriedChipLogo";

export const SettingsView: React.FC = () => {
  const [sysInfo, setSysInfo] = useState<SystemInfo | null>(null);
  const [settings, setSettings] = useState<AppSetting[]>([]);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const { themePref, setThemePref } = useTheme();

  useEffect(() => {
    api.getSystemInfo().then(setSysInfo).catch(console.error);
    api.getSettings().then(setSettings).catch(console.error);
  }, []);

  const copyPath = (key: string, text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  return (
    <div className="content-body">
      {/* Header Card */}
      <div className="card">
        <div className="card-title">
          <span>Workstation Configuration & Environment</span>
          <Settings size={15} color="var(--accent-primary)" />
        </div>
        <p style={{ color: "var(--text-muted)", fontSize: "12px" }}>
          FriedChip+ runtime parameters, persistent storage paths, and appearance settings.
        </p>
      </div>

      {/* Theme & Appearance Configuration */}
      <div className="card">
        <div className="card-title">
          <span>Visual Theme & Appearance</span>
          <Sparkles size={15} color="var(--accent-amber)" />
        </div>
        <p style={{ color: "var(--text-muted)", fontSize: "12px", marginBottom: "14px" }}>
          Configure the desktop appearance mode. Light mode is the default workstation aesthetic.
          Preferences persist automatically across restarts.
        </p>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: "12px" }}>
          <div
            onClick={() => setThemePref("light")}
            style={{
              padding: "16px",
              borderRadius: "var(--radius-lg)",
              border: `2px solid ${themePref === "light" ? "var(--accent-primary)" : "var(--border-subtle)"}`,
              background: "var(--bg-subtle)",
              cursor: "pointer",
              transition: "all var(--transition-fast)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "8px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <Sun size={18} color="var(--accent-amber)" />
                <span style={{ fontWeight: 600, fontSize: "13px", color: "var(--text-main)" }}>Light Mode</span>
              </div>
              <span className="badge badge-planned" style={{ fontSize: "9.5px" }}>Default</span>
            </div>
            <p style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: 1.4 }}>
              Warm off-white background, crisp white elevated surfaces, and dark graphite typography.
            </p>
          </div>

          <div
            onClick={() => setThemePref("dark")}
            style={{
              padding: "16px",
              borderRadius: "var(--radius-lg)",
              border: `2px solid ${themePref === "dark" ? "var(--accent-primary)" : "var(--border-subtle)"}`,
              background: "var(--bg-subtle)",
              cursor: "pointer",
              transition: "all var(--transition-fast)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "8px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <Moon size={18} color="var(--accent-primary)" />
                <span style={{ fontWeight: 600, fontSize: "13px", color: "var(--text-main)" }}>Dark Mode</span>
              </div>
            </div>
            <p style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: 1.4 }}>
              Deep charcoal background, subtle elevated surface colors, and luminous typography.
            </p>
          </div>

          <div
            onClick={() => setThemePref("system")}
            style={{
              padding: "16px",
              borderRadius: "var(--radius-lg)",
              border: `2px solid ${themePref === "system" ? "var(--accent-primary)" : "var(--border-subtle)"}`,
              background: "var(--bg-subtle)",
              cursor: "pointer",
              transition: "all var(--transition-fast)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "8px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <Monitor size={18} color="var(--accent-emerald)" />
                <span style={{ fontWeight: 600, fontSize: "13px", color: "var(--text-main)" }}>System Auto</span>
              </div>
            </div>
            <p style={{ fontSize: "11px", color: "var(--text-muted)", lineHeight: 1.4 }}>
              Synchronizes with your Windows display preference in real-time.
            </p>
          </div>
        </div>
      </div>

      {/* Grid: Storage Paths & Engine Info */}
      <div className="grid-2">
        <div className="card">
          <div className="card-title">
            <span>Storage & Persistence Locations</span>
            <Database size={15} color="var(--accent-emerald)" />
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "14px", marginTop: "4px" }}>
            <div>
              <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>SQLite Database Location</div>
              <div style={{ display: "flex", alignItems: "center", gap: "6px", marginTop: "4px" }}>
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--text-main)", wordBreak: "break-all", background: "var(--bg-subtle)", padding: "4px 8px", borderRadius: "var(--radius-sm)", flex: 1, border: "1px solid var(--border-subtle)" }}>
                  {sysInfo ? sysInfo.database_path : "Loading..."}
                </div>
                {sysInfo && (
                  <button className="btn btn-secondary" style={{ padding: "4px 8px" }} onClick={() => copyPath("db", sysInfo.database_path)}>
                    {copiedKey === "db" ? <Check size={12} color="var(--accent-emerald)" /> : <Copy size={12} />}
                  </button>
                )}
              </div>
            </div>

            <div>
              <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>Managed Filesystem Storage Root</div>
              <div style={{ display: "flex", alignItems: "center", gap: "6px", marginTop: "4px" }}>
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "11px", color: "var(--text-main)", wordBreak: "break-all", background: "var(--bg-subtle)", padding: "4px 8px", borderRadius: "var(--radius-sm)", flex: 1, border: "1px solid var(--border-subtle)" }}>
                  {sysInfo ? sysInfo.storage_root : "Loading..."}
                </div>
                {sysInfo && (
                  <button className="btn btn-secondary" style={{ padding: "4px 8px" }} onClick={() => copyPath("root", sysInfo.storage_root)}>
                    {copiedKey === "root" ? <Check size={12} color="var(--accent-emerald)" /> : <Copy size={12} />}
                  </button>
                )}
              </div>
            </div>

            <div>
              <div style={{ fontSize: "10px", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 700, letterSpacing: "0.05em" }}>SQLite Mode</div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "11.5px", color: "var(--accent-emerald)", marginTop: "2px" }}>
                Write-Ahead Logging (WAL) &bull; Foreign Keys Enforced
              </div>
            </div>
          </div>
        </div>

        <div className="card">
          <div className="card-title">
            <span>Host Engine Information</span>
            <HardDrive size={15} color="var(--accent-primary)" />
          </div>

          <div style={{ display: "flex", flexDirection: "column", gap: "10px", marginTop: "4px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", borderBottom: "1px solid var(--border-subtle)", paddingBottom: "7px" }}>
              <span style={{ color: "var(--text-muted)" }}>Engine Version</span>
              <span style={{ fontFamily: "var(--font-mono)", color: "var(--accent-primary)", fontWeight: 600 }}>v{sysInfo ? sysInfo.engine_version : "0.1.0"}</span>
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", borderBottom: "1px solid var(--border-subtle)", paddingBottom: "7px" }}>
              <span style={{ color: "var(--text-muted)" }}>Target Platform</span>
              <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-main)" }}>{sysInfo ? sysInfo.target_os : "Windows x86_64"}</span>
            </div>

            <div style={{ display: "flex", justifyContent: "space-between", borderBottom: "1px solid var(--border-subtle)", paddingBottom: "7px" }}>
              <span style={{ color: "var(--text-muted)" }}>Rust Compiler Toolchain</span>
              <span style={{ fontFamily: "var(--font-mono)", color: "var(--text-main)" }}>{sysInfo ? sysInfo.rust_version : "Rust stable MSVC"}</span>
            </div>

            <div style={{ display: "flex", justifyContent: "space-between" }}>
              <span style={{ color: "var(--text-muted)" }}>Clean-Room Compliance</span>
              <span style={{ color: "var(--accent-emerald)", display: "flex", alignItems: "center", gap: "5px", fontSize: "11.5px", fontWeight: 600 }}>
                <ShieldCheck size={14} /> 100% Independent Implementation
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Active Application Parameters */}
      <div className="card">
        <div className="card-title">
          <span>Active Application Parameters</span>
          <HardDrive size={15} color="var(--accent-primary)" />
        </div>

        {settings.length === 0 ? (
          <div style={{ color: "var(--text-dim)", fontSize: "12px", padding: "4px 0" }}>
            Default engineering parameters active. All pre-flight integrity assertions and atomic write guarantees enabled.
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
            {settings.map((s) => (
              <div key={s.key} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid var(--border-subtle)", paddingBottom: "6px", fontSize: "12px" }}>
                <span style={{ color: "var(--text-muted)", fontFamily: "var(--font-mono)", fontSize: "11px" }}>{s.key}</span>
                <span style={{ fontFamily: "var(--font-mono)", fontSize: "11.5px", color: "var(--text-main)", fontWeight: 600 }}>{s.value}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Creator & Brand Statement Card */}
      <div className="card" style={{
        background: "linear-gradient(135deg, var(--bg-card) 0%, var(--bg-subtle) 100%)",
        borderColor: "var(--border-strong)",
      }}>
        <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: "24px", flexWrap: "wrap" }}>
          <div style={{ display: "flex", alignItems: "flex-start", gap: "18px" }}>
            <FriedChipLogo size={52} variant="accent" />
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                <h3 style={{ fontSize: "18px", fontWeight: 700, color: "var(--text-main)", letterSpacing: "-0.02em" }}>
                  FriedChip<span style={{ color: "var(--accent-primary)" }}>+</span>
                </h3>
                <span className="badge badge-planned" style={{ fontSize: "10px" }}>Commercial Workstation</span>
              </div>
              <p style={{ color: "var(--text-muted)", fontSize: "12.5px", marginTop: "4px", maxWidth: "640px" }}>
                A modern, independent laptop BIOS and firmware engineering workstation.
                Designed for precision diagnostics, non-destructive SPI inspection, and commercial reliability.
              </p>
              <div style={{ marginTop: "12px", display: "flex", alignItems: "center", gap: "16px", flexWrap: "wrap", fontSize: "12px" }}>
                <div style={{ display: "flex", alignItems: "center", gap: "6px", color: "var(--text-main)", fontWeight: 600 }}>
                  <Award size={15} color="var(--accent-amber)" />
                  <span>Product Credit: Sufiyan Khan</span>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "6px", color: "var(--text-muted)" }}>
                  <Heart size={14} color="var(--accent-red)" />
                  <span>Crafted by Sufiyan Khan</span>
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: "6px", color: "var(--text-dim)", fontFamily: "var(--font-mono)", fontSize: "11px" }}>
                  <User size={13} />
                  <span>Developed by Sufiyan Khan</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
