import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

export interface ProgressState {
  done: number;
  total: number;
  success: number;
  failed: number;
  current: string;
  elapsed_secs: number;
  eta_secs: number | null;
}

const INITIAL: ProgressState = {
  done: 0,
  total: 0,
  success: 0,
  failed: 0,
  current: "",
  elapsed_secs: 0,
  eta_secs: null,
};

export function useProgress(isRunning: boolean) {
  const [progress, setProgress] = useState<ProgressState>(INITIAL);

  useEffect(() => {
    // Reset khi bắt đầu chạy mới
    if (isRunning) setProgress(INITIAL);
  }, [isRunning]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<ProgressState>("progress", (e) => {
      setProgress(e.payload);
    }).then((u) => (unlisten = u));
    return () => unlisten?.();
  }, []);

  const percent = progress.total > 0 ? (progress.done / progress.total) * 100 : 0;

  return { progress, percent };
}

/// Format giây → "Xm Ys" hoặc "Xs"
export function formatDuration(secs: number): string {
  if (secs <= 0) return "0s";
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}
