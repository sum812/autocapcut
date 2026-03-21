/// gate.rs — Feature gating dựa trên LicenseStatus.

use super::LicenseState;
use crate::license::validator::LicenseStatus;
use tauri::{AppHandle, Manager};

/// Kiểm tra xem automation có được phép chạy không.
pub fn can_run_automation(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<LicenseState>();
    match state.get() {
        LicenseStatus::Expired { expired_at } => {
            Err(format!("License đã hết hạn ({}).", expired_at))
        }
        LicenseStatus::Invalid { reason } => {
            Err(format!("License không hợp lệ: {}", reason))
        }
        // Valid, NotActivated (free tier) → cho phép
        _ => Ok(()),
    }
}

/// Số project tối đa (None = unlimited).
/// Free tier: 3 projects. Pro: unlimited.
pub fn get_project_limit(app: &AppHandle) -> Option<usize> {
    let state = app.state::<LicenseState>();
    match state.get() {
        LicenseStatus::Valid { .. } => None, // unlimited
        _ => Some(3),                        // free tier
    }
}
