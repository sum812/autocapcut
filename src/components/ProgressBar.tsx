import { ProgressState, formatDuration } from "../hooks/useProgress";

interface Props {
  progress: ProgressState;
  percent: number;
  isRunning: boolean;
}

export default function ProgressBar({ progress, percent, isRunning }: Props) {
  const { done, total, success, failed, current, elapsed_secs, eta_secs } = progress;
  const isActive = isRunning || (total > 0 && done > 0);

  if (!isActive) return null;

  const isDone = done === total && total > 0;

  return (
    <div
      className="px-4 py-2 border-b flex flex-col gap-1"
      style={{
        background: "var(--bg-surface)",
        borderColor: "var(--border)",
      }}
    >
      {/* Top row: fraction + counts + ETA */}
      <div className="flex items-center justify-between text-xs">
        <span style={{ color: "var(--text-sec)" }}>
          <span style={{ color: "var(--text-pri)", fontWeight: 600 }}>
            {done}/{total}
          </span>
          {" "}project
          {isDone && (
            <span style={{ color: "var(--success)", marginLeft: 6 }}>✓ Xong</span>
          )}
        </span>

        <div className="flex items-center gap-3">
          <span style={{ color: "var(--success)" }}>✅ {success}</span>
          <span style={{ color: "var(--danger)" }}>❌ {failed}</span>
          {elapsed_secs > 0 && (
            <span style={{ color: "var(--text-dim)" }}>
              ⏱ {formatDuration(elapsed_secs)}
            </span>
          )}
          {eta_secs != null && eta_secs > 0 && (
            <span style={{ color: "var(--accent)" }}>
              ~{formatDuration(eta_secs)} còn lại
            </span>
          )}
        </div>
      </div>

      {/* Progress bar */}
      <div
        className="h-1.5 rounded-full overflow-hidden"
        style={{ background: "var(--border)" }}
      >
        <div
          className="h-full rounded-full transition-all duration-500"
          style={{
            width: `${percent.toFixed(1)}%`,
            background: isDone
              ? "var(--success)"
              : failed > 0
              ? "var(--accent)"
              : "var(--accent)",
          }}
        />
      </div>

      {/* Current project */}
      {current && (
        <div
          className="text-xs truncate"
          style={{ color: "var(--text-dim)" }}
        >
          ▶ {current}
        </div>
      )}
    </div>
  );
}
