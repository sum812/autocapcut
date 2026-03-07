import { open } from "@tauri-apps/plugin-dialog";
import { AppConfig } from "../utils/SettingsContext";
import { useAutoDetect } from "../hooks/useAutoDetect";

interface Props {
  config: AppConfig;
  onChange: (patch: Partial<AppConfig>) => void;
  picking: string | null;
  onStartPick: (
    key: "first_project_coords" | "export_box_coords" | "search_button_coords"
  ) => void;
}

export default function FolderSelector({
  config,
  onChange,
  picking,
  onStartPick,
}: Props) {
  const { detectState, detect } = useAutoDetect(onChange);

  const pickFolder = async (field: "project_folder" | "export_folder") => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string" && selected) {
      onChange({ [field]: selected });
    }
  };

  const fmtCoord = (c: [number, number]) =>
    c[0] === 0 && c[1] === 0 ? "Chưa calibrate" : `(${c[0]}, ${c[1]})`;

  const CoordRow = ({
    label,
    value,
    pickKey,
    tip,
  }: {
    label: string;
    value: [number, number];
    pickKey: "first_project_coords" | "export_box_coords" | "search_button_coords";
    tip: string;
  }) => {
    const isDetecting = detectState.status === "detecting" && detectState.key === pickKey;
    const isDone = detectState.status === "done" && detectState.key === pickKey;
    const isError = detectState.status === "error" && detectState.key === pickKey;
    const hasTemplate = value[0] !== 0 || value[1] !== 0;
    const busy = picking !== null || detectState.status === "detecting";

    return (
      <div className="flex flex-col gap-0.5 py-1.5">
        <div className="flex items-center gap-2">
          <span
            className="w-40 shrink-0 text-xs"
            style={{ color: "var(--text-sec)" }}
          >
            {label}
          </span>
          <span
            className="flex-1 font-mono text-xs"
            style={{ color: value[0] === 0 ? "var(--text-dim)" : "var(--text-pri)" }}
          >
            {fmtCoord(value)}
          </span>
          <button
            className="btn btn-ghost text-xs py-1 px-2"
            disabled={busy}
            onClick={() => onStartPick(pickKey)}
            title={tip}
          >
            {picking === pickKey ? "⏳ Chờ..." : "Calibrate"}
          </button>
          <button
            className="btn btn-ghost text-xs py-1 px-2"
            disabled={busy || !hasTemplate}
            onClick={() => detect(pickKey)}
            title={
              hasTemplate
                ? "Tự động phát hiện vị trí dựa trên template đã lưu"
                : "Chưa có template — hãy Calibrate thủ công trước"
            }
          >
            {isDetecting ? "🔍 Đang tìm..." : isDone ? "✅ Xong" : "🔍 Auto"}
          </button>
        </div>
        {isError && (
          <span className="text-xs ml-40 pl-2" style={{ color: "var(--color-error, #f87171)" }}>
            {detectState.error}
          </span>
        )}
      </div>
    );
  };

  return (
    <div className="flex flex-col gap-3">
      {/* Project folder */}
      <div className="flex flex-col gap-1">
        <label className="text-xs font-medium" style={{ color: "var(--text-sec)" }}>
          Thư mục dự án CapCut
        </label>
        <div className="flex gap-2">
          <input
            className="input text-xs flex-1 font-mono"
            readOnly
            value={config.project_folder}
            placeholder="Chọn thư mục..."
          />
          <button
            className="btn btn-ghost text-xs py-1 px-3"
            onClick={() => pickFolder("project_folder")}
          >
            Chọn
          </button>
        </div>
      </div>

      {/* Export folder */}
      <div className="flex flex-col gap-1">
        <label className="text-xs font-medium" style={{ color: "var(--text-sec)" }}>
          Thư mục xuất video
        </label>
        <div className="flex gap-2">
          <input
            className="input text-xs flex-1 font-mono"
            readOnly
            value={config.export_folder}
            placeholder="Chọn thư mục..."
          />
          <button
            className="btn btn-ghost text-xs py-1 px-3"
            onClick={() => pickFolder("export_folder")}
          >
            Chọn
          </button>
        </div>
      </div>

      {/* Coordinate calibration */}
      <div
        className="rounded-lg p-3 flex flex-col gap-0.5"
        style={{ background: "var(--bg-surface)", border: "1px solid var(--border)" }}
      >
        <div className="text-xs font-semibold mb-1" style={{ color: "var(--text-sec)" }}>
          Hiệu chỉnh tọa độ — Calibrate thủ công lần đầu, sau đó dùng 🔍 Auto để tự phát hiện
        </div>
        <CoordRow
          label="Project đầu tiên"
          value={config.first_project_coords}
          pickKey="first_project_coords"
          tip="Di chuột đến project đầu tiên trong CapCut Home, nhấn Space"
        />
        <CoordRow
          label="Ô Export path"
          value={config.export_box_coords}
          pickKey="export_box_coords"
          tip="Di chuột đến ô chọn folder trong export dialog, nhấn Space"
        />
        <CoordRow
          label="Nút tìm kiếm"
          value={config.search_button_coords}
          pickKey="search_button_coords"
          tip="(Tuỳ chọn) Di chuột đến ô search trong CapCut Home, nhấn Space"
        />
      </div>
    </div>
  );
}
