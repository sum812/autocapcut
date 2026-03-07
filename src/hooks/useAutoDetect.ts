import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppConfig } from "../utils/SettingsContext";

type CoordKey = "first_project_coords" | "export_box_coords" | "search_button_coords";

type DetectStatus = "idle" | "detecting" | "done" | "error";

interface DetectState {
  status: DetectStatus;
  key: CoordKey | null;
  error: string | null;
}

export function useAutoDetect(onChange: (patch: Partial<AppConfig>) => void) {
  const [state, setState] = useState<DetectState>({
    status: "idle",
    key: null,
    error: null,
  });

  const detect = useCallback(async (key: CoordKey) => {
    setState({ status: "detecting", key, error: null });
    try {
      const coords = await invoke<[number, number]>("detect_ui_coords", { coordKey: key });
      onChange({ [key]: coords });
      setState({ status: "done", key, error: null });
      // Reset về idle sau 2 giây
      setTimeout(() => setState({ status: "idle", key: null, error: null }), 2000);
    } catch (e) {
      setState({ status: "error", key, error: String(e) });
      setTimeout(() => setState({ status: "idle", key: null, error: null }), 4000);
    }
  }, [onChange]);

  return { detectState: state, detect };
}
