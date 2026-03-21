/// validator.rs — LicenseStatus types (không dùng JWT nữa).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum LicenseStatus {
    /// License hợp lệ
    Valid {
        plan: String,
        expires_at: Option<String>,
    },
    /// License đã hết hạn (subscription expired)
    Expired {
        expired_at: String,
    },
    /// Chưa kích hoạt
    NotActivated,
    /// License không hợp lệ (revoked, key sai, v.v.)
    Invalid {
        reason: String,
    },
}

impl LicenseStatus {
    pub fn is_usable(&self) -> bool {
        matches!(self, LicenseStatus::Valid { .. })
    }
}
