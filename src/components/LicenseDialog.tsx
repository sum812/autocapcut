import { useState } from "react";
import type { LicenseStatus } from "../hooks/useLicense";

interface Props {
  status: LicenseStatus;
  loading: boolean;
  error: string | null;
  onActivate: (key: string) => Promise<boolean>;
  onDeactivate: () => Promise<void>;
  onRefresh: () => Promise<void>;
  onClose: () => void;
}

const PLAN_LABEL: Record<string, string> = {
  monthly: "Monthly (30 ngày)",
  "6months": "6 Tháng",
  lifetime: "Lifetime (vĩnh viễn)",
};

function formatKey(raw: string): string {
  const clean = raw.replace(/[^A-Z0-9]/gi, "").toUpperCase();
  const groups: string[] = [];
  for (let i = 0; i < clean.length && groups.length < 5; i += 5) {
    groups.push(clean.slice(i, i + 5));
  }
  return groups.join("-");
}

export default function LicenseDialog({
  status,
  loading,
  error,
  onActivate,
  onDeactivate,
  onRefresh,
  onClose,
}: Props) {
  const [key, setKey] = useState("");
  const [confirmDeactivate, setConfirmDeactivate] = useState(false);

  const isPro = status.status === "Valid" || status.status === "GracePeriod";
  const formattedKey = formatKey(key);
  const isKeyComplete = formattedKey.length === 29; // XXXXX-XXXXX-XXXXX-XXXXX-XXXXX

  const handleActivate = async () => {
    if (!isKeyComplete) return;
    const ok = await onActivate(formattedKey);
    if (ok) setKey("");
  };

  const handleKeyInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    const raw = e.target.value.replace(/[^A-Z0-9]/gi, "").toUpperCase().slice(0, 25);
    setKey(raw);
  };

  return (
    <div
      className="fixed inset-0 flex items-center justify-center z-50"
      style={{ background: "rgba(0,0,0,0.6)", backdropFilter: "blur(4px)" }}
      onClick={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        className="rounded-xl shadow-2xl p-6 w-[460px]"
        style={{ background: "var(--bg-base)", border: "1px solid var(--border)" }}
      >
        {/* Header */}
        <div className="flex items-center justify-between mb-5">
          <div className="text-sm font-bold" style={{ color: "var(--text-pri)" }}>
            🔑 Quản lý License
          </div>
          <button
            className="text-xs"
            style={{ color: "var(--text-dim)" }}
            onClick={onClose}
          >
            ✕
          </button>
        </div>

        {/* ─── Chưa activate / Activation form ─── */}
        {!isPro && (
          <div className="flex flex-col gap-4">
            {/* Status banner */}
            <div
              className="rounded-lg p-3 text-xs"
              style={{ background: "var(--bg-surface)", border: "1px solid var(--border)" }}
            >
              {status.status === "NotActivated" && (
                <div style={{ color: "var(--text-sec)" }}>
                  <div className="font-medium mb-1" style={{ color: "var(--text-pri)" }}>Free tier</div>
                  Tối đa <strong>3 project</strong> mỗi lần chạy. Nâng cấp để dùng không giới hạn.
                </div>
              )}
              {status.status === "Expired" && (
                <div style={{ color: "#f87171" }}>
                  <div className="font-medium">License đã hết hạn</div>
                  <div className="text-xs mt-0.5" style={{ color: "var(--text-dim)" }}>
                    Hết hạn lúc: {status.expired_at}
                  </div>
                </div>
              )}
              {status.status === "Invalid" && (
                <div style={{ color: "#f87171" }}>
                  <div className="font-medium">License lỗi</div>
                  <div className="text-xs mt-0.5">{status.reason}</div>
                </div>
              )}
            </div>

            {/* Input license key */}
            <div className="flex flex-col gap-2">
              <label className="text-xs font-medium" style={{ color: "var(--text-sec)" }}>
                License Key
              </label>
              <input
                className="input font-mono text-sm tracking-widest"
                placeholder="XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"
                value={formattedKey}
                onChange={handleKeyInput}
                maxLength={29}
                autoFocus
                onKeyDown={(e) => e.key === "Enter" && handleActivate()}
              />
              {error && (
                <span className="text-xs" style={{ color: "#f87171" }}>
                  ❌ {error}
                </span>
              )}
            </div>

            <button
              className="btn btn-primary"
              disabled={!isKeyComplete || loading}
              onClick={handleActivate}
            >
              {loading ? "Đang kích hoạt..." : "Kích hoạt"}
            </button>

            <div className="text-xs text-center" style={{ color: "var(--text-dim)" }}>
              Chưa có license?{" "}
              <a
                href="https://autocapcut.com"
                target="_blank"
                rel="noreferrer"
                style={{ color: "var(--accent)" }}
              >
                Mua tại autocapcut.com →
              </a>
            </div>
          </div>
        )}

        {/* ─── Đã activate: License Info ─── */}
        {isPro && (
          <div className="flex flex-col gap-4">
            <div
              className="rounded-lg p-4 flex flex-col gap-2"
              style={{ background: "var(--bg-surface)", border: "1px solid var(--border)" }}
            >
              <div className="flex items-center gap-2">
                <span style={{ color: "var(--accent)" }}>✅</span>
                <span className="text-sm font-medium" style={{ color: "var(--text-pri)" }}>
                  License đang hoạt động
                </span>
              </div>

              {status.status === "GracePeriod" && (
                <div className="text-xs rounded px-2 py-1" style={{ background: "#78350f20", color: "#f59e0b" }}>
                  ⚠️ Token hết hạn — grace period còn {Math.round((status.expires_in_hours ?? 0) / 24)} ngày. Cần kết nối internet để gia hạn.
                </div>
              )}

              <div className="flex flex-col gap-1 text-xs" style={{ color: "var(--text-sec)" }}>
                <div className="flex justify-between">
                  <span>Plan:</span>
                  <span className="font-medium" style={{ color: "var(--text-pri)" }}>
                    {PLAN_LABEL[status.plan ?? ""] ?? status.plan}
                  </span>
                </div>
                {status.license_expires_at && (
                  <div className="flex justify-between">
                    <span>Hết hạn:</span>
                    <span style={{ color: "var(--text-pri)" }}>
                      {new Date(status.license_expires_at).toLocaleDateString("vi-VN")}
                    </span>
                  </div>
                )}
                {!status.license_expires_at && (
                  <div className="flex justify-between">
                    <span>Hết hạn:</span>
                    <span style={{ color: "var(--accent)" }}>Không bao giờ (Lifetime)</span>
                  </div>
                )}
                <div className="flex justify-between">
                  <span>Projects/batch:</span>
                  <span style={{ color: "var(--text-pri)" }}>
                    {status.max_projects ?? "Không giới hạn"}
                  </span>
                </div>
              </div>
            </div>

            <div className="flex gap-2">
              <button
                className="btn btn-ghost text-xs flex-1"
                onClick={onRefresh}
                disabled={loading}
              >
                {loading ? "Đang gia hạn..." : "↻ Gia hạn token"}
              </button>
              <a
                href="https://autocapcut.com"
                target="_blank"
                rel="noreferrer"
                className="btn btn-ghost text-xs flex-1 text-center"
                style={{ display: "flex", alignItems: "center", justifyContent: "center" }}
              >
                Nâng cấp →
              </a>
            </div>

            {/* Deactivate */}
            {!confirmDeactivate ? (
              <button
                className="text-xs"
                style={{ color: "var(--text-dim)" }}
                onClick={() => setConfirmDeactivate(true)}
              >
                Chuyển sang máy khác (hủy kích hoạt)
              </button>
            ) : (
              <div
                className="rounded-lg p-3 text-xs flex flex-col gap-2"
                style={{ background: "#7f1d1d20", border: "1px solid #f87171" }}
              >
                <div style={{ color: "#f87171" }}>
                  Hủy kích hoạt trên máy này để có thể activate trên máy khác?
                </div>
                <div className="flex gap-2">
                  <button
                    className="btn btn-danger text-xs flex-1"
                    onClick={onDeactivate}
                    disabled={loading}
                  >
                    {loading ? "Đang hủy..." : "Hủy kích hoạt"}
                  </button>
                  <button
                    className="btn btn-ghost text-xs flex-1"
                    onClick={() => setConfirmDeactivate(false)}
                  >
                    Hủy bỏ
                  </button>
                </div>
              </div>
            )}

            {error && (
              <span className="text-xs" style={{ color: "#f87171" }}>❌ {error}</span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
