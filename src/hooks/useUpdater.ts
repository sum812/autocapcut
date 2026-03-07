import { useState, useEffect, useCallback } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus = "idle" | "checking" | "available" | "downloading" | "done" | "error";

export interface UpdateState {
  status: UpdateStatus;
  version: string | null;
  body: string | null;
  progress: number | null;  // 0-100, null nếu không có info
  error: string | null;
}

export function useUpdater() {
  const [state, setState] = useState<UpdateState>({
    status: "idle",
    version: null,
    body: null,
    progress: null,
    error: null,
  });

  const checkForUpdate = useCallback(async () => {
    setState((s) => ({ ...s, status: "checking", error: null }));
    try {
      const update = await check();
      if (update) {
        setState({
          status: "available",
          version: update.version,
          body: update.body ?? null,
          progress: null,
          error: null,
        });
      } else {
        setState({ status: "idle", version: null, body: null, progress: null, error: null });
      }
    } catch (e) {
      // Bỏ qua lỗi network/endpoint khi pubkey chưa được cấu hình
      setState({ status: "idle", version: null, body: null, progress: null, error: null });
      console.warn("[updater] check failed:", e);
    }
  }, []);

  const downloadAndInstall = useCallback(async () => {
    setState((s) => ({ ...s, status: "downloading", progress: 0 }));
    try {
      const update = await check();
      if (!update) {
        setState((s) => ({ ...s, status: "idle" }));
        return;
      }

      let downloaded = 0;
      let total = 0;

      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          const pct = total > 0 ? Math.round((downloaded / total) * 100) : null;
          setState((s) => ({ ...s, progress: pct }));
        } else if (event.event === "Finished") {
          setState((s) => ({ ...s, status: "done", progress: 100 }));
        }
      });

      await relaunch();
    } catch (e) {
      setState((s) => ({
        ...s,
        status: "error",
        error: String(e),
      }));
    }
  }, []);

  const dismiss = useCallback(() => {
    setState({ status: "idle", version: null, body: null, progress: null, error: null });
  }, []);

  // Kiểm tra update 3 giây sau khi app mở
  useEffect(() => {
    const timer = setTimeout(checkForUpdate, 3000);
    return () => clearTimeout(timer);
  }, []);

  return { state, checkForUpdate, downloadAndInstall, dismiss };
}
