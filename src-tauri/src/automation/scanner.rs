use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub status: String,
}

/// Quét thư mục path, trả về danh sách thư mục con dưới dạng ProjectInfo.
/// Bỏ qua thư mục ẩn (bắt đầu bằng '.').
/// Sort theo last modified (newest first) — khớp thứ tự hiển thị của CapCut Home.
#[tauri::command]
pub async fn scan_projects(path: String) -> Result<Vec<ProjectInfo>, String> {
    let mut projects = Vec::new();
    let base = std::path::Path::new(&path);

    if let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    if let Ok(name) = entry.file_name().into_string() {
                        if name.starts_with('.') {
                            continue;
                        }
                        projects.push(ProjectInfo {
                            id: name.clone(),
                            name,
                            status: "Pending".to_string(),
                        });
                    }
                }
            }
        }
    }

    // Sort by last modified (newest first) — match CapCut Home UI order
    projects.sort_by(|a, b| {
        let mtime_a = std::fs::metadata(base.join(&a.name))
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        let mtime_b = std::fs::metadata(base.join(&b.name))
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        mtime_b.cmp(&mtime_a)
    });

    Ok(projects)
}
