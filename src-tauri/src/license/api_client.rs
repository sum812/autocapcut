/// api_client.rs — Gọi autocapcut-api server.

use serde::{Deserialize, Serialize};

const API_BASE: &str = "https://api.aicoachtools.com/api/v1/license";

#[derive(Serialize)]
struct ActivateBody<'a> {
    key: &'a str,
    machine_id: &'a str,
}

#[derive(Serialize)]
struct ValidateBody<'a> {
    key: &'a str,
    machine_id: &'a str,
}

#[derive(Serialize)]
struct DeactivateBody<'a> {
    key: &'a str,
    machine_id: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct LicenseResponse {
    pub valid: bool,
    pub plan: Option<String>,
    pub expires_at: Option<String>,
    pub error: Option<String>,
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

/// Kích hoạt license key.
pub async fn activate(license_key: &str, machine_id: &str) -> Result<LicenseResponse, String> {
    let client = build_client()?;
    let resp = client
        .post(format!("{}/activate", API_BASE))
        .json(&ActivateBody { key: license_key, machine_id })
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối: {}", e))?;

    let data: LicenseResponse = resp.json().await
        .map_err(|e| format!("Lỗi parse response: {}", e))?;

    if !data.valid {
        return Err(data.error.unwrap_or_else(|| "Kích hoạt thất bại".into()));
    }
    Ok(data)
}

/// Kiểm tra license còn hợp lệ không (gọi khi khởi động app).
pub async fn validate(license_key: &str, machine_id: &str) -> Result<LicenseResponse, String> {
    let client = build_client()?;
    let resp = client
        .post(format!("{}/validate", API_BASE))
        .json(&ValidateBody { key: license_key, machine_id })
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối: {}", e))?;

    resp.json().await
        .map_err(|e| format!("Lỗi parse response: {}", e))
}

/// Hủy kích hoạt để giải phóng slot.
pub async fn deactivate(license_key: &str, machine_id: &str) -> Result<(), String> {
    let client = build_client()?;
    let resp = client
        .post(format!("{}/deactivate", API_BASE))
        .json(&DeactivateBody { key: license_key, machine_id })
        .send()
        .await
        .map_err(|e| format!("Lỗi kết nối: {}", e))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let data: serde_json::Value = resp.json().await.unwrap_or_default();
        Err(data["error"].as_str().unwrap_or("Deactivate thất bại").to_string())
    }
}
