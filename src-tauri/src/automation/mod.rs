pub mod helpers;
pub mod logger;
pub mod runner;
pub mod scanner;
pub mod steps;
pub mod window;

use enigo::{Enigo, Mouse, Settings};
use rdev::{listen, Event, EventType};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::thread;
use tauri::{AppHandle, Emitter, Manager};

// ─── Trạng thái automation ───────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum AutomationStatus {
    Idle,
    Running,
    Stopping,
    Stopped,
}

pub struct AutomationState {
    pub status: Mutex<AutomationStatus>,
    /// Flag dừng giữa chừng — tất cả sleep đều check flag này mỗi 100ms
    pub should_stop: AtomicBool,
    /// Đang ở chế độ pick tọa độ
    pub is_picking: AtomicBool,
}

impl AutomationState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(AutomationStatus::Idle),
            should_stop: AtomicBool::new(false),
            is_picking: AtomicBool::new(false),
        }
    }
}

// ─── Cấu hình truyền từ frontend khi bắt đầu automation ─────────────────────

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AutoConfig {
    pub project_path: String,
    pub export_path: String,
    pub project_names: Vec<String>,
    pub first_project_coords: (i32, i32),
    pub export_input_coords: (i32, i32),
    #[serde(default)]
    pub search_button_coords: (i32, i32),
    pub render_delay: u64,           // giây
    pub render_timeout_minutes: u64, // phút
    pub shutdown: bool,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,            // số lần retry mỗi project (0 = không retry)
    // F16 Notification
    #[serde(default = "default_notify_on_done")]
    pub notify_on_done: bool,
    #[serde(default)]
    pub notify_per_project: bool,
}

fn default_max_retries() -> u32 { 2 }
fn default_notify_on_done() -> bool { true }

// ─── Global keyboard listener (coordinate picking) ───────────────────────────

/// Khởi động rdev listener trong background thread để bắt Space/Esc khi picking coords.
pub fn init_listener(app: AppHandle) {
    thread::spawn(move || {
        if let Err(e) = listen(move |event| handle_input_event(event, &app)) {
            eprintln!("rdev listen error: {:?}", e);
        }
    });
}

fn handle_input_event(event: Event, app: &AppHandle) {
    let state = app.state::<AutomationState>();
    if !state.is_picking.load(Ordering::SeqCst) {
        return;
    }
    match event.event_type {
        EventType::KeyPress(rdev::Key::Space) | EventType::KeyPress(rdev::Key::Return) => {
            if let Ok(enigo) = Enigo::new(&Settings::default()) {
                if let Ok((x, y)) = enigo.location() {
                    let _ = app.emit("coordinate-picked", (x, y));
                }
            }
            state.is_picking.store(false, Ordering::SeqCst);
        }
        EventType::KeyPress(rdev::Key::Escape) => {
            state.is_picking.store(false, Ordering::SeqCst);
            let _ = app.emit("coordinate-picked-cancel", ());
        }
        _ => {}
    }
}

#[tauri::command]
pub fn start_picking_coords(state: tauri::State<AutomationState>) {
    state.is_picking.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn cancel_picking_coords(state: tauri::State<AutomationState>) {
    state.is_picking.store(false, Ordering::SeqCst);
}

// ─── Automation commands ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_automation(
    app: AppHandle,
    state: tauri::State<'_, AutomationState>,
    config: AutoConfig,
) -> Result<(), String> {
    {
        let mut status = state.status.lock().unwrap();
        if *status == AutomationStatus::Running {
            return Err("Automation đang chạy".to_string());
        }
        *status = AutomationStatus::Running;
        state.should_stop.store(false, Ordering::SeqCst);
    }

    let app_handle = app.clone();
    thread::spawn(move || {
        runner::run_automation_loop(&app_handle, config);

        let state = app_handle.state::<AutomationState>();
        *state.status.lock().unwrap() = AutomationStatus::Stopped;
        let _ = app_handle.emit("automation-status", "Stopped");
    });

    Ok(())
}

#[tauri::command]
pub fn stop_automation(state: tauri::State<AutomationState>) {
    state.should_stop.store(true, Ordering::SeqCst);
    *state.status.lock().unwrap() = AutomationStatus::Stopping;
}
