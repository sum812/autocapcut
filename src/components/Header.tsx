import React from "react";
import { AutoStatus } from "../hooks/useAutomation";

interface Props {
  autoStatus: AutoStatus;
}

const STATUS_LABELS: Record<AutoStatus, string> = {
  idle: "Sẵn sàng",
  running: "Đang chạy",
  stopping: "Đang dừng...",
  stopped: "Đã dừng",
};

const STATUS_COLORS: Record<AutoStatus, string> = {
  idle: "bg-[rgba(139,139,154,0.12)] text-[var(--text-sec)]",
  running: "bg-[rgba(59,130,246,0.15)] text-[var(--info)] animate-pulse",
  stopping: "bg-[rgba(245,158,11,0.15)] text-[var(--warning)]",
  stopped: "bg-[rgba(239,68,68,0.12)] text-[var(--danger)]",
};

export default function Header({ autoStatus }: Props) {
  return (
    <header
      className="flex items-center justify-between px-5 py-3 border-b"
      style={{ borderColor: "var(--border)", background: "var(--bg-surface)" }}
    >
      <div className="flex items-center gap-3">
        <span className="text-lg font-bold tracking-tight text-[var(--text-pri)]">
          AutoCapcut
        </span>
        <span className="text-[var(--text-dim)] text-xs">v1.0.0</span>
      </div>

      <div className="flex items-center gap-3">
        <span className={`badge text-xs ${STATUS_COLORS[autoStatus]}`}>
          {STATUS_LABELS[autoStatus]}
        </span>
      </div>
    </header>
  );
}
