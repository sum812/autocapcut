/// Steps 1-2: Search → Double-click → Verify open (poll wins) → Maximize Editor
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;

use super::super::logger::emit_log;
use super::super::window::{focus_capcut_log, get_capcut_window_count, log_diagnostic};
use super::super::{AutoConfig, AutomationState};
use super::{close_popups_editor, close_popups_home, maximize_with_verify, StepResult};

pub fn run(
    app: &AppHandle,
    config: &AutoConfig,
    state: &AutomationState,
    enigo: &mut Enigo,
    idx: usize,
    project_name: &str,
) -> StepResult {
    let step1_start = Instant::now();

    // Dismiss popup trước khi thao tác
    emit_log(app, "[step1] Đóng popup nếu có...");
    close_popups_home(app, enigo);
    emit_log(app, "[step1] Focus CapCut trước double-click...");
    focus_capcut_log(app);
    thread::sleep(Duration::from_millis(4000));

    // Search project theo tên (nếu đã calibrate search button)
    if config.search_button_coords.0 > 0 && config.search_button_coords.1 > 0 {
        emit_log(app, format!("[step1] Search: \"{}\"", project_name));
        let _ = enigo.move_mouse(
            config.search_button_coords.0,
            config.search_button_coords.1,
            Coordinate::Abs,
        );
        thread::sleep(Duration::from_millis(1000));
        let _ = enigo.button(Button::Left, Direction::Click);
        thread::sleep(Duration::from_millis(1500));
        // Ctrl+A xóa text cũ
        let _ = enigo.key(Key::Control, Direction::Press);
        let _ = enigo.key(Key::Unicode('a'), Direction::Click);
        let _ = enigo.key(Key::Control, Direction::Release);
        thread::sleep(Duration::from_millis(200));
        let _ = enigo.text(project_name);
        thread::sleep(Duration::from_millis(3000));
        emit_log(app, "  [step1] ✓ Search done — chờ filter");
    }

    // Snapshot wins TRƯỚC double-click
    let wins_before = get_capcut_window_count();
    log_diagnostic(app, "Trước double-click");

    emit_log(
        app,
        format!(
            "[step1] Mở project \"{}\" — double-click tại ({}, {})",
            project_name, config.first_project_coords.0, config.first_project_coords.1
        ),
    );

    let do_double_click = |enigo: &mut Enigo| {
        let _ = enigo.move_mouse(
            config.first_project_coords.0,
            config.first_project_coords.1,
            Coordinate::Abs,
        );
        thread::sleep(Duration::from_millis(300));
        // Press+Release riêng để kiểm soát DOWN-to-DOWN timing (< 500ms threshold)
        let _ = enigo.button(Button::Left, Direction::Press);
        let _ = enigo.button(Button::Left, Direction::Release);
        thread::sleep(Duration::from_millis(50));
        let _ = enigo.button(Button::Left, Direction::Press);
        let _ = enigo.button(Button::Left, Direction::Release);
    };

    do_double_click(enigo);
    emit_log(app, "  [step1] Double-click sent — chờ project mở...");

    // Poll verify project mở (wins tăng = new Electron BrowserWindow)
    let mut project_opened = false;
    let open_start = Instant::now();
    loop {
        let wins_now = get_capcut_window_count();
        if wins_now > wins_before {
            project_opened = true;
            emit_log(
                app,
                format!(
                    "  [step1] ✓ Project mở (wins: {}→{}, {}ms)",
                    wins_before,
                    wins_now,
                    open_start.elapsed().as_millis()
                ),
            );
            thread::sleep(Duration::from_secs(3));
            break;
        }
        if open_start.elapsed().as_secs() >= config.render_delay {
            break;
        }
        if state.should_stop.load(Ordering::SeqCst) {
            return StepResult::StopAll;
        }
        thread::sleep(Duration::from_millis(500));
    }

    // Retry 1 lần nếu chưa mở
    if !project_opened {
        emit_log(app, "  [step1] ⚠ Project chưa mở — retry double-click...");
        focus_capcut_log(app);
        thread::sleep(Duration::from_millis(1000));
        do_double_click(enigo);
        let retry_start = Instant::now();
        loop {
            let wins_now = get_capcut_window_count();
            if wins_now > wins_before {
                project_opened = true;
                emit_log(
                    app,
                    format!(
                        "  [step1] ✓ Retry OK (wins: {}→{}, {}ms)",
                        wins_before,
                        wins_now,
                        retry_start.elapsed().as_millis()
                    ),
                );
                thread::sleep(Duration::from_secs(3));
                break;
            }
            if retry_start.elapsed().as_secs() >= 10 {
                break;
            }
            if state.should_stop.load(Ordering::SeqCst) {
                return StepResult::StopAll;
            }
            thread::sleep(Duration::from_millis(500));
        }
    }

    if !project_opened {
        emit_log(app, format!("❌ Không mở được '{}' — bỏ qua", project_name));
        return StepResult::SkipProject;
    }

    log_diagnostic(app, "Sau project mở");
    emit_log(
        app,
        format!("[step1] done ({:.1}s)", step1_start.elapsed().as_secs_f64()),
    );

    // Dismiss "Missing Media" DOM dialog (nếu có).
    // Dialog này là Electron DOM overlay → FindWindowW không detect được.
    // Esc 2 lần: an toàn vì nếu không có dialog thì bị ignore,
    // nếu có dialog → dismiss → render tiếp (có thể thiếu media nhưng vẫn ra file).
    emit_log(app, "[step1] Dismiss potential Missing Media dialog (Esc x2)...");
    focus_capcut_log(app);
    thread::sleep(Duration::from_millis(1000));
    let _ = enigo.key(Key::Escape, Direction::Click);
    thread::sleep(Duration::from_millis(500));
    let _ = enigo.key(Key::Escape, Direction::Click);
    thread::sleep(Duration::from_millis(500));

    // Step 2: Maximize Editor
    close_popups_editor(app);
    emit_log(app, "[step2] Maximize CapCut editor...");
    maximize_with_verify(app, enigo, "Editor");

    StepResult::Continue
}
