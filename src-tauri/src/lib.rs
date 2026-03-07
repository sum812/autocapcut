mod automation;
mod config;
mod detect;
mod license;
mod updater;

use automation::AutomationState;
use license::LicenseState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AutomationState::new())
        .manage(LicenseState::new())
        .setup(|app| {
            // Khởi động global keyboard listener để bắt phím Space/Esc khi picking coords
            automation::init_listener(app.handle().clone());
            // Verify license từ cache (offline, không block UI)
            license::init_license(app.handle());
            Ok(())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running AutoCapcut");
}
