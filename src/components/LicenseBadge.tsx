import type { LicenseStatus } from "../hooks/useLicense";

interface Props {
  status: LicenseStatus;
  onClick: () => void;
}

const PLAN_LABEL: Record<string, string> = {
  monthly: "Monthly",
  "6months": "6 Tháng",
  lifetime: "Lifetime",
};

export default function LicenseBadge({ status, onClick }: Props) {
  let text = "";
  let color = "var(--text-dim)";
  let bg = "transparent";
  let border = "1px solid var(--border)";

  switch (status.status) {
    case "Valid": {
      const plan = PLAN_LABEL[status.plan ?? ""] ?? status.plan;
      const daysLeft = status.days_until_token_expire ?? 0;
      text = daysLeft < 7
        ? `✅ Pro ${plan} — còn ${daysLeft} ngày`
        : `✅ Pro ${plan}`;
      color = "var(--accent)";
      border = "1px solid var(--accent)";
      break;
    }
    case "GracePeriod":
      text = `⚠️ Grace — còn ${Math.round((status.expires_in_hours ?? 0) / 24)} ngày`;
      color = "#f59e0b";
      border = "1px solid #f59e0b";
      break;
    case "Expired":
      text = "❌ License hết hạn";
      color = "#f87171";
      border = "1px solid #f87171";
      break;
    case "NotActivated":
      text = "🔑 Free (3 project)";
      color = "var(--text-dim)";
      break;
    case "Invalid":
      text = "⚠️ License lỗi";
      color = "#f87171";
      border = "1px solid #f87171";
      break;
  }

  return (
    <button
      onClick={onClick}
      className="text-xs px-2 py-1 rounded transition-opacity hover:opacity-80"
      style={{ color, border, background: bg, cursor: "pointer" }}
      title="Nhấn để quản lý license"
    >
      {text}
    </button>
  );
}
