use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use tauri::{AppHandle, Emitter, Manager};

use super::AutomationState;

/// Restore cửa sổ tool: unminimize và focus lại.
pub fn restore_tool_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

/// Emit event `project-status` để cập nhật trạng thái project trên UI.
pub fn emit_project_status(app: &AppHandle, name: &str, status: &str) {
    let _ = app.emit(
        "project-status",
        serde_json::json!({ "name": name, "status": status }),
    );
}

/// Validate export folder: tồn tại, là directory, và có quyền đọc.
pub fn validate_export_path(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("Export folder không tồn tại: {}", path));
    }
    if !p.is_dir() {
        return Err(format!("Export path không phải thư mục: {}", path));
    }
    std::fs::read_dir(path)
        .map_err(|e| format!("Không có quyền đọc export folder: {}", e))?;
    Ok(())
}

/// Lấy danh sách tên file trong thư mục (snapshot để so sánh trước/sau render).
pub fn get_files_in_dir(path: &str) -> HashSet<String> {
    let mut files = HashSet::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                files.insert(name);
            }
        }
    }
    files
}

/// Poll export folder cho đến khi xuất hiện video file mới hoặc file cũ bị overwrite.
/// Trả về true nếu tìm thấy trong timeout_secs, false nếu timeout hoặc user dừng.
pub fn wait_for_new_video_file(
    app: &AppHandle,
    export_path: &str,
    before_files: &HashSet<String>,
    timeout_secs: u64,
    state: &AutomationState,
) -> bool {
    use super::logger::emit_log;

    let start = Instant::now();
    let export_started_at = SystemTime::now();
    let video_exts = ["mp4", "mov", "avi", "mkv", "webm", "m4v", "wmv", "flv"];
    let mut last_heartbeat = 0u64;

    loop {
        if state.should_stop.load(Ordering::SeqCst) {
            emit_log(app, "  [poll] Bị dừng bởi user");
            return false;
        }
        let elapsed = start.elapsed().as_secs();
        if elapsed >= timeout_secs {
            emit_log(app, format!("  [poll] ✗ Timeout sau {}s", elapsed));
            return false;
        }

        // Heartbeat mỗi 30s
        if elapsed >= last_heartbeat + 30 {
            let file_count =
                std::fs::read_dir(export_path).map(|rd| rd.count()).unwrap_or(0);
            emit_log(
                app,
                format!(
                    "  [poll] Chờ render... {}s/{}s | files: {}",
                    elapsed, timeout_secs, file_count
                ),
            );
            last_heartbeat = elapsed;
        }

        thread::sleep(Duration::from_secs(2));

        let current_files = match std::fs::read_dir(export_path) {
            Ok(rd) => rd,
            Err(e) => {
                emit_log(app, format!("  [poll] ⚠ read_dir failed: {}", e));
                continue;
            }
        };

        for entry in current_files.flatten() {
            let name = match entry.file_name().into_string() {
                Ok(n) => n,
                Err(_) => continue,
            };
            let lower = name.to_lowercase();
            if lower.starts_with('.') || lower.contains(".tmp") || lower.contains('~') {
                continue;
            }
            let ext = lower.rsplit('.').next().unwrap_or("");
            if !video_exts.contains(&ext) {
                continue;
            }

            let full_path = entry.path();
            let is_new = !before_files.contains(&name);
            let recently_modified = std::fs::metadata(&full_path)
                .and_then(|m| m.modified())
                .map(|mtime| mtime >= export_started_at)
                .unwrap_or(false);

            if is_new || recently_modified {
                let size_mb = std::fs::metadata(&full_path)
                    .map(|m| m.len() as f64 / 1_048_576.0)
                    .unwrap_or(0.0);
                emit_log(
                    app,
                    format!(
                        "  [poll] Candidate: '{}' (new={}, mtime_ok={}, {:.1}MB)",
                        name, is_new, recently_modified, size_mb
                    ),
                );

                // Check size stability
                let size1 = std::fs::metadata(&full_path).map(|m| m.len()).unwrap_or(0);
                thread::sleep(Duration::from_millis(2000));
                let size2 = std::fs::metadata(&full_path).map(|m| m.len()).unwrap_or(0);
                let stable = size1 == size2 && size1 > 0;

                if stable {
                    emit_log(
                        app,
                        format!(
                            "  [poll] ✓ File ready: '{}' ({:.1}MB, stable)",
                            name,
                            size2 as f64 / 1_048_576.0
                        ),
                    );
                    return true;
                } else {
                    emit_log(
                        app,
                        format!(
                            "  [poll] Size changing: {} {:.1}→{:.1}MB...",
                            name,
                            size1 as f64 / 1_048_576.0,
                            size2 as f64 / 1_048_576.0
                        ),
                    );
                }
            }
        }
    }
}

/// Sleep có thể bị interrupt khi should_stop = true. Kiểm tra flag mỗi 100ms.
pub fn sleep_interruptible(ms: u64, state: &AutomationState) {
    let steps = ms / 100;
    for _ in 0..steps {
        if state.should_stop.load(Ordering::SeqCst) {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
}
