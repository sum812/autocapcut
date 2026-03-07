/// license/mod.rs — Public API cho License System.
///
/// Tauri commands:
/// - `get_license_status`  — trả về LicenseStatus hiện tại (từ cache + JWT verify)
/// - `activate_license`    — kích hoạt license key mới
/// - `deactivate_license`  — hủy kích hoạt để giải phóng slot
/// - `refresh_license`     — gọi online để refresh token

pub mod api_client;
pub mod cache;
pub mod fingerprint;
pub mod gate;
pub mod validator;

use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use validator::LicenseStatus;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Shared state lưu LicenseStatus hiện tại (JSON string để dễ serialize qua IPC).
pub struct LicenseState {
    status_json: Mutex<Option<String>>,
    token: Mutex<Option<String>>,
}

impl LicenseState {
    pub fn new() -> Self {
        Self {
            status_json: Mutex::new(None),
            token: Mutex::new(None),
        }
    }

    pub fn get_status(&self) -> Option<String> {
        self.status_json.lock().ok()?.clone()
    }

    fn set_status(&self, status: &LicenseStatus) {
        if let Ok(mut lock) = self.status_json.lock() {
            *lock = serde_json::to_string(status).ok();
        }
    }

    fn get_token(&self) -> Option<String> {
        self.token.lock().ok()?.clone()
    }

    fn set_token(&self, token: Option<String>) {
        if let Ok(mut lock) = self.token.lock() {
            *lock = token;
        }
    }
}

/// Khởi động license check khi app mở.
/// Đọc cache → verify JWT → set LicenseState.
/// Gọi trong setup() của lib.rs.
pub fn init_license(app: &AppHandle) {
    let fingerprint = fingerprint::generate_fingerprint();
    let token = cache::load_token(app, &fingerprint.id);

    let state = app.state::<LicenseState>();

    match token {
        None => {
            state.set_status(&LicenseStatus::NotActivated);
        }
        Some(ref t) => {
            let status = validator::compute_status(t, &fingerprint.id);
            state.set_status(&status);
            state.set_token(Some(t.clone()));

            // Background refresh nếu token sắp hết hạn (< 5 ngày)
            if let LicenseStatus::Valid { days_until_token_expire, .. } = &status {
                if *days_until_token_expire < 5 {
                    let app_handle = app.clone();
                    let token_clone = t.clone();
                    let fp_clone = fingerprint.clone();
                    std::thread::spawn(move || {
                        tokio::runtime::Runtime::new().unwrap().block_on(async {
                            background_refresh(&app_handle, &token_clone, &fp_clone).await;
                        });
                    });
                }
            }
        }
    }
}

async fn background_refresh(
    app: &AppHandle,
    token: &str,
    fingerprint: &fingerprint::MachineFingerprint,
) {
    match api_client::validate(token, fingerprint, APP_VERSION).await {
        Ok(resp) => {
            let state = app.state::<LicenseState>();
            let _ = cache::save_token(app, &resp.token, &fingerprint.id);
            let new_status = validator::compute_status(&resp.token, &fingerprint.id);
            state.set_status(&new_status);
            state.set_token(Some(resp.token));
        }
        Err(_) => {
            // Lỗi network khi refresh ngầm → không làm gì, giữ nguyên status
        }
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────────────────

/// Trả về LicenseStatus hiện tại dưới dạng JSON.
#[tauri::command]
pub async fn get_license_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let state = app.state::<LicenseState>();
    let json_str = state.get_status().unwrap_or_else(|| {
        serde_json::to_string(&LicenseStatus::NotActivated).unwrap()
    });
    serde_json::from_str(&json_str).map_err(|e| e.to_string())
}

/// Kích hoạt license key.
#[tauri::command]
pub async fn activate_license(app: AppHandle, key: String) -> Result<serde_json::Value, String> {
    let fingerprint = fingerprint::generate_fingerprint();

    let resp = api_client::activate(&key, &fingerprint, APP_VERSION).await?;

    // Lưu token
    cache::save_token(&app, &resp.token, &fingerprint.id)?;

    // Cập nhật state
    let state = app.state::<LicenseState>();
    let status = validator::compute_status(&resp.token, &fingerprint.id);
    state.set_status(&status);
    state.set_token(Some(resp.token));

    // Trả về status mới
    get_license_status(app).await
}

/// Hủy kích hoạt (deactivate).
#[tauri::command]
pub async fn deactivate_license(app: AppHandle) -> Result<(), String> {
    let fingerprint = fingerprint::generate_fingerprint();
    let state = app.state::<LicenseState>();

    let token = state
        .get_token()
        .ok_or_else(|| "Chưa có token để deactivate".to_string())?;

    api_client::deactivate(&token, &fingerprint.id).await?;

    cache::clear_token(&app);
    state.set_status(&LicenseStatus::NotActivated);
    state.set_token(None);

    Ok(())
}

/// Refresh token online thủ công (user gọi nếu muốn).
#[tauri::command]
pub async fn refresh_license(app: AppHandle) -> Result<serde_json::Value, String> {
    let fingerprint = fingerprint::generate_fingerprint();
    let state = app.state::<LicenseState>();

    let token = state
        .get_token()
        .ok_or_else(|| "Chưa kích hoạt license".to_string())?;

    let resp = api_client::validate(&token, &fingerprint, APP_VERSION).await?;

    cache::save_token(&app, &resp.token, &fingerprint.id)?;
    let status = validator::compute_status(&resp.token, &fingerprint.id);
    state.set_status(&status);
    state.set_token(Some(resp.token));

    get_license_status(app).await
}

/// Trả về machine fingerprint (để hiển thị cho user nếu cần support).
#[tauri::command]
pub fn get_machine_fingerprint() -> serde_json::Value {
    let fp = fingerprint::generate_fingerprint();
    serde_json::json!({
        "id": &fp.id[..8],  // Chỉ show 8 ký tự đầu để privacy
        "component_count": fp.component_count,
    })
}
