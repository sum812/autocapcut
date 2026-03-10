/// Input Validation — kiểm tra config trước khi bắt đầu automation.
///
/// Kiểm tra:
/// 1. project_folder: tồn tại và đọc được
/// 2. export_folder: tồn tại và ghi được
/// 3. first_project_coords: trong screen bounds (không phải [0,0])
/// 4. export_box_coords: trong screen bounds (không phải [0,0])
/// 5. CapCut đã cài (kiểm tra registry / common paths)
/// 6. project_names: ít nhất 1 project được chọn

use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub ok: bool,
    pub errors: Vec<ValidationError>,
}

#[tauri::command]
pub fn validate_config(
    project_folder: String,
    export_folder: String,
    first_project_coords: [i32; 2],
    export_box_coords: [i32; 2],
    project_names: Vec<String>,
) -> ValidationResult {
    let mut errors: Vec<ValidationError> = Vec::new();

    // 1. project_folder
    if project_folder.is_empty() {
        errors.push(ValidationError {
            field: "project_folder".to_string(),
            message: "Chưa chọn thư mục Projects".to_string(),
        });
    } else if !Path::new(&project_folder).exists() {
        errors.push(ValidationError {
            field: "project_folder".to_string(),
            message: format!("Thư mục không tồn tại: {}", project_folder),
        });
    }

    // 2. export_folder: tồn tại + ghi được
    if export_folder.is_empty() {
        errors.push(ValidationError {
            field: "export_folder".to_string(),
            message: "Chưa chọn thư mục Export".to_string(),
        });
    } else {
        let export_path = Path::new(&export_folder);
        if !export_path.exists() {
            // Thử tạo
            if let Err(e) = fs::create_dir_all(export_path) {
                errors.push(ValidationError {
                    field: "export_folder".to_string(),
                    message: format!("Không thể tạo thư mục export: {}", e),
                });
            }
        }
        if export_path.exists() {
            // Kiểm tra quyền ghi bằng cách tạo + xóa file test
            let test_file = export_path.join(".autocapcut_write_test");
            match fs::write(&test_file, b"test") {
                Ok(_) => {
                    let _ = fs::remove_file(&test_file);
                }
                Err(_) => {
                    errors.push(ValidationError {
                        field: "export_folder".to_string(),
                        message: "Thư mục export không có quyền ghi".to_string(),
                    });
                }
            }
        }
    }

    // 3. first_project_coords
    if first_project_coords == [0, 0] {
        errors.push(ValidationError {
            field: "first_project_coords".to_string(),
            message: "Chưa calibrate tọa độ Project (Tọa độ XY)".to_string(),
        });
    } else if !is_coord_in_screen(first_project_coords) {
        errors.push(ValidationError {
            field: "first_project_coords".to_string(),
            message: format!(
                "Tọa độ Project ({},{}) nằm ngoài màn hình",
                first_project_coords[0], first_project_coords[1]
            ),
        });
    }

    // 4. export_box_coords
    if export_box_coords == [0, 0] {
        errors.push(ValidationError {
            field: "export_box_coords".to_string(),
            message: "Chưa calibrate tọa độ Export Path".to_string(),
        });
    } else if !is_coord_in_screen(export_box_coords) {
        errors.push(ValidationError {
            field: "export_box_coords".to_string(),
            message: format!(
                "Tọa độ Export ({},{}) nằm ngoài màn hình",
                export_box_coords[0], export_box_coords[1]
            ),
        });
    }

    // 5. CapCut installed
    if !is_capcut_installed() {
        errors.push(ValidationError {
            field: "capcut".to_string(),
            message: "Không tìm thấy CapCut Desktop. Vui lòng cài đặt CapCut trước.".to_string(),
        });
    }

    // 6. project_names
    if project_names.is_empty() {
        errors.push(ValidationError {
            field: "project_names".to_string(),
            message: "Chưa chọn project nào để render".to_string(),
        });
    }

    ValidationResult {
        ok: errors.is_empty(),
        errors,
    }
}

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

/// Kiểm tra tọa độ trong bounds của virtual screen (multi-monitor safe).
fn is_coord_in_screen(coord: [i32; 2]) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::mem;
        extern "system" {
            fn GetSystemMetrics(nIndex: i32) -> i32;
        }
        // SM_XVIRTUALSCREEN=76, SM_YVIRTUALSCREEN=77, SM_CXVIRTUALSCREEN=78, SM_CYVIRTUALSCREEN=79
        let vx = unsafe { GetSystemMetrics(76) };
        let vy = unsafe { GetSystemMetrics(77) };
        let vw = unsafe { GetSystemMetrics(78) };
        let vh = unsafe { GetSystemMetrics(79) };
        let _ = mem::size_of::<i32>(); // suppress unused import
        coord[0] >= vx
            && coord[0] < vx + vw
            && coord[1] >= vy
            && coord[1] < vy + vh
    }
    #[cfg(not(target_os = "windows"))]
    {
        coord[0] > 0 && coord[1] > 0
    }
}

/// Kiểm tra CapCut đã được cài đặt.
fn is_capcut_installed() -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::path::PathBuf;

        // Common install paths
        let candidates: &[&str] = &[
            r"C:\Program Files\CapCut\CapCut.exe",
            r"C:\Program Files (x86)\CapCut\CapCut.exe",
        ];
        for p in candidates {
            if Path::new(p).exists() {
                return true;
            }
        }

        // Kiểm tra qua %LOCALAPPDATA%\CapCut
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let capcut = PathBuf::from(&local_app_data).join("CapCut").join("Apps");
            if capcut.exists() {
                return true;
            }
        }

        // Kiểm tra qua registry (Uninstall key)
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::HKEY_LOCAL_MACHINE;
            use winreg::RegKey;
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            let uninstall_paths = [
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
                r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
            ];
            for path in &uninstall_paths {
                if let Ok(key) = hklm.open_subkey(path) {
                    for subkey_name in key.enum_keys().flatten() {
                        if subkey_name.to_lowercase().contains("capcut") {
                            return true;
                        }
                        if let Ok(sub) = key.open_subkey(&subkey_name) {
                            let display_name: String =
                                sub.get_value("DisplayName").unwrap_or_default();
                            if display_name.to_lowercase().contains("capcut") {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Non-Windows: bỏ qua check
        true
    }
}
