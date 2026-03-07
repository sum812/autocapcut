import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { SettingsProvider, useSettings, AppConfig } from "./utils/SettingsContext";
import { LogProvider, useLogs } from "./utils/LogContext";
import { useAutomation } from "./hooks/useAutomation";
import { useCoordPicker } from "./hooks/useCoordPicker";

import Header from "./components/Header";
import FolderSelector from "./components/FolderSelector";
import ProjectTable, { Project } from "./components/ProjectTable";
import ControlPanel from "./components/ControlPanel";
import Terminal from "./components/Terminal";
import ProgressBar from "./components/ProgressBar";
import { useProgress } from "./hooks/useProgress";

function AppInner() {
  const { config, setConfig, saveConfig, loadConfig } = useSettings();
  const { logs, clearLogs, startListening } = useLogs();

  const [projects, setProjects] = useState<Project[]>([]);
  const [activeTab, setActiveTab] = useState<"folders" | "settings">("folders");

  // Start log listener on mount
  useEffect(() => {
    loadConfig();
    let unlisten: (() => void) | undefined;
    startListening().then((u) => (unlisten = u));
    return () => unlisten?.();
  }, []);

  // Listen for project status updates
  useEffect(() => {
    const unlistenStatus = listen<{ name: string; status: string }>(
      "project-status",
      (e) => {
        setProjects((prev) =>
          prev.map((p) =>
            p.name === e.payload.name
              ? { ...p, status: e.payload.status as Project["status"] }
              : p
          )
        );
      }
    );
    return () => {
      unlistenStatus.then((u) => u());
    };
  }, []);

  // Scan projects when project_folder changes
  useEffect(() => {
    if (!config.project_folder) return;
    invoke<Array<{ id: string; name: string; status: string }>>(
      "scan_projects",
      { path: config.project_folder }
    )
      .then((raw) =>
        setProjects(
          raw.map((p) => ({
            ...p,
            status: p.status as Project["status"],
            selected: true,
          }))
        )
      )
      .catch(() => {});
  }, [config.project_folder]);

  const handleConfigChange = useCallback((patch: Partial<AppConfig>) => {
    setConfig((prev) => ({ ...prev, ...patch }));
  }, []);

  const handleToggleProject = useCallback((id: string) => {
    setProjects((prev) =>
      prev.map((p) => (p.id === id ? { ...p, selected: !p.selected } : p))
    );
  }, []);

  const handleSelectAll = useCallback((checked: boolean) => {
    setProjects((prev) => prev.map((p) => ({ ...p, selected: checked })));
  }, []);

  const selectedNames = projects.filter((p) => p.selected).map((p) => p.name);

  const { autoStatus, start, stop } = useAutomation(config, selectedNames);

  const isRunning = autoStatus === "running" || autoStatus === "stopping";

  const { progress, percent } = useProgress(isRunning);

  const { picking, startPick } = useCoordPicker(
    (key, coords) => handleConfigChange({ [key]: coords })
  );

  const handleSave = async () => {
    await saveConfig();
  };

  return (
    <div className="flex flex-col h-screen" style={{ background: "var(--bg-base)" }}>
      <Header autoStatus={autoStatus} />
      <ProgressBar progress={progress} percent={percent} isRunning={isRunning} />

      <div className="flex flex-1 overflow-hidden">
        {/* Left panel */}
        <div
          className="flex flex-col w-[440px] shrink-0 border-r overflow-hidden"
          style={{ borderColor: "var(--border)" }}
        >
          {/* Tab bar */}
          <div
            className="flex border-b"
            style={{ borderColor: "var(--border)", background: "var(--bg-surface)" }}
          >
            {(["folders", "settings"] as const).map((tab) => (
              <button
                key={tab}
                onClick={() => setActiveTab(tab)}
                className="px-4 py-2 text-xs font-medium transition-colors"
                style={{
                  color:
                    activeTab === tab ? "var(--text-pri)" : "var(--text-dim)",
                  borderBottom:
                    activeTab === tab
                      ? "2px solid var(--accent)"
                      : "2px solid transparent",
                  background: "transparent",
                  cursor: "pointer",
                }}
              >
                {tab === "folders" ? "Cấu hình" : "Cài đặt"}
              </button>
            ))}
          </div>

          {/* Tab content */}
          <div className="flex-1 overflow-y-auto">
            {activeTab === "folders" ? (
              <div className="p-4">
                <FolderSelector
                  config={config}
                  onChange={handleConfigChange}
                  picking={picking}
                  onStartPick={startPick}
                />
              </div>
            ) : (
              <ControlPanel
                config={config}
                onChange={handleConfigChange}
                onSave={handleSave}
              />
            )}
          </div>

          {/* Project table */}
          <div
            className="flex flex-col overflow-hidden border-t"
            style={{ borderColor: "var(--border)", height: "280px" }}
          >
            <ProjectTable
              projects={projects}
              onToggle={handleToggleProject}
              onSelectAll={handleSelectAll}
              isRunning={isRunning}
            />
          </div>

          {/* Action buttons */}
          <div
            className="flex gap-2 p-3 border-t"
            style={{ borderColor: "var(--border)", background: "var(--bg-surface)" }}
          >
            <button
              className="btn btn-primary flex-1"
              disabled={isRunning || selectedNames.length === 0 || !config.project_folder}
              onClick={start}
            >
              ▶ Bắt đầu ({selectedNames.length})
            </button>
            <button
              className="btn btn-danger"
              disabled={!isRunning}
              onClick={stop}
            >
              ■ Dừng
            </button>
          </div>
        </div>

        {/* Right panel: Terminal */}
        <div className="flex-1 overflow-hidden">
          <Terminal logs={logs} onClear={clearLogs} />
        </div>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <SettingsProvider>
      <LogProvider>
        <AppInner />
      </LogProvider>
    </SettingsProvider>
  );
}
