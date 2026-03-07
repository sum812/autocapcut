mod automation;
mod config;
mod updater;

use automation::AutomationState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AutomationState::new())
        .setup(|app| {
            // Khởi động global keyboard listener để bắt phím Space/Esc khi picking coords
            automation::init_listener(app.handle().clone());
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running AutoCapcut");
}
