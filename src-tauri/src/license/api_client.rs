/// api_client.rs — HTTP calls đến license server.

use serde::{Deserialize, Serialize};
use crate::license::fingerprint::MachineFingerprint;

const API_BASE: &str = "https://api.aicoachtools.com";

#[derive(Serialize)]
struct ActivateRequest<'a> {
    key: &'a str,
    fingerprint_id: &'a str,
    fingerprint_components: &'a [crate::license::fingerprint::FingerprintComponent],
    app_version: &'a str,
    platform: &'a str,
}

#[derive(Serialize)]
struct ValidateRequest<'a> {
    fingerprint_id: &'a str,
    fingerprint_components: &'a [crate::license::fingerprint::FingerprintComponent],
    app_version: &'a str,
}

#[derive(Serialize)]
struct DeactivateRequest<'a> {
    fingerprint_id: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct LicenseResponse {
    pub token: String,
    pub plan: String,
    pub license_expires_at: Option<String>,
    pub max_projects: Option<u32>,
    pub features: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    error: String,
    message: String,
}

/// Gửi yêu cầu activate license key.
pub async fn activate(
    key: &str,
    fingerprint: &MachineFingerprint,
    app_version: &str,
) -> Result<LicenseResponse, String> {
    let body = serde_json::to_string(&ActivateRequest {
        key,
        fingerprint_id: &fingerprint.id,
        fingerprint_components: &fingerprint.components,
        app_version,
        platform: "windows-x64",
    })
    .map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(format!("{}/api/v1/license/activate", API_BASE))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối server: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if status.is_success() {
        serde_json::from_str(&text).map_err(|e| format!("Lỗi parse response: {}", e))
    } else {
        let err: ApiError = serde_json::from_str(&text)
            .unwrap_or(ApiError { error: "UNKNOWN".into(), message: text });
        Err(err.message)
    }
}

/// Refresh JWT token (gọi ngầm khi token sắp hết hạn).
pub async fn validate(
    current_token: &str,
    fingerprint: &MachineFingerprint,
    app_version: &str,
) -> Result<LicenseResponse, String> {
    let body = serde_json::to_string(&ValidateRequest {
        fingerprint_id: &fingerprint.id,
        fingerprint_components: &fingerprint.components,
        app_version,
    })
    .map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(format!("{}/api/v1/license/validate", API_BASE))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", current_token))
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối server: {}", e))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if status.is_success() {
        serde_json::from_str(&text).map_err(|e| format!("Lỗi parse response: {}", e))
    } else {
        let err: ApiError = serde_json::from_str(&text)
            .unwrap_or(ApiError { error: "UNKNOWN".into(), message: text });
        Err(err.message)
    }
}

/// Hủy kích hoạt (deactivate) để giải phóng slot.
pub async fn deactivate(
    current_token: &str,
    fingerprint_id: &str,
) -> Result<(), String> {
    let body = serde_json::to_string(&DeactivateRequest { fingerprint_id })
        .map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(format!("{}/api/v1/license/deactivate", API_BASE))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", current_token))
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối server: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let text = resp.text().await.unwrap_or_default();
        let err: ApiError = serde_json::from_str(&text)
            .unwrap_or(ApiError { error: "UNKNOWN".into(), message: text });
        Err(err.message)
    }
}
