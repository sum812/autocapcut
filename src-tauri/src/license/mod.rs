/// license/mod.rs — License System dùng autocapcut-api server.

pub mod api_client;
pub mod cache;
pub mod fingerprint;
pub mod gate;
pub mod validator;

use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use validator::LicenseStatus;

/// Grace period: cho phép dùng offline tối đa 7 ngày sau lần validate cuối.
const OFFLINE_GRACE_SECS: i64 = 7 * 24 * 3600;

pub struct LicenseState {
    status: Mutex<LicenseStatus>,
}

impl LicenseState {
    pub fn new() -> Self {
        Self { status: Mutex::new(LicenseStatus::NotActivated) }
    }

    pub fn get(&self) -> LicenseStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or(LicenseStatus::NotActivated)
    }

    fn set(&self, s: LicenseStatus) {
        if let Ok(mut lock) = self.status.lock() {
            *lock = s;
        }
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn status_from_cache(data: &cache::LicenseData) -> LicenseStatus {
    if let Some(ref exp_str) = data.expires_at {
        if let Ok(exp_dt) = chrono::DateTime::parse_from_rfc3339(exp_str) {
            if exp_dt.timestamp() < now_ts() {
                return LicenseStatus::Expired { expired_at: exp_str.clone() };
            }
        }
    }
    LicenseStatus::Valid { plan: data.plan.clone(), expires_at: data.expires_at.clone() }
}

/// Khởi động license check khi app mở.
pub fn init_license(app: &AppHandle) {
    let machine_id = fingerprint::machine_id();
    let state = app.state::<LicenseState>();

    match cache::load(app, &machine_id) {
        None => state.set(LicenseStatus::NotActivated),
        Some(data) => {
            state.set(status_from_cache(&data));

            let app_handle = app.clone();
            let data_clone = data.clone();
            let mid = machine_id.clone();
            std::thread::spawn(move || {
                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    background_validate(&app_handle, &data_clone, &mid).await;
                });
            });
        }
    }
}

async fn background_validate(app: &AppHandle, data: &cache::LicenseData, machine_id: &str) {
    let state = app.state::<LicenseState>();

    match api_client::validate(&data.license_key, machine_id).await {
        Ok(resp) => {
            if resp.valid {
                let plan = resp.plan.unwrap_or_else(|| data.plan.clone());
                let expires_at = resp.expires_at.clone();
                let updated = cache::LicenseData {
                    license_key: data.license_key.clone(),
                    plan: plan.clone(),
                    expires_at: expires_at.clone(),
                    last_validated_at: now_ts(),
                };
                let _ = cache::save(app, &updated, machine_id);
                state.set(LicenseStatus::Valid { plan, expires_at });
            } else {
                let reason = resp.error.unwrap_or_else(|| "License không còn hợp lệ".into());
                cache::clear(app);
                state.set(LicenseStatus::Invalid { reason });
            }
        }
        Err(_) => {
            let elapsed = now_ts() - data.last_validated_at;
            if elapsed > OFFLINE_GRACE_SECS {
                state.set(LicenseStatus::Invalid {
                    reason: "Không thể xác minh license (offline quá 7 ngày).".into(),
                });
            }
        }
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_license_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let state = app.state::<LicenseState>();
    serde_json::to_value(&state.get()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn activate_license(app: AppHandle, key: String) -> Result<serde_json::Value, String> {
    let machine_id = fingerprint::machine_id();
    let resp = api_client::activate(&key, &machine_id).await?;

    let plan = resp.plan.unwrap_or_else(|| "Pro".into());
    let expires_at = resp.expires_at;

    let data = cache::LicenseData {
        license_key: key,
        plan: plan.clone(),
        expires_at: expires_at.clone(),
        last_validated_at: now_ts(),
    };
    cache::save(&app, &data, &machine_id)?;
    app.state::<LicenseState>().set(LicenseStatus::Valid { plan, expires_at });

    get_license_status(app).await
}

#[tauri::command]
pub async fn deactivate_license(app: AppHandle) -> Result<(), String> {
    let machine_id = fingerprint::machine_id();
    if let Some(data) = cache::load(&app, &machine_id) {
        api_client::deactivate(&data.license_key, &machine_id).await?;
    }
    cache::clear(&app);
    app.state::<LicenseState>().set(LicenseStatus::NotActivated);
    Ok(())
}

#[tauri::command]
pub async fn refresh_license(app: AppHandle) -> Result<serde_json::Value, String> {
    let machine_id = fingerprint::machine_id();
    let data = cache::load(&app, &machine_id).ok_or("Chưa kích hoạt license")?;

    let resp = api_client::validate(&data.license_key, &machine_id).await?;

    if resp.valid {
        let plan = resp.plan.unwrap_or_else(|| data.plan.clone());
        let expires_at = resp.expires_at;
        let updated = cache::LicenseData {
            license_key: data.license_key,
            plan: plan.clone(),
            expires_at: expires_at.clone(),
            last_validated_at: now_ts(),
        };
        cache::save(&app, &updated, &machine_id)?;
        app.state::<LicenseState>().set(LicenseStatus::Valid { plan, expires_at });
    } else {
        let reason = resp.error.unwrap_or_else(|| "License không hợp lệ".into());
        cache::clear(&app);
        app.state::<LicenseState>().set(LicenseStatus::Invalid { reason });
    }

    get_license_status(app).await
}

#[tauri::command]
pub fn get_machine_fingerprint() -> String {
    let id = fingerprint::machine_id();
    id[..id.len().min(8)].to_string()
}
