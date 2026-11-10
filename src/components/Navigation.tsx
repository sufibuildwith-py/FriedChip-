import React from "react";
import {
  LayoutDashboard,
  FolderGit2,
  Unplug,
  FlaskConical,
  Binary,
  Layers,
  Database,
  ScrollText,
  Settings,
  Sun,
  Moon,
  Monitor,
} from "lucide-react";
import { NavigationDestination } from "../types";
import { FriedChipLogo } from "./FriedChipLogo";
import { useTheme } from "../context/ThemeContext";

interface NavigationProps {
  currentView: NavigationDestination;
  onSelectView: (view: NavigationDestination) => void;
}

export const Navigation: React.FC<NavigationProps> = ({ currentView, onSelectView }) => {
  const { themePref, setThemePref } = useTheme();

  return (
    <aside className="sidebar">
      {/* Brand Header */}
      <div className="brand">
        <FriedChipLogo size={32} variant="accent" />
        <div className="brand-info">
          <div className="brand-title-row">
            <span className="brand-title">
              FriedChip<span className="brand-plus">+</span>
            </span>
            <span className="brand-badge">v0.1.0</span>
          </div>
          <span className="brand-subtitle">Firmware Engineering</span>
        </div>
      </div>

      {/* Navigation Sections */}
      <nav className="nav-menu">
        <div className="nav-section-title">Core Workbench</div>
        <div
          className={`nav-item ${currentView === "overview" ? "active" : ""}`}
          onClick={() => onSelectView("overview")}
        >
          <LayoutDashboard size={15} />
          <span>Overview</span>
        </div>
        <div
          className={`nav-item ${currentView === "workspace" ? "active" : ""}`}
          onClick={() => onSelectView("workspace")}
        >
          <FolderGit2 size={15} />
          <span>Workspace</span>
        </div>
        <div
          className={`nav-item ${currentView === "operation-history" ? "active" : ""}`}
          onClick={() => onSelectView("operation-history")}
        >
          <ScrollText size={15} />
          <span>Operation History</span>
        </div>

        <div className="nav-section-title">Firmware Engine</div>
        <div
          className={`nav-item ${currentView === "extractors" ? "active" : ""}`}
          onClick={() => onSelectView("extractors")}
        >
          <Unplug size={15} />
          <span>Extractors</span>
          <span className="badge-planned">Phase 6</span>
        </div>
        <div
          className={`nav-item ${currentView === "firmware-lab" ? "active" : ""}`}
          onClick={() => onSelectView("firmware-lab")}
        >
          <FlaskConical size={15} />
          <span>Firmware Lab</span>
          <span className="badge-planned">Phase 4</span>
        </div>
        <div
          className={`nav-item ${currentView === "binary-studio" ? "active" : ""}`}
          onClick={() => onSelectView("binary-studio")}
        >
          <Binary size={15} />
          <span>Binary Studio</span>
          <span className="badge-planned">Phase 2</span>
        </div>

        <div className="nav-section-title">Catalog & Reference</div>
        <div
          className={`nav-item ${currentView === "device-profiles" ? "active" : ""}`}
          onClick={() => onSelectView("device-profiles")}
        >
          <Layers size={15} />
          <span>Device Profiles</span>
          <span className="badge-planned">Phase 5</span>
        </div>
        <div
          className={`nav-item ${currentView === "firmware-library" ? "active" : ""}`}
          onClick={() => onSelectView("firmware-library")}
        >
          <Database size={15} />
          <span>Firmware Library</span>
          <span className="badge-planned">Phase 8</span>
        </div>

        <div className="nav-section-title">Workstation</div>
        <div
          className={`nav-item ${currentView === "settings" ? "active" : ""}`}
          onClick={() => onSelectView("settings")}
        >
          <Settings size={15} />
          <span>Settings & About</span>
        </div>
      </nav>

      {/* Sidebar Footer with Theme Switcher & Creator Signature */}
      <div className="sidebar-footer">
        <div className="theme-switch-row">
          <div className="theme-segmented">
            <button
              type="button"
              className={`theme-btn ${themePref === "light" ? "active" : ""}`}
              onClick={() => setThemePref("light")}
              title="Light Mode (Default)"
            >
              <Sun size={12} />
              <span>Light</span>
            </button>
            <button
              type="button"
              className={`theme-btn ${themePref === "dark" ? "active" : ""}`}
              onClick={() => setThemePref("dark")}
              title="Dark Mode"
            >
              <Moon size={12} />
              <span>Dark</span>
            </button>
            <button
              type="button"
              className={`theme-btn ${themePref === "system" ? "active" : ""}`}
              onClick={() => setThemePref("system")}
              title="System Mode (Windows Appearance)"
            >
              <Monitor size={12} />
              <span>Auto</span>
            </button>
          </div>
        </div>

        <div className="sidebar-credit">
          <div className="creator-tag" title="Creator & Lead Architect">
            <span className="creator-dot" />
            <span>Crafted by Sufiyan Khan</span>
          </div>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "9.5px", color: "var(--text-dim)" }}>
            Commercial
          </span>
        </div>
      </div>
    </aside>
  );
};
