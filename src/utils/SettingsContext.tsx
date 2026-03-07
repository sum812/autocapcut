import React, {
  createContext,
  useContext,
  useState,
  useCallback,
  ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";

export interface AppConfig {
  project_folder: string;
  export_folder: string;
  first_project_coords: [number, number];
  export_box_coords: [number, number];
  search_button_coords: [number, number];
  render_delay: number;
  render_timeout: number;
  shutdown: boolean;
}

const DEFAULT_CONFIG: AppConfig = {
  project_folder: "",
  export_folder: "",
  first_project_coords: [0, 0],
  export_box_coords: [0, 0],
  search_button_coords: [0, 0],
  render_delay: 5,
  render_timeout: 30,
  shutdown: false,
};

interface SettingsCtx {
  config: AppConfig;
  setConfig: React.Dispatch<React.SetStateAction<AppConfig>>;
  saveConfig: () => Promise<void>;
  loadConfig: () => Promise<void>;
}

const SettingsContext = createContext<SettingsCtx | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG);

  const saveConfig = useCallback(async () => {
    await invoke("save_config", { config });
  }, [config]);

  const loadConfig = useCallback(async () => {
    try {
      const saved = await invoke<AppConfig>("load_config");
      if (saved) setConfig(saved);
    } catch {}
  }, []);

  return (
    <SettingsContext.Provider value={{ config, setConfig, saveConfig, loadConfig }}>
      {children}
    </SettingsContext.Provider>
  );
}

export function useSettings() {
  const ctx = useContext(SettingsContext);
  if (!ctx) throw new Error("useSettings must be used within SettingsProvider");
  return ctx;
}
