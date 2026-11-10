import React from "react";
import { Folder, HardDrive, ShieldCheck, WifiOff } from "lucide-react";
import { NavigationDestination, Workspace } from "../types";

interface HeaderProps {
  currentView: NavigationDestination;
  activeWorkspace: Workspace | null;
}

export const Header: React.FC<HeaderProps> = ({ currentView, activeWorkspace }) => {
  const getTitle = () => {
    switch (currentView) {
      case "overview":
        return "Workstation Overview";
      case "workspace":
        return "Firmware Workspace";
      case "operation-history":
        return "Operation History & Audit Journal";
      case "extractors":
        return "Manufacturer Extractors";
      case "firmware-lab":
        return "Firmware Lab & Diagnostics";
      case "binary-studio":
        return "Binary Studio & Hex Inspector";
      case "device-profiles":
        return "Device Profiles & SIO Matrices";
      case "firmware-library":
        return "Local Reference Library";
      case "settings":
        return "Workstation Settings & About";
    }
  };

  return (
    <header className="header">
      <div className="header-left">
        <h2 className="view-title">{getTitle()}</h2>
        {activeWorkspace && (
          <div className="workspace-pill" title={`Active Workspace: ${activeWorkspace.root_path}`}>
            <Folder size={12} color="var(--accent-primary)" />
            <span>{activeWorkspace.name}</span>
          </div>
        )}
      </div>

      <div className="header-right">
        <div className="status-pill active-safe" title="Isolated local computing. Zero telemetry or network egress.">
          <WifiOff size={11} />
          <span>OFFLINE ENGINE</span>
        </div>
        <div className="status-pill active-wal" title="Transactional persistence with WAL journaling.">
          <HardDrive size={11} />
          <span>SQLITE WAL</span>
        </div>
        <div className="status-pill" title="Original firmware sources remain byte-for-byte immutable.">
          <ShieldCheck size={11} color="var(--accent-emerald)" />
          <span>NON-DESTRUCTIVE</span>
        </div>
      </div>
    </header>
  );
};
