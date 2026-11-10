import React, { useEffect, useState, useCallback } from "react";
import { Navigation } from "./components/Navigation";
import { Header } from "./components/Header";
import { OverviewView } from "./components/OverviewView";
import { WorkspaceView } from "./components/WorkspaceView";
import { HistoryView } from "./components/HistoryView";
import { SettingsView } from "./components/SettingsView";
import { PlannedView } from "./components/PlannedView";
import { ExtractorsView } from "./components/ExtractorsView";
import { FirmwareFile, NavigationDestination, Operation, Workspace } from "./types";
import { api } from "./api/tauriBridge";
import { ThemeProvider } from "./context/ThemeContext";
import { FriedChipLogo } from "./components/FriedChipLogo";
import "./styles/workstation.css";

const MainContent: React.FC = () => {
  const getInitialView = (): NavigationDestination => {
    const hash = window.location.hash.replace("#", "") as NavigationDestination;
    const validViews: NavigationDestination[] = [
      "overview",
      "workspace",
      "operation-history",
      "extractors",
      "firmware-lab",
      "binary-studio",
      "device-profiles",
      "firmware-library",
      "settings",
    ];
    return validViews.includes(hash) ? hash : "overview";
  };

  const [currentView, setCurrentView] = useState<NavigationDestination>(getInitialView);
  const [activeWorkspace, setActiveWorkspace] = useState<Workspace | null>(null);
  const [files, setFiles] = useState<FirmwareFile[]>([]);
  const [operations, setOperations] = useState<Operation[]>([]);
  const [loading, setLoading] = useState<boolean>(true);

  // Sync hash changes
  useEffect(() => {
    const onHashChange = () => {
      const hash = window.location.hash.replace("#", "") as NavigationDestination;
      if (hash) setCurrentView(hash);
    };
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  const handleSelectView = (view: NavigationDestination) => {
    window.location.hash = view;
    setCurrentView(view);
  };

  const refreshData = useCallback(async () => {
    try {
      const ws = await api.getActiveWorkspace();
      setActiveWorkspace(ws);

      const [wsFiles, wsOps] = await Promise.all([
        api.listWorkspaceFiles(ws.id),
        api.listOperations(ws.id),
      ]);

      setFiles(wsFiles);
      setOperations(wsOps);
    } catch (err) {
      console.error("Failed to load workspace data:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshData();
  }, [refreshData]);

  const renderContent = () => {
    if (loading) {
      return (
        <div className="content-body" style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%" }}>
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: "16px" }}>
            <FriedChipLogo size={48} variant="accent" />
            <div style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-main)", letterSpacing: "-0.01em" }}>
              Initializing FriedChip<span style={{ color: "var(--accent-primary)" }}>+</span> Workstation...
            </div>
            <div style={{ fontSize: "11px", color: "var(--text-dim)", fontFamily: "var(--font-mono)" }}>
              SQLite WAL Persistence &bull; Offline Engine Ready
            </div>
          </div>
        </div>
      );
    }

    switch (currentView) {
      case "overview":
        return (
          <OverviewView
            activeWorkspace={activeWorkspace}
            files={files}
            operations={operations}
            onNavigate={(view) => setCurrentView(view)}
          />
        );
      case "workspace":
        return (
          <WorkspaceView
            activeWorkspace={activeWorkspace}
            files={files}
            onFileImported={refreshData}
          />
        );
      case "operation-history":
        return <HistoryView operations={operations} />;
      case "settings":
        return <SettingsView />;
      case "extractors":
        return (
          <ExtractorsView
            activeWorkspace={activeWorkspace}
            files={files}
            onRefreshWorkspace={refreshData}
            onNavigateToWorkspace={() => setCurrentView("workspace")}
          />
        );
      case "firmware-lab":
      case "binary-studio":
      case "device-profiles":
      case "firmware-library":
        return (
          <PlannedView
            view={currentView}
            onNavigateToWorkspace={() => setCurrentView("workspace")}
          />
        );
      default:
        return <div>Unknown View</div>;
    }
  };

  return (
    <div className="app-container">
      <Navigation currentView={currentView} onSelectView={handleSelectView} />
      <div className="main-wrapper">
        <Header currentView={currentView} activeWorkspace={activeWorkspace} />
        {renderContent()}
      </div>
    </div>
  );
};

export const App: React.FC = () => {
  return (
    <ThemeProvider>
      <MainContent />
    </ThemeProvider>
  );
};

export default App;
