use std::io::Write;
use std::sync::OnceLock;
use std::time::SystemTime;
use tauri::{AppHandle, Emitter, Manager};

static LOG_FILE_PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Khởi tạo đường dẫn log file. Gọi 1 lần khi bắt đầu automation.
pub fn init_log_file(app: &AppHandle) {
    LOG_FILE_PATH.get_or_init(|| {
        let data_dir = app
            .path()
            .app_local_data_dir()
            .unwrap_or_else(|_| std::env::temp_dir());
        let _ = std::fs::create_dir_all(&data_dir);
        let log_path = data_dir.join("automation.log");
        if let Ok(mut f) =
            std::fs::OpenOptions::new().create(true).append(true).open(&log_path)
        {
            let _ = writeln!(f, "\n════════════════════════════════════════════");
            let _ = writeln!(f, "Session started: {}", timestamp());
            let _ = writeln!(f, "════════════════════════════════════════════");
        }
        log_path
    });
}

/// Trả về timestamp dạng HH:MM:SS.mmm (UTC).
pub fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() % 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let ms = now.subsec_millis();
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
}

/// Ghi log ra UI (Tauri emit) và file automation.log.
pub fn emit_log(app: &AppHandle, msg: impl AsRef<str>) {
    let msg = msg.as_ref();
    let _ = app.emit("log", msg);
    if let Some(path) = LOG_FILE_PATH.get() {
        if let Ok(mut f) =
            std::fs::OpenOptions::new().create(true).append(true).open(path)
        {
            let _ = writeln!(f, "[{}] {}", timestamp(), msg);
        }
    }
}

/// Tauri command: trả về đường dẫn file log.
#[tauri::command]
pub fn get_log_file_path(app: AppHandle) -> String {
    init_log_file(&app);
    LOG_FILE_PATH
        .get()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
