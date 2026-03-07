import { AppConfig } from "../utils/SettingsContext";

interface Props {
  config: AppConfig;
  onChange: (patch: Partial<AppConfig>) => void;
  onSave: () => void;
}

export default function ControlPanel({ config, onChange, onSave }: Props) {
  return (
    <div className="flex flex-col gap-4 p-4">
      <div className="text-xs font-semibold uppercase tracking-wider" style={{ color: "var(--text-dim)" }}>
        Cài đặt
      </div>

      <div className="grid grid-cols-2 gap-3">
        {/* Render delay */}
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-sec)" }}>
            Chờ CapCut mở (giây)
          </label>
          <input
            type="number"
            className="input text-xs"
            min={3}
            max={30}
            value={config.render_delay}
            onChange={(e) => onChange({ render_delay: Number(e.target.value) })}
          />
        </div>

        {/* Render timeout */}
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-sec)" }}>
            Timeout render (phút)
          </label>
          <input
            type="number"
            className="input text-xs"
            min={5}
            max={180}
            value={config.render_timeout}
            onChange={(e) => onChange({ render_timeout: Number(e.target.value) })}
          />
        </div>

        {/* Max retries */}
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-sec)" }}>
            Retry khi lỗi (lần)
          </label>
          <input
            type="number"
            className="input text-xs"
            min={0}
            max={5}
            value={config.max_retries}
            onChange={(e) => onChange({ max_retries: Number(e.target.value) })}
          />
        </div>
      </div>

      {/* Shutdown option */}
      <label className="flex items-center gap-2 cursor-pointer">
        <input
          type="checkbox"
          className="w-4 h-4 accent-[var(--accent)]"
          checked={config.shutdown}
          onChange={(e) => onChange({ shutdown: e.target.checked })}
        />
        <span className="text-xs" style={{ color: "var(--text-sec)" }}>
          Tắt máy sau khi hoàn thành
        </span>
      </label>

      <button className="btn btn-ghost text-xs self-start" onClick={onSave}>
        💾 Lưu cài đặt
      </button>
    </div>
  );
}
