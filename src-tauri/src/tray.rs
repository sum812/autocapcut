/// System Tray — minimize to tray, status icon, menu
///
/// States:
/// - Idle:    icon bình thường (32x32.png)
/// - Running: icon xanh lá  (dùng tooltip)
/// - Error:   icon đỏ       (dùng tooltip)
///
/// Menu items:
/// - Show      → unhide + focus window
/// - ---
/// - Stop      → emit stop signal (khi đang Running)
/// - ---
/// - Quit      → thoát hoàn toàn

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

pub const TRAY_ID: &str = "main";

/// App state lưu handle đến "Stop" menu item để enable/disable sau này.
pub struct TrayStopItem(pub MenuItem<tauri::Wry>);

/// Khởi tạo tray icon + menu khi app start.
pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Hiện cửa sổ", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let stop_item = MenuItem::with_id(app, "stop", "Dừng render", false, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Thoát AutoCapcut", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_item, &sep, &stop_item, &sep2, &quit_item])?;

    // Lưu stop_item vào app state để update_tray_status dùng
    app.manage(TrayStopItem(stop_item));

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("AutoCapcut — Idle")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_window(app),
            "stop" => {
                let _ = app.emit("tray-stop", ());
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Cập nhật tooltip + enable/disable menu item "Stop" dựa trên trạng thái.
/// Gọi từ automation runner khi status thay đổi.
pub fn update_tray_status(app: &AppHandle, status: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let tooltip = match status {
            "running" => "AutoCapcut — Đang render ▶",
            "stopping" => "AutoCapcut — Đang dừng ⏸",
            "error" => "AutoCapcut — Lỗi ❌",
            _ => "AutoCapcut — Idle",
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }

    // Enable "Stop" chỉ khi đang running
    if let Some(state) = app.try_state::<TrayStopItem>() {
        let enabled = status == "running";
        let _ = state.0.set_enabled(enabled);
    }
}

fn show_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

/// Tauri command: cập nhật trạng thái tray từ automation events.
#[tauri::command]
pub fn set_tray_status(app: AppHandle, status: String) {
    update_tray_status(&app, &status);
}
