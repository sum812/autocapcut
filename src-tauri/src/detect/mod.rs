/// detect/mod.rs — Tauri commands cho Auto Detect UI.
///
/// Hai command chính:
/// - `capture_ui_template`: Chụp 48×48 vùng quanh tọa độ đã chọn, lưu PNG làm template.
/// - `detect_ui_coords`:   Chụp cửa sổ CapCut, tìm template, trả về tọa độ screen.

pub mod capture;
pub mod matcher;

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

use crate::automation::window as win;
use matcher::TEMPLATE_SIZE;

/// Trả về thư mục lưu template: {app_data_dir}/templates/
fn templates_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Không lấy được app_data_dir: {}", e))?
        .join("templates");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Tạo thư mục templates thất bại: {}", e))?;
    Ok(dir)
}

/// Lưu RGBA bytes thành file PNG.
fn save_png(path: &PathBuf, rgba: &[u8], w: u32, h: u32) -> Result<(), String> {
    use image::{ImageBuffer, Rgba};
    let img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(w, h, rgba.to_vec())
        .ok_or_else(|| "Tạo ImageBuffer thất bại".to_string())?;
    img.save(path)
        .map_err(|e| format!("Lưu PNG thất bại: {}", e))
}

/// Đọc PNG và trả về (rgba_bytes, w, h).
fn load_png(path: &PathBuf) -> Result<(Vec<u8>, u32, u32), String> {
    use image::GenericImageView;
    let img = image::open(path).map_err(|e| format!("Mở PNG thất bại: {}", e))?;
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8().into_raw();
    Ok((rgba, w, h))
}

/// Chụp vùng 48×48 quanh (x, y), lưu làm template cho coord_key.
///
/// Gọi sau khi người dùng pick tọa độ thủ công — template được lưu để dùng cho auto-detect sau.
///
/// # Arguments
/// - `coord_key`: `"first_project_coords"` | `"export_box_coords"` | `"search_button_coords"`
/// - `x`, `y`: tọa độ screen mà người dùng đã chọn (tâm)
#[tauri::command]
pub async fn capture_ui_template(
    app: AppHandle,
    coord_key: String,
    x: i32,
    y: i32,
) -> Result<String, String> {
    let half = (TEMPLATE_SIZE / 2) as i32;
    let cx = x - half;
    let cy = y - half;

    let (rgba, w, h) = capture::capture_screen_region(cx, cy, TEMPLATE_SIZE as i32, TEMPLATE_SIZE as i32)
        .ok_or_else(|| "Chụp màn hình thất bại".to_string())?;

    let dir = templates_dir(&app)?;
    let path = dir.join(format!("{}.png", coord_key));
    save_png(&path, &rgba, w, h)?;

    Ok(format!("✓ Template '{}' đã lưu: {}×{}", coord_key, w, h))
}

/// Chụp cửa sổ CapCut, tìm template của coord_key, trả về [x, y] screen coords.
///
/// # Returns
/// - `Ok([cx, cy])`: tọa độ tâm của vị trí khớp trong screen coordinates
/// - `Err(msg)`: không tìm thấy CapCut, không có template, hoặc score quá cao
#[tauri::command]
pub async fn detect_ui_coords(
    app: AppHandle,
    coord_key: String,
) -> Result<[i32; 2], String> {
    // 1. Tìm CapCut window + lấy rect
    let (win_left, win_top, _win_right, _win_bottom) = win::get_capcut_window_rect()
        .ok_or_else(|| "Không tìm thấy cửa sổ CapCut".to_string())?;

    let hwnd = win::get_capcut_main_hwnd()
        .ok_or_else(|| "Không lấy được HWND của CapCut".to_string())?;

    // 2. Chụp cửa sổ CapCut bằng PrintWindow
    let (src_rgba, src_w, src_h) = capture::capture_window(hwnd)
        .ok_or_else(|| "Chụp cửa sổ CapCut thất bại".to_string())?;

    // 3. Đọc template
    let dir = templates_dir(&app)?;
    let path = dir.join(format!("{}.png", coord_key));
    if !path.exists() {
        return Err(format!(
            "Chưa có template cho '{}'. Hãy Calibrate thủ công trước.",
            coord_key
        ));
    }
    let (tmpl_rgba, tmpl_w, tmpl_h) = load_png(&path)?;

    // 4. Template matching
    let result = matcher::find_template(&src_rgba, src_w, src_h, &tmpl_rgba, tmpl_w, tmpl_h)
        .ok_or_else(|| "Template matching thất bại (ảnh quá nhỏ?)".to_string())?;

    if result.avg_ssd > matcher::SSD_THRESHOLD {
        return Err(format!(
            "Không tìm thấy '{}' — score={:.1} (threshold={:.1}). UI có thể đã thay đổi.",
            coord_key,
            result.avg_ssd,
            matcher::SSD_THRESHOLD
        ));
    }

    // 5. Convert từ window-relative → screen coords
    let screen_x = win_left + result.cx as i32;
    let screen_y = win_top + result.cy as i32;

    Ok([screen_x, screen_y])
}
