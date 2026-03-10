import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AppConfig } from "../utils/SettingsContext";

export type AutoStatus = "idle" | "running" | "stopping" | "stopped";

export function useAutomation(
  config: AppConfig,
  projectNames: string[],
) {
  const [autoStatus, setAutoStatus] = useState<AutoStatus>("idle");

  useEffect(() => {
    const unlistens: Array<() => void> = [];

    listen<string>("automation-status", (e) => {
      const s = e.payload.toLowerCase() as AutoStatus;
      setAutoStatus(s);
    }).then((u) => unlistens.push(u));

    return () => unlistens.forEach((u) => u());
  }, []);

  const start = useCallback(async () => {
    if (autoStatus === "running") return;
    setAutoStatus("running");
    try {
      await invoke("start_automation", {
        config: {
          project_path: config.project_folder,
          export_path: config.export_folder,
          project_names: projectNames,
          first_project_coords: config.first_project_coords,
          export_input_coords: config.export_box_coords,
          search_button_coords: config.search_button_coords,
          render_delay: config.render_delay,
          render_timeout_minutes: config.render_timeout,
          shutdown: config.shutdown,
          max_retries: config.max_retries,
          notify_on_done: config.notify_on_done,
          notify_per_project: config.notify_per_project,
        },
      });
    } catch (e) {
      console.error(e);
      setAutoStatus("idle");
    }
  }, [autoStatus, config, projectNames]);

  const stop = useCallback(async () => {
    if (autoStatus !== "running") return;
    setAutoStatus("stopping");
    await invoke("stop_automation");
  }, [autoStatus]);

  return { autoStatus, start, stop };
}
