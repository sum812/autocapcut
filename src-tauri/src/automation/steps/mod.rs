pub mod cleanup;
pub mod export;
pub mod project;
pub mod recovery;
pub mod render;
pub mod setup;

use enigo::{Direction, Enigo, Key, Keyboard};
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;

use super::logger::emit_log;
use super::window::focus_capcut_log;

/// Kết quả trả về của từng step.
pub enum StepResult {
    /// Step thành công, tiếp tục.
    Continue,
    /// Bỏ qua project hiện tại (ghi Error), sang project tiếp theo.
    SkipProject,
    /// Dừng toàn bộ automation (user stop hoặc lỗi nghiêm trọng).
    StopAll,
}

// ─── Shared step utilities ────────────────────────────────────────────────────

/// Đóng popup trên CapCut Home screen.
/// Xử lý extra OS popup windows (WM_CLOSE + Ctrl+W). Sau đó focus main CapCut.
/// KHÔNG gửi Ctrl+W lên main window — Ctrl+W đóng CapCut Home khi không có in-app panel.
pub fn close_popups_home(app: &AppHandle, enigo: &mut Enigo) {
    #[cfg(target_os = "windows")]
    {
        use super::window::win_focus;
        let closed = win_focus::close_extra_capcut_windows();
        if closed > 0 {
            emit_log(app, format!("  [popup] WM_CLOSE {} extra OS window", closed));
            thread::sleep(Duration::from_millis(2000));
        }
        if let Some(hwnd) = win_focus::get_capcut_extra_window() {
            emit_log(app, "  [popup] Focus extra window + Ctrl+W...");
            win_focus::focus_hwnd(hwnd);
            thread::sleep(Duration::from_millis(2000));
            let _ = enigo.key(Key::Control, Direction::Press);
            let _ = enigo.key(Key::Unicode('w'), Direction::Click);
            let _ = enigo.key(Key::Control, Direction::Release);
            thread::sleep(Duration::from_millis(2000));
        }
    }
    if !focus_capcut_log(app) {
        emit_log(app, "  [popup] ⚠ CapCut window không tìm thấy — bỏ qua");
        return;
    }
    thread::sleep(Duration::from_millis(2000));
}

/// Đóng popup trên CapCut Editor screen (sau khi project đã load).
/// KHÔNG gửi Ctrl+W — tránh đóng nhầm project đang edit.
/// Chỉ focus main window để keyboard shortcut tiếp theo hoạt động đúng.
pub fn close_popups_editor(app: &AppHandle) {
    focus_capcut_log(app);
    thread::sleep(Duration::from_millis(2000));
}

/// Maximize CapCut qua ShowWindow API. Fallback sang Win+Up nếu cần.
/// Verify bằng kích thước thực tế so với screen dimensions.
/// Returns: true = maximize thành công, false = window không tồn tại.
pub fn maximize_with_verify(app: &AppHandle, enigo: &mut Enigo, label: &str) -> bool {
    if !focus_capcut_log(app) {
        emit_log(app, format!("  [max] ✗ {} — CapCut không tìm thấy", label));
        return false;
    }
    thread::sleep(Duration::from_millis(2000));

    #[cfg(target_os = "windows")]
    {
        use super::window::win_focus;

        // Đợi CapCut thoát "Not Responding" trước khi gửi command.
        if win_focus::is_capcut_not_responding() {
            emit_log(app, "  [max] CapCut Not Responding — chờ project load...");
            let nr_start = Instant::now();
            loop {
                thread::sleep(Duration::from_millis(2000));
                if !win_focus::is_capcut_not_responding() {
                    emit_log(
                        app,
                        format!(
                            "  [max] CapCut responsive trở lại sau {}s",
                            nr_start.elapsed().as_secs()
                        ),
                    );
                    break;
                }
                if nr_start.elapsed().as_secs() >= 60 {
                    emit_log(
                        app,
                        format!("  [max] ✗ {} — CapCut vẫn Not Responding sau 60s", label),
                    );
                    return false;
                }
            }
            if !focus_capcut_log(app) {
                emit_log(app, format!("  [max] ✗ {} — mất CapCut sau Not Responding", label));
                return false;
            }
            thread::sleep(Duration::from_millis(2000));
        }

        let screen = win_focus::get_screen_size();
        if let Some((sw, sh)) = screen {
            emit_log(app, format!("  [max] Screen: {}×{}px", sw, sh));
        }

        let is_maximized = || {
            if let (Some((sw, sh)), Some((l, t, r, b))) =
                (win_focus::get_screen_size(), win_focus::get_capcut_rect())
            {
                let cw = r - l;
                let ch = b - t;
                cw >= sw - 10 && ch >= sh - 120
            } else {
                false
            }
        };

        let rect_str = || {
            win_focus::get_capcut_rect()
                .map(|(l, t, r, b)| format!("{}×{}px", r - l, b - t))
                .unwrap_or_else(|| "N/A".into())
        };

        if is_maximized() {
            emit_log(app, format!("  [max] ✓ {} đã maximize — {}", label, rect_str()));
            return true;
        }

        emit_log(app, format!("  [max] SW_MAXIMIZE {} (trước: {})...", label, rect_str()));
        if !win_focus::maximize_capcut() {
            emit_log(app, format!("  [max] ✗ {} — window không tồn tại", label));
            return false;
        }
        thread::sleep(Duration::from_millis(2000));

        if is_maximized() {
            emit_log(app, format!("  [max] ✓ {} OK (API) — {}", label, rect_str()));
            return true;
        }

        // Fallback: Win+Up
        emit_log(
            app,
            format!("  [max] Thử Win+Up cho {}... (hiện tại: {})", label, rect_str()),
        );
        let _ = enigo.key(Key::Meta, Direction::Press);
        let _ = enigo.key(Key::UpArrow, Direction::Click);
        let _ = enigo.key(Key::Meta, Direction::Release);
        thread::sleep(Duration::from_millis(2000));

        if is_maximized() {
            emit_log(app, format!("  [max] ✓ {} OK (Win+Up) — {}", label, rect_str()));
            return true;
        }

        emit_log(app, format!("  [max] ✗ {} vẫn chưa maximize — {}", label, rect_str()));
        return false;
    }

    #[cfg(not(target_os = "windows"))]
    {
        emit_log(app, format!("  [max] {} (non-Windows)", label));
        true
    }
}
