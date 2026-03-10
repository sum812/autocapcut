import { useState } from "react";
import { AppConfig } from "../utils/SettingsContext";

interface Props {
  config: AppConfig;
  onChange: (patch: Partial<AppConfig>) => void;
  onSave: () => void;
  selectedProjects: string[];
  onStartProcessing: () => void;
  isProcessing: boolean;
}

type Tab = "sync" | "advanced";

export default function ControlPanel({
  config,
  onChange,
  onSave,
  selectedProjects,
  onStartProcessing,
  isProcessing,
}: Props) {
  const [tab, setTab] = useState<Tab>("sync");

  return (
    <div className="flex flex-col gap-0 h-full">
      {/* Tab bar */}
      <div className="flex border-b" style={{ borderColor: "var(--border)" }}>
        {(["sync", "advanced"] as Tab[]).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className="px-4 py-2 text-xs font-medium transition-colors"
            style={{
              color: tab === t ? "var(--accent)" : "var(--text-sec)",
              borderBottom: tab === t ? "2px solid var(--accent)" : "2px solid transparent",
              background: "transparent",
            }}
          >
            {t === "sync" ? "Đồng bộ" : "Nâng cao"}
          </button>
        ))}
      </div>

      <div className="flex flex-col gap-4 p-4 flex-1 overflow-y-auto">
        {/* TAB: SYNC */}
        {tab === "sync" && (
          <>
            <Section title="Đồng bộ Media">
              <CheckRow
                label="Đồng bộ Video/Audio"
                hint="Match từng video/ảnh với audio theo thứ tự 1:1"
                checked={config.sync_video_audio}
                onChange={(v) => onChange({ sync_video_audio: v })}
              />
              <CheckRow
                label="Đồng bộ Thời lượng Ảnh"
                hint="Kéo dài ảnh để khớp tổng thời lượng audio"
                checked={config.sync_image_duration}
                onChange={(v) => onChange({ sync_image_duration: v })}
              />
              <CheckRow
                label="Đồng bộ Phụ đề"
                hint="Căn thời điểm video/ảnh theo subtitle timing"
                checked={config.sync_subtitles}
                onChange={(v) => onChange({ sync_subtitles: v })}
              />
            </Section>

            <div
              className="text-xs p-2 rounded"
              style={{ background: "var(--bg-panel)", color: "var(--text-dim)" }}
            >
              ⚠ Sync sẽ backup draft_content.json → .bak trước khi modify.
            </div>
          </>
        )}

        {/* TAB: ADVANCED */}
        {tab === "advanced" && (
          <>
            <Section title="Thời gian">
              <div className="grid grid-cols-2 gap-3">
                <NumberField
                  label="Chờ CapCut mở (giây)"
                  min={3}
                  max={30}
                  value={config.render_delay}
                  onChange={(v) => onChange({ render_delay: v })}
                />
                <NumberField
                  label="Timeout render (phút)"
                  min={5}
                  max={180}
                  value={config.render_timeout}
                  onChange={(v) => onChange({ render_timeout: v })}
                />
                <NumberField
                  label="Retry khi lỗi (lần)"
                  min={0}
                  max={5}
                  value={config.max_retries}
                  onChange={(v) => onChange({ max_retries: v })}
                />
              </div>
            </Section>

            <Section title="Thông báo">
              <CheckRow
                label="Thông báo khi tất cả xong"
                checked={config.notify_on_done}
                onChange={(v) => onChange({ notify_on_done: v })}
              />
              <CheckRow
                label="Thông báo sau mỗi project"
                checked={config.notify_per_project}
                onChange={(v) => onChange({ notify_per_project: v })}
              />
              <CheckRow
                label="Phát âm thanh"
                checked={config.notify_sound}
                onChange={(v) => onChange({ notify_sound: v })}
              />
            </Section>

            <Section title="Hệ thống">
              <CheckRow
                label="Tắt máy sau khi hoàn thành"
                checked={config.shutdown}
                onChange={(v) => onChange({ shutdown: v })}
              />
            </Section>

            <button className="btn btn-ghost text-xs self-start" onClick={onSave}>
              💾 Lưu cài đặt
            </button>
          </>
        )}
      </div>

      {/* Action buttons */}
      <div className="flex flex-col gap-2 p-4 border-t" style={{ borderColor: "var(--border)" }}>
        <button
          className="btn text-sm font-semibold"
          style={{
            background: isProcessing ? "var(--bg-panel)" : "var(--accent)",
            color: isProcessing ? "var(--text-dim)" : "#fff",
            cursor: isProcessing || selectedProjects.length === 0 ? "not-allowed" : "pointer",
            opacity: isProcessing || selectedProjects.length === 0 ? 0.6 : 1,
          }}
          onClick={onStartProcessing}
          disabled={isProcessing || selectedProjects.length === 0}
          title={selectedProjects.length === 0 ? "Chọn ít nhất 1 project" : ""}
        >
          {isProcessing ? "⏳ Đang xử lý..." : "⚡ BẮT ĐẦU XỬ LÝ"}
        </button>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      <div className="text-xs font-semibold uppercase tracking-wider" style={{ color: "var(--text-dim)" }}>
        {title}
      </div>
      <div className="flex flex-col gap-2">{children}</div>
    </div>
  );
}

function CheckRow({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-start gap-2 cursor-pointer">
      <input
        type="checkbox"
        className="w-4 h-4 mt-0.5 accent-[var(--accent)] flex-shrink-0"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
      />
      <div>
        <div className="text-xs" style={{ color: "var(--text-sec)" }}>{label}</div>
        {hint && (
          <div className="text-[11px] mt-0.5" style={{ color: "var(--text-dim)" }}>{hint}</div>
        )}
      </div>
    </label>
  );
}

function NumberField({
  label,
  min,
  max,
  value,
  onChange,
}: {
  label: string;
  min: number;
  max: number;
  value: number;
  onChange: (v: number) => void;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs" style={{ color: "var(--text-sec)" }}>{label}</label>
      <input
        type="number"
        className="input text-xs"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </div>
  );
}
