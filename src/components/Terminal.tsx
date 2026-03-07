import { useEffect, useRef } from "react";

interface Props {
  logs: string[];
  onClear: () => void;
}

export default function Terminal({ logs, onClear }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const isNearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
    if (isNearBottom) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs]);

  const getLineColor = (line: string) => {
    if (line.includes("✅") || line.includes("✓") || line.includes("Hoàn thành")) return "var(--success)";
    if (line.includes("❌") || line.includes("✗") || line.includes("Error")) return "var(--danger)";
    if (line.includes("⚠") || line.includes("Timeout") || line.includes("retry")) return "var(--warning)";
    if (line.startsWith("🚀") || line.startsWith("📁") || line.startsWith("⏱")) return "var(--info)";
    if (line.includes("[diag]") || line.includes("[debug]")) return "var(--text-dim)";
    return "var(--text-sec)";
  };

  return (
    <div className="flex flex-col h-full overflow-hidden">
      <div
        className="flex items-center justify-between px-3 py-1.5 border-b text-xs"
        style={{
          background: "var(--bg-surface)",
          borderColor: "var(--border)",
          color: "var(--text-dim)",
        }}
      >
        <span className="font-mono font-semibold">LOG</span>
        <div className="flex items-center gap-2">
          <span>{logs.length} dòng</span>
          <button
            className="btn btn-ghost text-xs py-0.5 px-2"
            onClick={onClear}
          >
            Xóa
          </button>
        </div>
      </div>

      <div
        ref={containerRef}
        className="flex-1 overflow-y-auto font-mono text-xs p-2 space-y-0.5"
        style={{ background: "var(--bg-base)" }}
      >
        {logs.map((line, i) => (
          <div
            key={i}
            style={{ color: getLineColor(line), whiteSpace: "pre-wrap", wordBreak: "break-all" }}
          >
            {line}
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
