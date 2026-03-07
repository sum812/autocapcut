/// Steps 3-5: Re-maximize → Focus → Ctrl+E → Set export path → Enter
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse};
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

use super::super::logger::emit_log;
use super::super::window::{focus_capcut_log, log_diagnostic};
use super::super::{AutoConfig, AutomationState};
use super::StepResult;

pub fn run(
    app: &AppHandle,
    config: &AutoConfig,
    _state: &AutomationState,
    enigo: &mut Enigo,
    idx: usize,
) -> StepResult {
    // QUAN TRỌNG: Maximize TRƯỚC khi gửi Ctrl+E.
    // Windows dialog có position "fixed" khi mở — không di chuyển khi parent resize.
    #[cfg(target_os = "windows")]
    {
        use super::super::window::win_focus;
        if !win_focus::is_capcut_maximized() {
            emit_log(app, "[step3] Re-maximize trước Ctrl+E (dialog position fix)...");
            win_focus::maximize_capcut();
            thread::sleep(Duration::from_millis(2000));
        }
    }

    emit_log(app, "[step3] Focus CapCut trước Ctrl+E...");
    focus_capcut_log(app);
    thread::sleep(Duration::from_millis(2000));

    // Safety net: gửi Esc thêm một lần nữa phòng trường hợp Missing Media dialog
    // xuất hiện muộn (sau khi maximize) mà step1 chưa kịp dismiss.
    // Esc ở đây an toàn: nếu export dialog đã mở → Esc đóng nó (sẽ retry sau);
    // nếu không có gì → bị ignore.
    emit_log(app, "  [step3] Pre-Ctrl+E safety Esc...");
    let _ = enigo.key(Key::Escape, Direction::Click);
    thread::sleep(Duration::from_millis(300));

    // Click giữa editor canvas để set DOM focus.
    // SetForegroundWindow chỉ set OS focus, không control DOM focus bên trong Electron.
    emit_log(app, "  [step3] Click editor canvas để set DOM focus...");
    let _ = enigo.move_mouse(960, 540, Coordinate::Abs);
    thread::sleep(Duration::from_millis(200));
    let _ = enigo.button(Button::Left, Direction::Click);
    thread::sleep(Duration::from_millis(500));

    // Verify foreground là CapCut
    #[cfg(target_os = "windows")]
    {
        use super::super::window::win_focus;
        let fg = win_focus::get_foreground_title();
        if !fg.contains("CapCut") || fg.contains("Auto") {
            emit_log(app, "  [step3] ⚠ FG sai — re-focus CapCut...");
            focus_capcut_log(app);
            thread::sleep(Duration::from_millis(500));
            let _ = enigo.move_mouse(960, 540, Coordinate::Abs);
            thread::sleep(Duration::from_millis(200));
            let _ = enigo.button(Button::Left, Direction::Click);
            thread::sleep(Duration::from_millis(500));
        }
    }

    log_diagnostic(app, "Trước Ctrl+E");
    emit_log(app, "[step3] Gửi Ctrl+E → Export dialog...");
    let _ = enigo.key(Key::Control, Direction::Press);
    let _ = enigo.key(Key::Unicode('e'), Direction::Click);
    let _ = enigo.key(Key::Control, Direction::Release);
    thread::sleep(Duration::from_secs(2));

    // Step 4: Set export path — CHỈ project đầu tiên (idx == 0).
    // CapCut nhớ export path từ lần đầu, project 2+ không cần set lại.
    if idx == 0 && config.export_input_coords.0 > 0 && config.export_input_coords.1 > 0 {
        thread::sleep(Duration::from_millis(2000));
        log_diagnostic(app, "Trước click export path");

        emit_log(
            app,
            format!(
                "[step4] Click export path button tại ({}, {})",
                config.export_input_coords.0, config.export_input_coords.1
            ),
        );
        let _ = enigo.move_mouse(
            config.export_input_coords.0,
            config.export_input_coords.1,
            Coordinate::Abs,
        );
        thread::sleep(Duration::from_millis(2000));
        let _ = enigo.button(Button::Left, Direction::Click);

        // Poll cho đến khi folder picker xuất hiện (hỗ trợ EN + VI)
        let picker_titles: &[&str] =
            &["Select exporting path", "Chọn đường dẫn khi xuất"];
        emit_log(app, format!("[step4] Chờ folder picker {:?}...", picker_titles));

        let picker_found;
        #[cfg(target_os = "windows")]
        {
            use super::super::window::win_focus;
            picker_found = win_focus::wait_for_window_any_title(picker_titles, 6000).is_some();
        }
        #[cfg(not(target_os = "windows"))]
        {
            picker_found = false;
        }

        if !picker_found {
            emit_log(app, "[step4] ⚠ Folder picker không xuất hiện — bỏ qua thay đổi path");
        } else {
            emit_log(app, "[step4] ✓ Folder picker mở — dùng Alt+D focus address bar...");
            thread::sleep(Duration::from_millis(2000));

            // Alt+D: focus address bar (thanh path trên cùng).
            // Enter trong address bar: navigate VÀ confirm picker.
            let _ = enigo.key(Key::Alt, Direction::Press);
            let _ = enigo.key(Key::Unicode('d'), Direction::Click);
            let _ = enigo.key(Key::Alt, Direction::Release);
            thread::sleep(Duration::from_millis(2000));

            emit_log(app, format!("[step4] Gõ path: {}", config.export_path));
            // Dùng clipboard + Ctrl+V thay vì enigo.text() (fail với Unicode/Vietnamese path)
            #[cfg(target_os = "windows")]
            {
                use super::super::window::win_focus;
                if win_focus::set_clipboard_text(&config.export_path) {
                    let _ = enigo.key(Key::Control, Direction::Press);
                    let _ = enigo.key(Key::Unicode('v'), Direction::Click);
                    let _ = enigo.key(Key::Control, Direction::Release);
                    emit_log(app, "  [step4] ✓ Paste path qua clipboard");
                } else {
                    emit_log(app, "  [step4] ⚠ Clipboard fail — fallback enigo.text()");
                    let _ = enigo.text(&config.export_path);
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = enigo.text(&config.export_path);
            }
            thread::sleep(Duration::from_millis(2000));

            // Enter 1: navigate address bar đến folder (picker chưa đóng)
            emit_log(app, "[step4] Enter 1 — confirm path...");
            let _ = enigo.key(Key::Return, Direction::Click);
            thread::sleep(Duration::from_millis(3000));

            // Click giữa title bar dialog → focus dialog level → Enter trigger "Select Folder"
            emit_log(app, "  [step4] Click title bar dialog để focus...");
            #[cfg(target_os = "windows")]
            {
                use super::super::window::win_focus;
                if let Some((l, t, r, _b)) =
                    win_focus::get_window_rect_by_any_title(picker_titles)
                {
                    let title_x = l + (r - l) / 2;
                    let title_y = t + 15;
                    emit_log(
                        app,
                        format!("  [step4] Title bar click ({}, {})", title_x, title_y),
                    );
                    let _ = enigo.move_mouse(title_x, title_y, Coordinate::Abs);
                    thread::sleep(Duration::from_millis(300));
                    let _ = enigo.button(Button::Left, Direction::Click);
                }
            }
            thread::sleep(Duration::from_millis(1000));

            // Enter 2: trigger default button "Select Folder" → đóng picker
            emit_log(app, "[step4] Enter 2 — confirm Select Folder...");
            let _ = enigo.key(Key::Return, Direction::Click);

            // Poll picker đóng
            #[cfg(target_os = "windows")]
            {
                use super::super::window::win_focus;
                let closed = win_focus::wait_for_window_any_close(picker_titles, 4000);
                if closed {
                    emit_log(app, "[step4] ✓ Picker đã đóng");
                } else {
                    emit_log(app, "[step4] ⚠ Picker vẫn mở — retry title bar + Enter...");
                    if let Some((l, t, r, _b)) =
                        win_focus::get_window_rect_by_any_title(picker_titles)
                    {
                        let _ = enigo
                            .move_mouse(l + (r - l) / 2, t + 15, Coordinate::Abs);
                        thread::sleep(Duration::from_millis(300));
                        let _ = enigo.button(Button::Left, Direction::Click);
                        thread::sleep(Duration::from_millis(500));
                        let _ = enigo.key(Key::Return, Direction::Click);
                    }
                    let closed2 = win_focus::wait_for_window_any_close(picker_titles, 4000);
                    if !closed2 {
                        emit_log(app, "[step4] ⚠ Picker không đóng — Escape để hủy");
                        let _ = enigo.key(Key::Escape, Direction::Click);
                        let _ = win_focus::wait_for_window_any_close(picker_titles, 3000);
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                thread::sleep(Duration::from_millis(1000));
            }

            emit_log(app, "[step4] ✓ Export path sequence done");
        }
    }

    // Step 5: Enter để bắt đầu export.
    // Với idx > 0: step4 bị skip → chờ export dialog load trước khi Enter.
    if idx > 0 {
        emit_log(app, "[step5] Chờ export dialog load (3s)...");
        thread::sleep(Duration::from_secs(3));
    }
    log_diagnostic(app, "Trước Enter export");
    emit_log(app, "[step5] Enter → bắt đầu export...");
    let _ = enigo.key(Key::Return, Direction::Click);

    StepResult::Continue
}
