use enigo::{Enigo, Settings};
use std::sync::atomic::Ordering;
use std::time::Instant;
use tauri::{AppHandle, Manager};

use super::helpers::{
    emit_project_status, get_files_in_dir, restore_tool_window, validate_export_path,
};
use super::logger::{emit_log, init_log_file};
use super::steps::{self, StepResult};
use super::{AutoConfig, AutomationState};

/// Entry point của automation loop, chạy trong thread riêng.
pub fn run_automation_loop(app: &AppHandle, config: AutoConfig) {
    init_log_file(app);

    let state = app.state::<AutomationState>();

    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => {
            emit_log(app, format!("❌ Không thể khởi tạo automation: {:?}", e));
            return;
        }
    };

    let total = config.project_names.len();
    if total == 0 {
        emit_log(app, "⚠️ Không có project nào được chọn!");
        return;
    }

    let max_retries = config.max_retries;
    emit_log(app, format!("🚀 Bắt đầu Auto Render {} project...", total));
    emit_log(app, format!("📁 Export folder: {}", config.export_path));
    emit_log(app, format!("⏱ Timeout: {} phút/project", config.render_timeout_minutes));
    emit_log(app, format!("🔄 Max retries: {}/project", max_retries));

    // Pre-check export path
    if let Err(e) = validate_export_path(&config.export_path) {
        emit_log(app, format!("❌ Export folder lỗi: {}", e));
        return;
    }
    emit_log(app, "[pre] ✓ Export path OK");

    // Debug: list CapCut windows
    #[cfg(target_os = "windows")]
    {
        let wins = super::window::win_focus::list_capcut_windows();
        emit_log(app, format!("[debug] CapCut windows hiện tại: {:?}", wins));
    }

    // Step 0: Kill → Launch → Wait → Maximize Home
    match steps::setup::run(app, &config, &state, &mut enigo) {
        StepResult::StopAll | StepResult::SkipProject => return,
        StepResult::Continue => {}
    }

    // ─── Loop qua từng project ────────────────────────────────────────────────
    let mut success_count = 0u32;
    'projects: for (idx, project_name) in config.project_names.iter().enumerate() {
        if state.should_stop.load(Ordering::SeqCst) {
            emit_log(app, "⏹ Đã dừng bởi người dùng");
            break;
        }

        emit_log(app, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        emit_log(app, format!("[{}/{}] Project: {}", idx + 1, total, project_name));
        emit_project_status(app, project_name, "Running");

        let project_start = Instant::now();
        let mut render_done = false;
        let mut render_elapsed = 0u64;

        // ─── Retry loop ───────────────────────────────────────────────────────
        for attempt in 1..=(max_retries + 1) {
            if state.should_stop.load(Ordering::SeqCst) {
                restore_tool_window(app);
                break 'projects;
            }

            if attempt > 1 {
                steps::recovery::emit_retry_status(
                    app,
                    project_name,
                    attempt - 1,
                    max_retries,
                );
                if !steps::recovery::recover_capcut(app, &config, &state, &mut enigo) {
                    emit_log(app, "❌ Recovery thất bại — dừng automation");
                    restore_tool_window(app);
                    break 'projects;
                }
                if state.should_stop.load(Ordering::SeqCst) {
                    restore_tool_window(app);
                    break 'projects;
                }
            }

            // Snapshot export folder trước khi render
            let before_files = get_files_in_dir(&config.export_path);

            // Steps 1-2: Open project + Maximize Editor
            match steps::project::run(app, &config, &state, &mut enigo, idx, project_name) {
                StepResult::StopAll => {
                    restore_tool_window(app);
                    break 'projects;
                }
                StepResult::SkipProject => {
                    if attempt <= max_retries {
                        emit_log(
                            app,
                            format!(
                                "  ⚠ Open project thất bại (attempt {}/{}), sẽ retry...",
                                attempt,
                                max_retries + 1
                            ),
                        );
                        continue; // retry
                    }
                    break; // đã hết retry
                }
                StepResult::Continue => {}
            }

            // Steps 3-5: Ctrl+E → Set path → Enter
            match steps::export::run(app, &config, &state, &mut enigo, idx) {
                StepResult::StopAll => {
                    restore_tool_window(app);
                    break 'projects;
                }
                StepResult::SkipProject => {
                    if attempt <= max_retries {
                        emit_log(
                            app,
                            format!(
                                "  ⚠ Export dialog thất bại (attempt {}/{}), sẽ retry...",
                                attempt,
                                max_retries + 1
                            ),
                        );
                        continue;
                    }
                    break;
                }
                StepResult::Continue => {}
            }

            if state.should_stop.load(Ordering::SeqCst) {
                restore_tool_window(app);
                break 'projects;
            }

            // Step 6: Poll render done
            let (done, elapsed) = steps::render::run(app, &config, &state, &before_files);
            render_done = done;
            render_elapsed = elapsed;

            if state.should_stop.load(Ordering::SeqCst) {
                restore_tool_window(app);
                break 'projects;
            }

            if render_done {
                break; // thành công → thoát retry loop
            }

            // Render timeout
            if attempt <= max_retries {
                emit_log(
                    app,
                    format!(
                        "  ⚠ Render timeout sau {}s (attempt {}/{}), sẽ retry...",
                        render_elapsed,
                        attempt,
                        max_retries + 1
                    ),
                );
            }
        }
        // ─── End retry loop ───────────────────────────────────────────────────

        if render_done {
            emit_log(
                app,
                format!(
                    "✅ [{}/{}] Render xong: {} ({}s)",
                    idx + 1, total, project_name, render_elapsed
                ),
            );
            emit_project_status(app, project_name, "Done");
            success_count += 1;
        } else {
            emit_log(
                app,
                format!(
                    "⚠️ [{}/{}] Thất bại sau {} lần thử: {}",
                    idx + 1,
                    total,
                    max_retries + 1,
                    project_name
                ),
            );
            emit_project_status(app, project_name, "Error");
        }

        let project_elapsed = project_start.elapsed().as_secs();
        emit_log(
            app,
            format!(
                "[summary] '{}': {} trong {}s (render={}s)",
                project_name,
                if render_done { "DONE" } else { "FAIL" },
                project_elapsed,
                render_elapsed
            ),
        );

        // Step 7: Đóng project → về Home (bỏ qua project cuối cùng)
        if idx + 1 < total {
            match steps::cleanup::run(app, &config, &state, &mut enigo) {
                StepResult::StopAll => {
                    restore_tool_window(app);
                    return;
                }
                _ => {}
            }
        }
    }

    restore_tool_window(app);
    emit_log(app, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    emit_log(
        app,
        format!("🎉 Hoàn thành! Đã render {}/{} project thành công.", success_count, total),
    );

    if config.shutdown {
        emit_log(app, "🔌 Tắt máy sau 10 giây...");
        std::thread::sleep(std::time::Duration::from_secs(10));
        let _ = system_shutdown::shutdown();
    }
}
