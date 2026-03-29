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
    _idx: usize,
    project_name: &str,
) -> StepResult {
    let step1_start = Instant::now();

    // Dismiss popup trước khi thao tác
    emit_log(app, "[step1] Đóng popup nếu có...");
    close_popups_home(app, enigo);
    emit_log(app, "[step1] Focus CapCut trước double-click...");
    focus_capcut_log(app);
    thread::sleep(Duration::from_millis(4000));

    // Tính số lần cần nhấn Right arrow để đến đúng project trong search results.
    // CapCut sort results theo recency (newest first) — giống scanner.rs.
    // Nếu "0325 (1)" mới hơn "0325", nó xuất hiện trước trong results khi search "0325".
    let search_offset: usize = if config.search_button_coords.0 > 0
        && !config.all_project_names.is_empty()
    {
        let lower_target = project_name.to_lowercase();
        // Số project trong all_project_names có tên CHỨA project_name VÀ đứng TRƯỚC nó
        let offset = config
            .all_project_names
            .iter()
            .take_while(|n| n.as_str() != project_name)
            .filter(|n| n.to_lowercase().contains(&lower_target))
            .count();
        offset
    } else {
        0
    };

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
        emit_log(app, format!("  [step1] ✓ Search done — offset={}", search_offset));
    }

    // Snapshot wins TRƯỚC double-click
    let wins_before = get_capcut_window_count();
    log_diagnostic(app, "Trước double-click");

    // Mở project:
    // - offset=0: double-click trực tiếp tại first_project_coords
    // - offset>0: single-click để focus grid → Right arrow N lần → Enter
    emit_log(
        app,
        format!(
            "[step1] Mở project \"{}\" (offset={}) tại ({}, {})",
            project_name, search_offset, config.first_project_coords.0, config.first_project_coords.1
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

    let do_nav_open = |enigo: &mut Enigo| {
        // Single click → focus/select project tại vị trí 0 trong grid
        let _ = enigo.move_mouse(
            config.first_project_coords.0,
            config.first_project_coords.1,
            Coordinate::Abs,
        );
        thread::sleep(Duration::from_millis(300));
        let _ = enigo.button(Button::Left, Direction::Click);
        thread::sleep(Duration::from_millis(500));
        // Dùng Right arrow để navigate đến vị trí đúng
        for i in 0..search_offset {
            let _ = enigo.key(Key::RightArrow, Direction::Click);
            thread::sleep(Duration::from_millis(200));
            emit_log(app, format!("    [step1] → arrow {}/{}", i + 1, search_offset));
        }
        thread::sleep(Duration::from_millis(300));
        // Enter để mở project đã chọn
        let _ = enigo.key(Key::Return, Direction::Click);
    };

    if search_offset == 0 {
        do_double_click(enigo);
    } else {
        do_nav_open(enigo);
    }
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
        emit_log(app, "  [step1] ⚠ Project chưa mở — retry...");
        focus_capcut_log(app);
        thread::sleep(Duration::from_millis(1000));
        if search_offset == 0 {
            do_double_click(enigo);
        } else {
            do_nav_open(enigo);
        }
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
    // QUAN TRỌNG: Phải chờ CapCut thoát "Not Responding" TRƯỚC khi gửi Esc,
    // vì project nặng có thể làm CapCut freeze 10-30s sau khi mở.
    // Nếu gửi Esc khi frozen → bị nuốt → dialog còn → Ctrl+E bị block.
    emit_log(app, "[step1] Chờ CapCut responsive (tối đa 30s) trước khi dismiss dialog...");
    #[cfg(target_os = "windows")]
    {
        use super::super::window::win_focus;
        let nr_start = Instant::now();
        while win_focus::is_capcut_not_responding() {
            if nr_start.elapsed().as_secs() >= 30 {
                emit_log(app, "  [step1] ⚠ CapCut vẫn Not Responding sau 30s — tiếp tục");
                break;
            }
            if state.should_stop.load(Ordering::SeqCst) {
                return StepResult::StopAll;
            }
            thread::sleep(Duration::from_millis(1000));
        }
        if nr_start.elapsed().as_secs() > 0 {
            emit_log(
                app,
                format!(
                    "  [step1] CapCut responsive sau {}s",
                    nr_start.elapsed().as_secs()
                ),
            );
        }
    }

    // Esc x3: an toàn vì nếu không có dialog thì bị ignore,
    // nếu có dialog → dismiss → render tiếp (CapCut render thiếu media thay vì stuck).
    emit_log(app, "[step1] Dismiss potential Missing Media dialog (Esc x3)...");
    focus_capcut_log(app);
    thread::sleep(Duration::from_millis(800));
    let _ = enigo.key(Key::Escape, Direction::Click);
    thread::sleep(Duration::from_millis(500));
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
