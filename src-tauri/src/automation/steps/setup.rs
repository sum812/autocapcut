/// Step 0: Kill → Launch → Wait window → Dismiss popups → Maximize Home
use enigo::Enigo;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

use super::super::helpers::{restore_tool_window, sleep_interruptible};
use super::super::logger::emit_log;
use super::super::window::{kill_capcut_verified, launch_capcut, wait_for_capcut_window};
use super::super::{AutoConfig, AutomationState};
use super::{close_popups_home, maximize_with_verify, StepResult};

pub fn run(
    app: &AppHandle,
    config: &AutoConfig,
    state: &AutomationState,
    enigo: &mut Enigo,
) -> StepResult {
    use std::sync::atomic::Ordering;

    // Minimize tool window để không che CapCut
    emit_log(app, "[step0] Minimize tool window...");
    if let Some(win) = tauri::Manager::get_webview_window(app, "main") {
        let _ = win.minimize();
    }
    thread::sleep(Duration::from_millis(2000));

    // Kill tất cả CapCut hiện có
    emit_log(app, "[step0] Đóng tất cả CapCut hiện có...");
    kill_capcut_verified(app);

    // Launch CapCut
    emit_log(app, "[step0] Mở CapCut...");
    if !launch_capcut(&config.project_path) {
        emit_log(app, "❌ Không tìm thấy CapCut.exe! Kiểm tra lại cài đặt.");
        restore_tool_window(app);
        return StepResult::StopAll;
    }
    emit_log(app, "[step0] ✓ CapCut.exe đã spawn");

    // Wait for CapCut window
    if !wait_for_capcut_window(app, 20, state) {
        if state.should_stop.load(Ordering::SeqCst) {
            restore_tool_window(app);
            return StepResult::StopAll;
        }
        emit_log(app, "❌ CapCut không mở window sau 20s! Hủy.");
        restore_tool_window(app);
        return StepResult::StopAll;
    }
    if state.should_stop.load(Ordering::SeqCst) {
        restore_tool_window(app);
        return StepResult::StopAll;
    }

    // Chờ CapCut khởi động đầy đủ
    emit_log(
        app,
        format!("[step0] Chờ {}s CapCut khởi động đầy đủ...", config.render_delay),
    );
    sleep_interruptible(config.render_delay * 1000, state);
    if state.should_stop.load(Ordering::SeqCst) {
        restore_tool_window(app);
        return StepResult::StopAll;
    }

    // Dismiss popup + Maximize Home
    emit_log(app, "[step0] Đóng popup nếu có...");
    close_popups_home(app, enigo);

    emit_log(app, "[step0] Maximize CapCut Home...");
    if !maximize_with_verify(app, enigo, "Home") {
        emit_log(app, "❌ CapCut Home maximize thất bại — hủy automation");
        restore_tool_window(app);
        return StepResult::StopAll;
    }

    // Chờ Home screen render xong sau maximize (5s cho máy yếu)
    thread::sleep(Duration::from_millis(5000));
    emit_log(app, "[step0] ✓ CapCut Home ready");

    StepResult::Continue
}
