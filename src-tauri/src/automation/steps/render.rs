/// Step 6: Poll export folder cho đến khi video file mới xuất hiện
use std::collections::HashSet;
use tauri::AppHandle;

use super::super::helpers::wait_for_new_video_file;
use super::super::logger::emit_log;
use super::super::{AutoConfig, AutomationState};

/// Returns (render_done, elapsed_secs)
pub fn run(
    app: &AppHandle,
    config: &AutoConfig,
    state: &AutomationState,
    before_files: &HashSet<String>,
) -> (bool, u64) {
    use std::time::Instant;

    emit_log(
        app,
        format!(
            "[step6] Chờ render xong (poll export folder, timeout {}p)...",
            config.render_timeout_minutes
        ),
    );

    let timeout_secs = config.render_timeout_minutes * 60;
    let render_start = Instant::now();
    let render_done =
        wait_for_new_video_file(app, &config.export_path, before_files, timeout_secs, state);
    let elapsed = render_start.elapsed().as_secs();

    (render_done, elapsed)
}
