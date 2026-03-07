/// gate.rs — Feature gating dựa trên LicenseStatus.

use super::LicenseState;
use tauri::{AppHandle, Manager};

/// Số project tối đa cho mỗi batch.
/// None = unlimited (Pro), Some(n) = giới hạn.
pub fn get_project_limit(app: &AppHandle) -> Option<usize> {
    let state = app.state::<LicenseState>();
    let status = state.get_status();

    match status {
        Some(s) if s.contains("\"status\":\"Valid\"") || s.contains("\"status\":\"GracePeriod\"") => {
            // Parse max_projects từ JSON status string
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(mp) = v.get("max_projects").and_then(|x| x.as_u64()) {
                    return Some(mp as usize);
                }
                return None; // unlimited
            }
            None
        }
        _ => Some(3), // Free tier: 3 projects
    }
}

/// Kiểm tra xem automation có được phép chạy không.
pub fn can_run_automation(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<LicenseState>();
    let status = state.get_status();

    match status {
        Some(s) if s.contains("\"status\":\"Valid\"") || s.contains("\"status\":\"GracePeriod\"") => Ok(()),
        Some(s) if s.contains("\"status\":\"NotActivated\"") => Ok(()), // Free tier được chạy (nhưng limit 3)
        Some(s) if s.contains("\"status\":\"Expired\"") => {
            Err("License đã hết hạn. Vui lòng gia hạn tại autocapcut.com".into())
        }
        _ => Ok(()), // Default: cho phép (free tier)
    }
}
