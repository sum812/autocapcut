export interface Project {
  id: string;
  name: string;
  status: "Pending" | "Running" | "Done" | "Error";
  selected: boolean;
}

interface Props {
  projects: Project[];
  onToggle: (id: string) => void;
  onSelectAll: (checked: boolean) => void;
  isRunning: boolean;
}

const STATUS_BADGE: Record<Project["status"], string> = {
  Pending: "badge badge-pending",
  Running: "badge badge-running",
  Done: "badge badge-done",
  Error: "badge badge-error",
};

export default function ProjectTable({
  projects,
  onToggle,
  onSelectAll,
  isRunning,
}: Props) {
  const allSelected = projects.length > 0 && projects.every((p) => p.selected);
  const someSelected = projects.some((p) => p.selected);
  const selectedCount = projects.filter((p) => p.selected).length;

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Header */}
      <div
        className="flex items-center gap-3 px-3 py-2 border-b text-xs font-semibold"
        style={{
          background: "var(--bg-card)",
          borderColor: "var(--border)",
          color: "var(--text-sec)",
        }}
      >
        <input
          type="checkbox"
          className="w-3.5 h-3.5 accent-[var(--accent)] cursor-pointer"
          checked={allSelected}
          ref={(el) => {
            if (el) el.indeterminate = !allSelected && someSelected;
          }}
          onChange={(e) => onSelectAll(e.target.checked)}
          disabled={isRunning || projects.length === 0}
        />
        <span className="flex-1">Tên dự án</span>
        <span className="w-20 text-right">Trạng thái</span>
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto">
        {projects.length === 0 ? (
          <div
            className="flex items-center justify-center h-full text-xs"
            style={{ color: "var(--text-dim)" }}
          >
            Chọn thư mục dự án để quét danh sách
          </div>
        ) : (
          projects.map((p) => (
            <div
              key={p.id}
              className="flex items-center gap-3 px-3 py-2 border-b cursor-pointer hover:bg-[var(--bg-card)] transition-colors"
              style={{ borderColor: "var(--border)" }}
              onClick={() => !isRunning && onToggle(p.id)}
            >
              <input
                type="checkbox"
                className="w-3.5 h-3.5 accent-[var(--accent)] cursor-pointer"
                checked={p.selected}
                onChange={() => !isRunning && onToggle(p.id)}
                onClick={(e) => e.stopPropagation()}
                disabled={isRunning}
              />
              <span
                className="flex-1 text-xs truncate"
                style={{ color: "var(--text-pri)" }}
              >
                {p.name}
              </span>
              <span className={STATUS_BADGE[p.status]}>{p.status}</span>
            </div>
          ))
        )}
      </div>

      {/* Footer */}
      <div
        className="px-3 py-1.5 text-xs border-t"
        style={{ borderColor: "var(--border)", color: "var(--text-dim)" }}
      >
        {selectedCount}/{projects.length} đã chọn
      </div>
    </div>
  );
}
