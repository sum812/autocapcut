import { useState, useCallback } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { AppConfig } from "../utils/SettingsContext";
import { useCoordPicker } from "../hooks/useCoordPicker";
import { useAutoDetect } from "../hooks/useAutoDetect";

interface Props {
  config: AppConfig;
  onChange: (patch: Partial<AppConfig>) => void;
  onComplete: () => void;
}

const TOTAL_STEPS = 4;

type CoordKey = "first_project_coords" | "export_box_coords" | "search_button_coords";

const COORD_INFO: { key: CoordKey; label: string; tip: string; required: boolean }[] = [
  {
    key: "first_project_coords",
    label: "Project đầu tiên",
    tip: "Di chuột đến project đầu tiên trong CapCut Home rồi bấm Space",
    required: true,
  },
  {
    key: "export_box_coords",
    label: "Ô Export path",
    tip: "Di chuột đến nút chọn thư mục trong Export dialog rồi bấm Space",
    required: true,
  },
  {
    key: "search_button_coords",
    label: "Nút tìm kiếm (tuỳ chọn)",
    tip: "Di chuột đến ô search trong CapCut Home rồi bấm Space",
    required: false,
  },
];

function StepIndicator({ step }: { step: number }) {
  return (
    <div className="flex items-center gap-2">
      {Array.from({ length: TOTAL_STEPS }, (_, i) => i + 1).map((n) => (
        <div key={n} className="flex items-center gap-2">
          <div
            className="w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold transition-all"
            style={{
              background: n < step ? "var(--accent)" : n === step ? "var(--accent)" : "var(--bg-surface)",
              color: n <= step ? "#fff" : "var(--text-dim)",
              border: n > step ? "1px solid var(--border)" : "none",
              opacity: n < step ? 0.6 : 1,
            }}
          >
            {n < step ? "✓" : n}
          </div>
          {n < TOTAL_STEPS && (
            <div
              className="w-8 h-px"
              style={{ background: n < step ? "var(--accent)" : "var(--border)" }}
            />
          )}
        </div>
      ))}
    </div>
  );
}

