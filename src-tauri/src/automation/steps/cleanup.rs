/// Step 7: Alt+F4 → Poll wins decrease → Fallback kill+relaunch → Maximize Home
use enigo::{Direction, Enigo, Key, Keyboard};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

use super::super::helpers::sleep_interruptible;
use super::super::logger::emit_log;
use super::super::window::{
    focus_capcut_log, get_capcut_window_count, kill_capcut_verified, launch_capcut,
    wait_for_capcut_window,
};
use super::super::{AutoConfig, AutomationState};
use super::{close_popups_home, maximize_with_verify, StepResult};

pub fn run(
    app: &AppHandle,
    config: &AutoConfig,
    state: &AutomationState,
    enigo: &mut Enigo,
) -> StepResult {
    emit_log(app, "[step7] Đóng project (về Home)...");

    let wins_before_close = get_capcut_window_count();
    focus_capcut_log(app);
    thread::sleep(Duration::from_millis(500));

    let _ = enigo.key(Key::Alt, Direction::Press);
    let _ = enigo.key(Key::F4, Direction::Click);
    let _ = enigo.key(Key::Alt, Direction::Release);

    // Poll wins giảm (editor đóng = về Home)
    let mut back_home = false;
    let close_start = std::time::Instant::now();
    loop {
        let wins_now = get_capcut_window_count();
        if wins_now < wins_before_close {
            back_home = true;
            emit_log(
                app,
                format!(
                    "  [step7] ✓ Về Home (wins: {}→{}, {}ms)",
                    wins_before_close,
                    wins_now,
                    close_start.elapsed().as_millis()
                ),
            );
            break;
        }
        if close_start.elapsed().as_secs() >= 10 {
            emit_log(app, "  [step7] ⚠ Timeout 10s chờ về Home");
            break;
        }
        if state.should_stop.load(Ordering::SeqCst) {
            return StepResult::StopAll;
        }
        thread::sleep(Duration::from_millis(500));
    }

    if !back_home {
        // Fallback: Kill + relaunch nếu Alt+F4 không đóng được
        emit_log(app, "  [step7] Fallback: Kill + relaunch CapCut...");
        kill_capcut_verified(app);
        if !launch_capcut(&config.project_path) {
            emit_log(app, "❌ Không tìm thấy CapCut.exe!");
            return StepResult::StopAll;
        }
        if !wait_for_capcut_window(app, 20, state) {
            emit_log(app, "❌ CapCut không mở window sau 20s!");
            return StepResult::StopAll;
        }
        sleep_interruptible(config.render_delay * 1000, state);
        if state.should_stop.load(Ordering::SeqCst) {
            return StepResult::StopAll;
        }
    }

    // Chờ Home ổn định + dismiss popup + maximize
    thread::sleep(Duration::from_secs(4));
    close_popups_home(app, enigo);
    if !maximize_with_verify(app, enigo, "Home") {
        emit_log(app, "❌ Maximize Home thất bại!");
        return StepResult::StopAll;
    }
    thread::sleep(Duration::from_millis(5000));
    emit_log(app, "[step7] ✓ CapCut Home ready");

    StepResult::Continue
}
