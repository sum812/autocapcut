/// Recovery: Kill CapCut → Relaunch → Wait → Maximize Home
/// Gọi trước mỗi lần retry project khi automation bị lỗi.
use enigo::Enigo;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

use super::super::helpers::{emit_project_status, sleep_interruptible};
use super::super::logger::emit_log;
use super::super::window::{kill_capcut_verified, launch_capcut, wait_for_capcut_window};
use super::super::{AutoConfig, AutomationState};
use super::{close_popups_home, maximize_with_verify};

/// Kill + relaunch CapCut, chờ Home, maximize.
/// Returns `true` nếu CapCut sẵn sàng ở Home sau recovery.
pub fn recover_capcut(
    app: &AppHandle,
    config: &AutoConfig,
    state: &AutomationState,
    enigo: &mut Enigo,
) -> bool {
    emit_log(app, "🔄 [recovery] Kill + relaunch CapCut để phục hồi...");

    kill_capcut_verified(app);
    thread::sleep(Duration::from_secs(2));

    if !launch_capcut(&config.project_path) {
        emit_log(app, "❌ [recovery] Không tìm thấy CapCut.exe!");
        return false;
    }

    if !wait_for_capcut_window(app, 30, state) {
        emit_log(app, "❌ [recovery] CapCut không mở window sau 30s!");
        return false;
    }

    // Chờ CapCut khởi động ổn định
    sleep_interruptible(config.render_delay * 1000, state);
    if state.should_stop.load(Ordering::SeqCst) {
        return false;
    }

    close_popups_home(app, enigo);
    if !maximize_with_verify(app, enigo, "Home (recovery)") {
        emit_log(app, "❌ [recovery] Maximize Home thất bại!");
        return false;
    }

    thread::sleep(Duration::from_secs(3));
    emit_log(app, "✅ [recovery] CapCut đã phục hồi, sẵn sàng retry");
    true
}

/// Emit trạng thái retry cho project (để UI cập nhật badge).
pub fn emit_retry_status(app: &AppHandle, project_name: &str, attempt: u32, max: u32) {
    emit_project_status(app, project_name, "Retrying");
    emit_log(
        app,
        format!(
            "  🔄 Retry {}/{} cho project '{}'...",
            attempt, max, project_name
        ),
    );
}
