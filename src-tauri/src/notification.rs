/// F16: Notification System
///
/// Gửi Windows toast notification khi:
/// - Mỗi project render xong (notify_per_project=true)
/// - Tất cả batch xong (notify_on_done=true)
/// Dùng tauri-plugin-notification (đã có sẵn trong Cargo.toml).

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

/// Gửi toast notification.
/// Không panic nếu thất bại (notification là optional feature).
pub fn send(app: &AppHandle, title: &str, body: &str) {
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
}

/// Notify khi 1 project render xong.
pub fn notify_project_done(app: &AppHandle, project_name: &str, success: bool) {
    if success {
        send(
            app,
            "CapCut Auto Tool",
            &format!("✅ Render xong: {}", project_name),
        );
    } else {
        send(
            app,
            "CapCut Auto Tool",
            &format!("❌ Render thất bại: {}", project_name),
        );
    }
}

/// Notify khi toàn bộ batch xong.
pub fn notify_batch_done(app: &AppHandle, total: usize, done: usize, failed: usize) {
    let body = if failed == 0 {
        format!("🎉 Hoàn thành {} project!", done)
    } else {
        format!("✅ {} thành công  ❌ {} thất bại / {} tổng", done, failed, total)
    };
    send(app, "CapCut Auto Tool — Render xong", &body);
}

/// Tauri command: gửi test notification từ frontend.
#[tauri::command]
pub fn send_test_notification(app: AppHandle) -> Result<(), String> {
    send(
        &app,
        "CapCut Auto Tool",
        "🔔 Thông báo hoạt động bình thường!",
    );
    Ok(())
}
