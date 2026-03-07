/// cache.rs — Lưu/đọc JWT token trên disk, mã hóa bằng AES-256-GCM.
///
/// Encryption key = PBKDF2(machine_fingerprint_id + APP_SECRET).
/// Copy license.dat sang máy khác sẽ không decrypt được (fingerprint khác → key khác).

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

/// Compile-time secret — thay bằng giá trị random thực khi build production.
/// Set qua env var APP_SECRET trong CI/CD.
const APP_SECRET: &str = env!("APP_SECRET", "Set APP_SECRET env var at build time");

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u8,
    nonce: String,       // base64
    ciphertext: String,  // base64
}

fn cache_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    let _ = std::fs::create_dir_all(&dir);
    dir.join("license.dat")
}

fn derive_key(fingerprint_id: &str) -> [u8; 32] {
    let password = format!("{}:{}", fingerprint_id, APP_SECRET);
    let salt = b"autocapcut-license-v1";
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 10_000, &mut key);
    key
}

/// Lưu JWT token vào file mã hóa.
pub fn save_token(app: &AppHandle, token: &str, fingerprint_id: &str) -> Result<(), String> {
    let key_bytes = derive_key(fingerprint_id);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, token.as_bytes())
        .map_err(|e| format!("Encrypt thất bại: {}", e))?;

    let data = CacheFile {
        version: 1,
        nonce: B64.encode(nonce.as_slice()),
        ciphertext: B64.encode(&ciphertext),
    };

    let json = serde_json::to_string(&data).map_err(|e| e.to_string())?;
    let path = cache_path(app);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("Ghi file thất bại: {}", e))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Rename thất bại: {}", e))?;
    Ok(())
}

/// Đọc và giải mã JWT token từ cache.
pub fn load_token(app: &AppHandle, fingerprint_id: &str) -> Option<String> {
    let path = cache_path(app);
    let json = std::fs::read_to_string(&path).ok()?;
    let data: CacheFile = serde_json::from_str(&json).ok()?;

    let key_bytes = derive_key(fingerprint_id);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));

    let nonce_bytes = B64.decode(&data.nonce).ok()?;
    let ciphertext = B64.decode(&data.ciphertext).ok()?;

    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).ok()?;
    String::from_utf8(plaintext).ok()
}

/// Xóa cache (sau khi deactivate).
pub fn clear_token(app: &AppHandle) {
    let path = cache_path(app);
    let _ = std::fs::remove_file(path);
}
