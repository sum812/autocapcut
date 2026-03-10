use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager};

/// Toàn bộ cấu hình được lưu trên disk.
/// Path: %APPDATA%\Local\com.autocapcut.app\config.json
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct AppConfig {
    // Thư mục
    pub project_folder: String,
    pub export_folder: String,

    // Tọa độ calibrate (absolute screen pixels)
    pub first_project_coords: [i32; 2],   // Vị trí project đầu tiên trên CapCut Home
    pub export_box_coords: [i32; 2],      // Nút chọn đường dẫn export trong dialog
    #[serde(default)]
    pub search_button_coords: [i32; 2],   // Nút search trên CapCut Home (0,0 = không dùng)

    // Timing
    pub render_delay: u64,       // Giây chờ CapCut load (default: 10)
    pub render_timeout: u64,     // Phút timeout mỗi project (default: 30)

    // Options
    pub shutdown: bool,          // Tắt máy sau khi render xong
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,        // Số lần retry mỗi project khi thất bại (default: 2)

    // Onboarding
    #[serde(default)]
    pub wizard_completed: bool,  // true sau khi hoàn thành Setup Wizard lần đầu

    // Sync options (F17)
    #[serde(default)]
    pub sync_video_audio: bool,       // Match video/image với audio 1:1
    #[serde(default)]
    pub sync_image_duration: bool,    // Kéo dài ảnh khớp tổng thời lượng audio
    #[serde(default)]
    pub sync_subtitles: bool,         // Đồng bộ video/ảnh với timing subtitle

    // Notification options (F16)
    #[serde(default = "default_notify_on_done")]
    pub notify_on_done: bool,         // Windows toast khi tất cả xong
    #[serde(default)]
    pub notify_per_project: bool,     // Toast sau mỗi project
    #[serde(default = "default_notify_sound")]
    pub notify_sound: bool,           // Phát âm thanh khi xong
}

fn default_max_retries() -> u32 { 2 }
fn default_notify_on_done() -> bool { true }
fn default_notify_sound() -> bool { true }

fn config_path(app: &AppHandle) -> std::path::PathBuf {
    let dir = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    let _ = fs::create_dir_all(&dir);
    dir.join("config.json")
}

#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let path = config_path(&app);
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Lỗi serialize config: {}", e))?;
    // Atomic write: ghi vào .tmp trước, sau đó rename — tránh corrupt nếu crash giữa chừng
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, json).map_err(|e| format!("Lỗi ghi file tạm: {}", e))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Lỗi lưu config: {}", e))?;
    Ok(())
}

#[tauri::command]
pub fn load_config(app: AppHandle) -> Result<Option<AppConfig>, String> {
    let path = config_path(&app);
    if !path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(&path)
        .map_err(|e| format!("Lỗi đọc config: {}", e))?;
    let config: AppConfig = serde_json::from_str(&json)
        .map_err(|e| format!("Lỗi parse config: {}", e))?;
    Ok(Some(config))
}
