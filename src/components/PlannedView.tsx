import React from "react";
import {
  Clock,
  Unplug,
  FlaskConical,
  Binary,
  Layers,
  Database,
  ArrowRight,
  ShieldCheck,
  CheckCircle2,
} from "lucide-react";
import { NavigationDestination } from "../types";

interface PlannedViewProps {
  view: NavigationDestination;
  onNavigateToWorkspace: () => void;
}

export const PlannedView: React.FC<PlannedViewProps> = ({
  view,
  onNavigateToWorkspace,
}) => {
  const getDetails = () => {
    switch (view) {
      case "extractors":
        return {
          title: "Manufacturer Firmware Extractors",
          icon: <Unplug size={36} color="var(--accent-primary)" />,
          targetPhase: "Phase 6",
          scope:
            "Unpacking OEM update payloads into programmer-ready raw SPI images and EC binaries.",
          capabilities: [
            "Dell PFS container decompression & HDR payload separation",
            "HP Insyde encrypted payload unpacker & key discovery",
            "Lenovo ThinkPad FL1 / FL2 / Inno extractor",
            "Asus .CAP UEFI capsule header stripper",
            "Acer, Toshiba, Samsung, Sony, Fujitsu & Panasonic format profiles",
          ],
          prerequisite:
            "Requires candidate reference test packages (Dell Latitude 5490, HP EliteBook 840 G5, Lenovo ThinkPad T480).",
        };
      case "firmware-lab":
        return {
          title: "Firmware Lab & Intel ME/CSME Diagnostics",
          icon: <FlaskConical size={36} color="var(--accent-primary)" />,
          targetPhase: "Phase 4",
          scope:
            "Intel Flash Descriptor (IFD) decoding, Flash Partition Table (FPT) inspection, ME/CSME manifest parsing, and verified repository injection.",
          capabilities: [
            "Intel IFD region boundary and access permissions parser",
            "Flash Partition Table (FPT) entry inspection and length validation",
            "Intel ME 7.x through CSME 16.x SKU & version detection",
            "Safe Clean ME injection with pre-replacement compatibility checks",
            "Verification against external SPI programming constraints",
          ],
          prerequisite:
            "Requires clean ME repository blobs and platform compatibility matrices.",
        };
      case "binary-studio":
        return {
          title: "Binary Studio & Virtual Hex Inspector",
          icon: <Binary size={36} color="var(--accent-primary)" />,
          targetPhase: "Phase 2",
          scope:
            "High-performance 60fps virtualized 64MB+ hex inspector, byte-level diff visualizer between firmware revisions, multi-chip BIOS splitter/merger, and entropy analyzer.",
          capabilities: [
            "Virtualized 64MB+ buffer renderer with synchronized ASCII column",
            "Byte-level visual diff between stock and target dumps",
            "Multi-chip SPI splitter (e.g. 16MB + 8MB Dual BIOS pairs)",
            "Shannon entropy analyzer for encrypted/compressed region mapping",
            "Non-destructive slicing with atomic output writes",
          ],
          prerequisite: "Next milestone following Phase 1 acceptance.",
        };
      case "device-profiles":
        return {
          title: "Device & Microcontroller EC Profiles",
          icon: <Layers size={36} color="var(--accent-primary)" />,
          targetPhase: "Phase 5",
          scope:
            "Architecture-specific EC validation for ITE, ENE, Nuvoton, and Microchip embedded controllers.",
          capabilities: [
            "ITE IT8586E / IT8587E microcode vector table validation",
            "ENE KB9012 / KB9022 architecture profiles",
            "Nuvoton NPCE series firmware block boundary checkers",
            "Microchip MEC series image size & checksum heuristics",
            "Automatic fallback to UNCLASSIFIED on uncertain boundaries",
          ],
          prerequisite: "Requires validated architectural golden dumps.",
        };
      case "firmware-library":
        return {
          title: "Local Reference Firmware Library",
          icon: <Database size={36} color="var(--accent-primary)" />,
          targetPhase: "Phase 8",
          scope:
            "Offline searchable repository indexing verified clean ME builds, stock EC dumps, board schematics references, and diagnostic reports.",
          capabilities: [
            "Local filesystem catalog with metadata indexing",
            "Cryptographic signature search across local reference stores",
            "Motherboard part-number and chassis cross-referencing",
            "Offline operation without cloud or third-party dependencies",
          ],
          prerequisite: "Requires local filesystem indexing and bulk ingestion pipelines.",
        };
      default:
        return {
          title: "Planned Module",
          icon: <Clock size={36} color="var(--accent-primary)" />,
          targetPhase: "Future Milestone",
          scope: "Planned capability according to the FriedChip+ engineering roadmap.",
          capabilities: ["Implementation scheduled in subsequent phases"],
          prerequisite: "Pending roadmap milestone.",
        };
    }
  };

  const details = getDetails();

  return (
    <div className="content-body" style={{ display: "flex", alignItems: "center", justifyContent: "center" }}>
      <div
        className="card"
        style={{
          maxWidth: "680px",
          width: "100%",
          padding: "36px 32px",
          boxShadow: "var(--shadow-md)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "center", marginBottom: "16px" }}>
          <div style={{
            width: "64px",
            height: "64px",
            borderRadius: "16px",
            background: "var(--bg-subtle)",
            border: "1px solid var(--border-subtle)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
          }}>
            {details.icon}
          </div>
        </div>

        <div style={{ textAlign: "center", marginBottom: "20px" }}>
          <div style={{ display: "inline-flex", alignItems: "center", gap: "6px", marginBottom: "8px" }}>
            <span className="badge badge-planned" style={{ fontSize: "11px", padding: "3px 8px" }}>
              {details.targetPhase} &bull; Roadmap Milestone
            </span>
          </div>
          <h2 style={{ fontSize: "20px", fontWeight: 700, color: "var(--text-main)", letterSpacing: "-0.02em" }}>
            {details.title}
          </h2>
          <p style={{ color: "var(--text-muted)", fontSize: "13px", marginTop: "8px", maxWidth: "540px", margin: "8px auto 0" }}>
            {details.scope}
          </p>
        </div>

        <div style={{
          background: "var(--bg-subtle)",
          border: "1px solid var(--border-subtle)",
          borderRadius: "var(--radius-lg)",
          padding: "18px 20px",
          textAlign: "left",
          marginBottom: "20px",
        }}>
          <div style={{ fontSize: "11px", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.05em", color: "var(--text-dim)", marginBottom: "10px" }}>
            Planned Module Capabilities
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
            {details.capabilities.map((cap, i) => (
              <div key={i} style={{ display: "flex", alignItems: "flex-start", gap: "8px", fontSize: "12px", color: "var(--text-main)" }}>
                <CheckCircle2 size={14} color="var(--accent-primary)" style={{ flexShrink: 0, marginTop: "2px" }} />
                <span>{cap}</span>
              </div>
            ))}
          </div>

          <div style={{ marginTop: "14px", paddingTop: "12px", borderTop: "1px solid var(--border-subtle)", fontSize: "11.5px", color: "var(--text-dim)", display: "flex", alignItems: "center", gap: "6px" }}>
            <ShieldCheck size={14} color="var(--accent-amber)" />
            <span><strong>Prerequisite:</strong> {details.prerequisite}</span>
          </div>
        </div>

        <div style={{ display: "flex", justifyContent: "center" }}>
          <button className="btn btn-secondary" onClick={onNavigateToWorkspace}>
            Return to Active Workspace <ArrowRight size={14} />
          </button>
        </div>
      </div>
    </div>
  );
};
