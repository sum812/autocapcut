mod automation;
mod config;
mod detect;
mod license;
pub mod notification;
mod sync;
mod tray;
mod updater;
mod validation;

use automation::AutomationState;
use license::LicenseState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_tray::init())
        .manage(AutomationState::new())
        .manage(LicenseState::new())
        .setup(|app| {
            // Khởi động global keyboard listener để bắt phím Space/Esc khi picking coords
            automation::init_listener(app.handle().clone());
            // Verify license từ cache (offline, không block UI)
            license::init_license(app.handle());
            // Khởi tạo system tray
            tray::setup_tray(app.handle())?;
            Ok(())
        })
        // Close window → ẩn xuống tray thay vì thoát
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Config
            config::save_config,
            config::load_config,
            // Automation control
            automation::start_automation,
            automation::stop_automation,
            // Coordinate picking
            automation::start_picking_coords,
            automation::cancel_picking_coords,
            // Project scanner
            automation::scanner::scan_projects,
            // Logging
            automation::logger::get_log_file_path,
            // Updater
            updater::check_update,
            // Auto-detect UI coords
            detect::capture_ui_template,
            detect::detect_ui_coords,
            // License System
            license::get_license_status,
            license::activate_license,
            license::deactivate_license,
            license::refresh_license,
            license::get_machine_fingerprint,
            // Sync (F17)
            sync::process_batch,
            // Notification (F16)
            notification::send_test_notification,
            // Tray
            tray::set_tray_status,
            // Validation
            validation::validate_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AutoCapcut");
}