export default function SetupWizard({ config, onChange, onComplete }: Props) {
  const [step, setStep] = useState(1);
  const [testStatus, setTestStatus] = useState<"idle" | "running" | "done" | "error">("idle");

  const handleConfigChange = useCallback(
    (patch: Partial<AppConfig>) => onChange(patch),
    [onChange]
  );

  const { picking, startPick } = useCoordPicker(
    (key, coords) => handleConfigChange({ [key]: coords })
  );

  const { detectState, detect } = useAutoDetect(handleConfigChange);

  const pickFolder = async (field: "project_folder" | "export_folder") => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string" && selected) {
      onChange({ [field]: selected });
    }
  };

  const fmtCoord = (c: [number, number]) =>
    c[0] === 0 && c[1] === 0 ? "Chưa calibrate" : `(${c[0]}, ${c[1]})`;

  const hasRequiredFolders = config.project_folder && config.export_folder;
  const hasRequiredCoords =
    config.first_project_coords[0] !== 0 && config.export_box_coords[0] !== 0;

  const canProceedStep1 = hasRequiredFolders;
  const canProceedStep2 = hasRequiredCoords;

  const handleFinish = async () => {
    onChange({ wizard_completed: true });
    // saveConfig sẽ được gọi từ App.tsx sau khi onChange propagate
    // Gọi invoke trực tiếp ở đây để đảm bảo config với wizard_completed=true được lưu
    try {
      await invoke("save_config", { config: { ...config, wizard_completed: true } });
    } catch {}
    onComplete();
  };

  // Step 3: chạy test với 1 project
  const handleTestRun = async () => {
    if (!config.project_folder) return;
    setTestStatus("running");
    try {
      // Scan để lấy project đầu tiên
      const projects = await invoke<Array<{ id: string; name: string }>>( "scan_projects", { path: config.project_folder });
      if (projects.length === 0) {
        setTestStatus("error");
        return;
      }
      const firstName = projects[0].name;
      await invoke("start_automation", {
        config: {
          project_path: config.project_folder,
          export_path: config.export_folder,
          project_names: [firstName],
          first_project_coords: config.first_project_coords,
          export_input_coords: config.export_box_coords,
          search_button_coords: config.search_button_coords,
          render_delay: config.render_delay,
          render_timeout_minutes: config.render_timeout,
          shutdown: false,
          max_retries: 1,
        },
      });
      setTestStatus("done");
    } catch {
      setTestStatus("error");
    }
  };

  return (
    <div
      className="fixed inset-0 flex items-center justify-center z-50"
      style={{ background: "rgba(0,0,0,0.7)", backdropFilter: "blur(4px)" }}
    >
      <div
        className="rounded-xl shadow-2xl w-[560px] max-h-[85vh] overflow-y-auto"
        style={{ background: "var(--bg-base)", border: "1px solid var(--border)" }}
      >
        {/* Header */}
        <div className="px-8 pt-7 pb-5 border-b" style={{ borderColor: "var(--border)" }}>
          <div className="text-lg font-bold" style={{ color: "var(--text-pri)" }}>
            Thiết lập AutoCapcut
          </div>
          <div className="text-xs mt-1" style={{ color: "var(--text-dim)" }}>
            Bước {step}/{TOTAL_STEPS} — chỉ cần làm 1 lần
          </div>
          <div className="mt-5">
            <StepIndicator step={step} />
          </div>
        </div>

        {/* Step content */}
        <div className="px-8 py-6">

        {/* ─── Step 1: Chọn thư mục ─────────────────────────── */}
        {step === 1 && (
          <div className="flex flex-col gap-6">
            <p className="text-xs" style={{ color: "var(--text-sec)" }}>
              Chọn thư mục chứa project CapCut và thư mục xuất video ra.
            </p>

            {/* Project folder */}
            <div className="flex flex-col gap-2">
              <label className="text-xs font-medium" style={{ color: "var(--text-sec)" }}>
                Thư mục dự án CapCut <span style={{ color: "var(--color-error, #f87171)" }}>*</span>
              </label>
              <div className="flex gap-2">
                <input
                  className="input text-xs flex-1 font-mono"
                  readOnly
                  value={config.project_folder}
                  placeholder="Chọn thư mục..."
                />
                <button className="btn btn-ghost text-xs py-1 px-3" onClick={() => pickFolder("project_folder")}>
                  Chọn
                </button>
              </div>
              {config.project_folder && (
                <span className="text-xs" style={{ color: "var(--accent)" }}>✓ Đã chọn</span>
              )}
            </div>

            {/* Export folder */}
            <div className="flex flex-col gap-2">
              <label className="text-xs font-medium" style={{ color: "var(--text-sec)" }}>
                Thư mục xuất video <span style={{ color: "var(--color-error, #f87171)" }}>*</span>
              </label>
              <div className="flex gap-2">
                <input
                  className="input text-xs flex-1 font-mono"
                  readOnly
                  value={config.export_folder}
                  placeholder="Chọn thư mục..."
                />
                <button className="btn btn-ghost text-xs py-1 px-3" onClick={() => pickFolder("export_folder")}>
                  Chọn
                </button>
              </div>
              {config.export_folder && (
                <span className="text-xs" style={{ color: "var(--accent)" }}>✓ Đã chọn</span>
              )}
            </div>
          </div>
        )}

        {/* ─── Step 2: Hiệu chỉnh tọa độ ───────────────────── */}
        {step === 2 && (
          <div className="flex flex-col gap-4">
            <div>
              <div className="text-sm font-semibold mb-1" style={{ color: "var(--text-pri)" }}>
                Hiệu chỉnh tọa độ
              </div>
              <div
                className="rounded-lg p-3 mb-4 text-xs"
                style={{ background: "var(--bg-surface)", color: "var(--text-sec)", border: "1px solid var(--border)" }}
              >
                <div className="font-medium mb-1" style={{ color: "var(--text-pri)" }}>Cách làm:</div>
                <ol className="list-decimal list-inside flex flex-col gap-1">
                  <li>Mở CapCut Desktop và maximize cửa sổ lên toàn màn hình</li>
                  <li>Bấm nút <strong>Calibrate</strong> cho từng điểm bên dưới</li>
                  <li>Di chuột đến đúng vị trí trong CapCut, bấm <strong>Space</strong> để xác nhận</li>
                  <li>Bấm <strong>Esc</strong> để hủy</li>
                </ol>
              </div>

              <div
                className="rounded-lg p-3 flex flex-col gap-1"
                style={{ background: "var(--bg-surface)", border: "1px solid var(--border)" }}
              >
                {COORD_INFO.map(({ key, label, tip, required }) => {
                  const val = config[key] as [number, number];
                  const isDone = val[0] !== 0 || val[1] !== 0;
                  const isDetecting = detectState.status === "detecting" && detectState.key === key;
                  const isDoneDetect = detectState.status === "done" && detectState.key === key;
                  const busy = picking !== null || detectState.status === "detecting";

                  return (
                    <div key={key} className="flex flex-col gap-0.5 py-1.5">
                      <div className="flex items-center gap-2">
                        <span
                          className="flex items-center gap-1 shrink-0 text-xs"
                          style={{ color: "var(--text-sec)", width: "170px" }}
                        >
                          {isDone ? (
                            <span style={{ color: "var(--accent)" }}>✓</span>
                          ) : required ? (
                            <span style={{ color: "var(--color-error, #f87171)" }}>*</span>
                          ) : (
                            <span style={{ color: "var(--text-dim)" }}>○</span>
                          )}
                          {label}
                        </span>
                        <span
                          className="flex-1 font-mono text-xs"
                          style={{ color: isDone ? "var(--text-pri)" : "var(--text-dim)" }}
                        >
                          {fmtCoord(val)}
                        </span>
                        <button
                          className="btn btn-ghost text-xs py-1 px-2"
                          disabled={busy}
                          onClick={() => startPick(key)}
                          title={tip}
                        >
                          {picking === key ? "⏳ Chờ..." : "Calibrate"}
                        </button>
                        {isDone && (
                          <button
                            className="btn btn-ghost text-xs py-1 px-2"
                            disabled={busy}
                            onClick={() => detect(key)}
                            title="Tự động phát hiện lại dựa trên template đã lưu"
                          >
                            {isDetecting ? "🔍..." : isDoneDetect ? "✅" : "🔍 Auto"}
                          </button>
                        )}
                      </div>
                      {detectState.status === "error" && detectState.key === key && (
                        <span className="text-xs ml-44" style={{ color: "var(--color-error, #f87171)" }}>
                          {detectState.error}
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>

              {!hasRequiredCoords && (
                <p className="text-xs mt-2" style={{ color: "var(--color-error, #f87171)" }}>
                  * Hai điểm đầu tiên là bắt buộc
                </p>
              )}
            </div>
          </div>
        )}

        {/* ─── Step 3: Test chạy thử ────────────────────────── */}
        {step === 3 && (
          <div className="flex flex-col gap-4">
            <div>
              <div className="text-sm font-semibold mb-1" style={{ color: "var(--text-pri)" }}>
                Test chạy thử (tuỳ chọn)
              </div>
              <p className="text-xs mb-4" style={{ color: "var(--text-sec)" }}>
                Chạy thử với 1 project đầu tiên để kiểm tra automation hoạt động đúng
                trước khi xử lý hàng loạt.
              </p>

              <div
                className="rounded-lg p-4 flex flex-col items-center gap-3"
                style={{ background: "var(--bg-surface)", border: "1px solid var(--border)" }}
              >
                {testStatus === "idle" && (
                  <button className="btn btn-primary px-6" onClick={handleTestRun}>
                    ▶ Chạy thử 1 project
                  </button>
                )}
                {testStatus === "running" && (
                  <div className="text-sm" style={{ color: "var(--text-sec)" }}>
                    ⏳ Đang chạy thử... (kiểm tra Terminal sau khi chuyển về màn hình chính)
                  </div>
                )}
                {testStatus === "done" && (
                  <div className="text-sm" style={{ color: "var(--accent)" }}>
                    ✅ Đã gửi lệnh render thử — kiểm tra thư mục export sau vài phút
                  </div>
                )}
                {testStatus === "error" && (
                  <div className="flex flex-col items-center gap-2">
                    <div className="text-sm" style={{ color: "var(--color-error, #f87171)" }}>
                      ❌ Lỗi — không tìm thấy project hoặc automation lỗi
                    </div>
                    <button className="btn btn-ghost text-xs" onClick={() => setTestStatus("idle")}>
                      Thử lại
                    </button>
                  </div>
                )}
              </div>

              <p className="text-xs mt-3" style={{ color: "var(--text-dim)" }}>
                Bỏ qua bước này nếu bạn muốn bắt đầu ngay.
              </p>
            </div>
          </div>
        )}

        {/* ─── Step 4: Hoàn tất ─────────────────────────────── */}
        {step === 4 && (
          <div className="flex flex-col items-center gap-4 py-2">
            <div className="text-4xl">🎉</div>
            <div className="text-base font-bold" style={{ color: "var(--text-pri)" }}>
              Thiết lập hoàn tất!
            </div>
            <div
              className="rounded-lg p-4 w-full text-xs flex flex-col gap-2"
              style={{ background: "var(--bg-surface)", border: "1px solid var(--border)", color: "var(--text-sec)" }}
            >
              <div className="flex gap-2">
                <span style={{ color: "var(--accent)" }}>✓</span>
                <span>Project folder: <span className="font-mono" style={{ color: "var(--text-pri)" }}>{config.project_folder || "—"}</span></span>
              </div>
              <div className="flex gap-2">
                <span style={{ color: "var(--accent)" }}>✓</span>
                <span>Export folder: <span className="font-mono" style={{ color: "var(--text-pri)" }}>{config.export_folder || "—"}</span></span>
              </div>
              <div className="flex gap-2">
                <span style={{ color: "var(--accent)" }}>✓</span>
                <span>Tọa độ đã calibrate: {hasRequiredCoords ? "Đầy đủ" : "Thiếu"}</span>
              </div>
            </div>
            <p className="text-xs text-center" style={{ color: "var(--text-dim)" }}>
              Cấu hình đã được lưu. Bạn có thể điều chỉnh lại bất kỳ lúc nào trong tab Cấu hình.
            </p>
          </div>
        )}

        </div>{/* end step content */}

        {/* ─── Footer navigation ────────────────────────────── */}
        <div
          className="flex justify-between items-center px-8 py-4"
          style={{ borderTop: "1px solid var(--border)", background: "var(--bg-surface)" }}
        >
          <button
            className="btn btn-ghost text-xs"
            onClick={() => step > 1 ? setStep(step - 1) : undefined}
            style={{ visibility: step > 1 ? "visible" : "hidden" }}
          >
            ← Quay lại
          </button>

          <div className="flex gap-2">
            {(step === 2 || step === 3) && (
              <button
                className="btn btn-ghost text-xs"
                onClick={() => setStep(step + 1)}
              >
                Bỏ qua
              </button>
            )}

            {step < TOTAL_STEPS ? (
              <button
                className="btn btn-primary text-xs px-5"
                disabled={
                  (step === 1 && !canProceedStep1) ||
                  (step === 2 && !canProceedStep2)
                }
                onClick={() => setStep(step + 1)}
              >
                Tiếp theo →
              </button>
            ) : (
              <button className="btn btn-primary text-xs px-5" onClick={handleFinish}>
                Bắt đầu dùng AutoCapcut →
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
