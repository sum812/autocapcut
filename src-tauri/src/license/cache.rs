/// cache.rs — Lưu license info trên disk, mã hóa bằng AES-256-GCM.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Compile-time secret — set qua env var APP_SECRET trong CI/CD.
const APP_SECRET: &str = env!("APP_SECRET", "Set APP_SECRET env var at build time");

/// Dữ liệu license được lưu vào disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseData {
    pub license_key: String,
    pub plan: String,
    pub expires_at: Option<String>,
    /// Unix timestamp của lần validate online gần nhất.
    pub last_validated_at: i64,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u8,
    nonce: String,
    ciphertext: String,
}

fn cache_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    let _ = std::fs::create_dir_all(&dir);
    dir.join("license.dat")
}

fn derive_key(machine_id: &str) -> [u8; 32] {
    let password = format!("{}:{}", machine_id, APP_SECRET);
    let salt = b"autocapcut-license-v2";
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 10_000, &mut key);
    key
}

pub fn save(app: &AppHandle, data: &LicenseData, machine_id: &str) -> Result<(), String> {
    let key_bytes = derive_key(machine_id);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

    let plaintext = serde_json::to_vec(data).map_err(|e| e.to_string())?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_slice())
        .map_err(|e| format!("Encrypt thất bại: {}", e))?;

    let file = CacheFile {
        version: 2,
        nonce: B64.encode(nonce.as_slice()),
        ciphertext: B64.encode(&ciphertext),
    };

    let json = serde_json::to_string(&file).map_err(|e| e.to_string())?;
    let path = cache_path(app);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("Ghi file thất bại: {}", e))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Rename thất bại: {}", e))?;
    Ok(())
}

pub fn load(app: &AppHandle, machine_id: &str) -> Option<LicenseData> {
    let path = cache_path(app);
    let json = std::fs::read_to_string(&path).ok()?;
    let file: CacheFile = serde_json::from_str(&json).ok()?;

    if file.version != 2 {
        return None;
    }

    let key_bytes = derive_key(machine_id);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

    let nonce_bytes = B64.decode(&file.nonce).ok()?;
    let ciphertext = B64.decode(&file.ciphertext).ok()?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).ok()?;

    serde_json::from_slice(&plaintext).ok()
}

pub fn clear(app: &AppHandle) {
    let _ = std::fs::remove_file(cache_path(app));
}
