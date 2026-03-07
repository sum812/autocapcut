import { UpdateState } from "../hooks/useUpdater";

interface Props {
  state: UpdateState;
  onInstall: () => void;
  onDismiss: () => void;
}

export default function UpdateBanner({ state, onInstall, onDismiss }: Props) {
  if (state.status === "idle" || state.status === "checking") return null;

  const isDownloading = state.status === "downloading";
  const isDone = state.status === "done";
  const isError = state.status === "error";

  return (
    <div
      className="flex items-center gap-3 px-4 py-2 text-xs border-b"
      style={{
        background: isError ? "rgba(239,68,68,0.1)" : "rgba(59,130,246,0.1)",
        borderColor: isError ? "rgba(239,68,68,0.3)" : "rgba(59,130,246,0.3)",
        color: "var(--text-pri)",
      }}
    >
      {/* Icon */}
      <span style={{ fontSize: 14 }}>
        {isError ? "⚠️" : isDone ? "✅" : "🔄"}
      </span>

      {/* Message */}
      <span className="flex-1" style={{ color: "var(--text-sec)" }}>
        {state.status === "available" && (
          <>Có phiên bản mới <strong style={{ color: "var(--text-pri)" }}>v{state.version}</strong> — cập nhật để nhận tính năng mới và sửa lỗi.</>
        )}
        {isDownloading && (
          <>Đang tải update{state.progress != null ? ` (${state.progress}%)` : "..."}
            {state.progress != null && (
              <span
                className="inline-block ml-2 rounded-full overflow-hidden align-middle"
                style={{ width: 60, height: 4, background: "var(--border)" }}
              >
                <span
                  className="block h-full rounded-full"
                  style={{ width: `${state.progress}%`, background: "var(--accent)" }}
                />
              </span>
            )}
          </>
        )}
        {isDone && "Cài đặt xong — đang khởi động lại..."}
        {isError && `Lỗi update: ${state.error}`}
      </span>

      {/* Actions */}
      {state.status === "available" && (
        <div className="flex gap-2 shrink-0">
          <button
            className="btn btn-primary"
            style={{ fontSize: 11, padding: "3px 10px" }}
            onClick={onInstall}
          >
            Cập nhật ngay
          </button>
          <button
            className="btn btn-ghost"
            style={{ fontSize: 11, padding: "3px 8px" }}
            onClick={onDismiss}
          >
            Bỏ qua
          </button>
        </div>
      )}

      {isError && (
        <button
          className="btn btn-ghost shrink-0"
          style={{ fontSize: 11, padding: "3px 8px" }}
          onClick={onDismiss}
        >
          Đóng
        </button>
      )}
    </div>
  );
}
