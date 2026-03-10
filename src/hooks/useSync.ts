import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface SyncResult {
  project: string;
  success: boolean;
  message: string;
  changes: string[];
}

export interface SyncOptions {
  sync_video_audio: boolean;
  sync_image_duration: boolean;
  sync_subtitles: boolean;
}

export function useSync() {
  const [isProcessing, setIsProcessing] = useState(false);
  const [logs, setLogs] = useState<SyncResult[]>([]);

  useEffect(() => {
    const unlisten = listen<SyncResult>("sync_log", (event) => {
      setLogs((prev) => [event.payload, ...prev]);
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const processProjects = useCallback(
    async (
      projectNames: string[],
      projectFolder: string,
      options: SyncOptions
    ): Promise<SyncResult[]> => {
      setIsProcessing(true);
      setLogs([]);
      try {
        const results = await invoke<SyncResult[]>("process_batch", {
          projectNames,
          projectFolder,
          options,
        });
        return results;
      } catch (e) {
        const errResult: SyncResult = {
          project: "ALL",
          success: false,
          message: String(e),
          changes: [],
        };
        setLogs([errResult]);
        return [errResult];
      } finally {
        setIsProcessing(false);
      }
    },
    []
  );

  return { isProcessing, logs, processProjects };
}
