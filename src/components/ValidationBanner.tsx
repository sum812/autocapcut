import { ValidationError } from "../hooks/useValidation";

interface Props {
  errors: ValidationError[];
  onDismiss: () => void;
}

const FIELD_LABELS: Record<string, string> = {
  project_folder: "Thư mục Projects",
  export_folder: "Thư mục Export",
  first_project_coords: "Tọa độ Project",
  export_box_coords: "Tọa độ Export Path",
  capcut: "CapCut",
  project_names: "Project",
  system: "Hệ thống",
};

export default function ValidationBanner({ errors, onDismiss }: Props) {
  if (errors.length === 0) return null;

  return (
    <div
      className="flex flex-col gap-1 px-4 py-3 border-b"
      style={{
        background: "rgba(239, 68, 68, 0.1)",
        borderColor: "rgba(239, 68, 68, 0.3)",
      }}
    >
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold" style={{ color: "#ef4444" }}>
          ⚠ Không thể bắt đầu — cần kiểm tra lại:
        </span>
        <button
          className="text-xs opacity-60 hover:opacity-100"
          style={{ color: "#ef4444" }}
          onClick={onDismiss}
        >
          ✕
        </button>
      </div>
      <ul className="flex flex-col gap-0.5">
        {errors.map((err, i) => (
          <li key={i} className="text-xs" style={{ color: "#fca5a5" }}>
            • <span style={{ color: "#f87171" }}>{FIELD_LABELS[err.field] ?? err.field}:</span>{" "}
            {err.message}
          </li>
        ))}
      </ul>
    </div>
  );
}
