/// F17: Batch Pre-Processing — đọc/ghi draft_content.json để sync media
///
/// CapCut lưu project ở dạng JSON với structure:
/// {
///   "tracks": [
///     { "type": "video", "segments": [ { "source_timerange": {...}, "target_timerange": {...}, ... } ] },
///     { "type": "audio", "segments": [...] },
///   ],
///   "materials": { "videos": [...], "audios": [...], ... }
/// }
///
/// Thời lượng tính bằng microseconds (1s = 1_000_000).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

// ──────────────────────────────────────────────
// Types
// ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncOptions {
    pub sync_video_audio: bool,
    pub sync_image_duration: bool,
    pub sync_subtitles: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct SyncResult {
    pub project: String,
    pub success: bool,
    pub message: String,
    pub changes: Vec<String>,
}

// ──────────────────────────────────────────────
// Tauri command: process_batch
// ──────────────────────────────────────────────

/// Chạy pre-processing cho danh sách project.
/// Emit event "sync_log" { project, success, message, changes } cho từng project.
#[tauri::command]
pub async fn process_batch(
    app: AppHandle,
    project_names: Vec<String>,
    project_folder: String,
    options: SyncOptions,
) -> Result<Vec<SyncResult>, String> {
    let mut results = Vec::new();

    for name in &project_names {
        let project_dir = PathBuf::from(&project_folder).join(name);
        let result = process_one(&project_dir, name, &options);

        // Emit log event cho frontend
        let _ = app.emit("sync_log", &result);
        results.push(result);
    }

    Ok(results)
}

// ──────────────────────────────────────────────
// Core logic
// ──────────────────────────────────────────────

fn process_one(project_dir: &Path, name: &str, opts: &SyncOptions) -> SyncResult {
    let draft_path = project_dir.join("draft_content.json");

    if !draft_path.exists() {
        return SyncResult {
            project: name.to_string(),
            success: false,
            message: "Không tìm thấy draft_content.json".to_string(),
            changes: vec![],
        };
    }

    let json_str = match fs::read_to_string(&draft_path) {
        Ok(s) => s,
        Err(e) => {
            return SyncResult {
                project: name.to_string(),
                success: false,
                message: format!("Lỗi đọc file: {}", e),
                changes: vec![],
            }
        }
    };

    let mut draft: Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            return SyncResult {
                project: name.to_string(),
                success: false,
                message: format!("Lỗi parse JSON: {}", e),
                changes: vec![],
            }
        }
    };

    let mut changes: Vec<String> = Vec::new();

    // Backup trước khi modify
    let backup_path = draft_path.with_extension("json.bak");
    let _ = fs::copy(&draft_path, &backup_path);

    if opts.sync_video_audio {
        match sync_video_audio(&mut draft) {
            Ok(msgs) => changes.extend(msgs),
            Err(e) => {
                return SyncResult {
                    project: name.to_string(),
                    success: false,
                    message: format!("Sync Video/Audio lỗi: {}", e),
                    changes,
                }
            }
        }
    }

    if opts.sync_image_duration {
        match sync_image_duration(&mut draft) {
            Ok(msgs) => changes.extend(msgs),
            Err(e) => {
                return SyncResult {
                    project: name.to_string(),
                    success: false,
                    message: format!("Sync Image Duration lỗi: {}", e),
                    changes,
                }
            }
        }
    }

    if opts.sync_subtitles {
        match sync_subtitles(&mut draft) {
            Ok(msgs) => changes.extend(msgs),
            Err(e) => {
                return SyncResult {
                    project: name.to_string(),
                    success: false,
                    message: format!("Sync Subtitles lỗi: {}", e),
                    changes,
                }
            }
        }
    }

    if changes.is_empty() {
        return SyncResult {
            project: name.to_string(),
            success: true,
            message: "Không có thay đổi".to_string(),
            changes,
        };
    }

    // Ghi lại file
    match serde_json::to_string(&draft) {
        Ok(out) => {
            if let Err(e) = fs::write(&draft_path, out) {
                return SyncResult {
                    project: name.to_string(),
                    success: false,
                    message: format!("Lỗi ghi file: {}", e),
                    changes,
                };
            }
        }
        Err(e) => {
            return SyncResult {
                project: name.to_string(),
                success: false,
                message: format!("Lỗi serialize JSON: {}", e),
                changes,
            };
        }
    }

    SyncResult {
        project: name.to_string(),
        success: true,
        message: format!("{} thay đổi đã áp dụng", changes.len()),
        changes,
    }
}

// ──────────────────────────────────────────────
// Sync Video/Audio: match video segments với audio 1:1
// ──────────────────────────────────────────────

fn sync_video_audio(draft: &mut Value) -> Result<Vec<String>, String> {
    let tracks = draft["tracks"].as_array_mut().ok_or("Không tìm thấy tracks")?;

    // Tìm video track và audio track
    let mut video_durations: Vec<i64> = Vec::new();
    let mut audio_durations: Vec<i64> = Vec::new();

    for track in tracks.iter() {
        let t = track["type"].as_str().unwrap_or("");
        if t == "video" {
            for seg in track["segments"].as_array().unwrap_or(&vec![]) {
                let dur = get_target_duration(seg);
                video_durations.push(dur);
            }
        } else if t == "audio" {
            for seg in track["segments"].as_array().unwrap_or(&vec![]) {
                let dur = get_target_duration(seg);
                audio_durations.push(dur);
            }
        }
    }

    if audio_durations.is_empty() {
        return Ok(vec!["Sync V/A: Không có audio track".to_string()]);
    }

    let min_count = video_durations.len().min(audio_durations.len());
    let mut changes = Vec::new();

    for track in tracks.iter_mut() {
        let t = track["type"].as_str().unwrap_or("").to_string();
        if t == "video" {
            let segs = match track["segments"].as_array_mut() {
                Some(s) => s,
                None => continue,
            };
            for i in 0..min_count.min(segs.len()) {
                let audio_dur = audio_durations[i];
                let current_dur = get_target_duration(&segs[i]);
                if (current_dur - audio_dur).abs() > 1000 {
                    // diff > 1ms
                    set_target_duration(&mut segs[i], audio_dur);
                    changes.push(format!(
                        "Sync V/A: segment {} {}μs → {}μs",
                        i, current_dur, audio_dur
                    ));
                }
            }
        }
    }

    if changes.is_empty() {
        changes.push("Sync V/A: Đã đồng bộ (không cần thay đổi)".to_string());
    }
    Ok(changes)
}

// ──────────────────────────────────────────────
// Sync Image Duration: kéo dài ảnh khớp tổng thời lượng audio
// ──────────────────────────────────────────────

fn sync_image_duration(draft: &mut Value) -> Result<Vec<String>, String> {
    let tracks = draft["tracks"].as_array_mut().ok_or("Không tìm thấy tracks")?;

    // Tính tổng thời lượng audio
    let mut total_audio_dur: i64 = 0;
    for track in tracks.iter() {
        if track["type"].as_str() == Some("audio") {
            for seg in track["segments"].as_array().unwrap_or(&vec![]) {
                total_audio_dur += get_target_duration(seg);
            }
        }
    }

    if total_audio_dur == 0 {
        return Ok(vec!["Sync Img Duration: Không có audio".to_string()]);
    }

    // Tính tổng thời lượng image (video segments có is_image=true hoặc type image)
    let mut total_image_dur: i64 = 0;
    let mut image_count = 0usize;

    for track in tracks.iter() {
        if track["type"].as_str() == Some("video") {
            for seg in track["segments"].as_array().unwrap_or(&vec![]) {
                if is_image_segment(seg) {
                    total_image_dur += get_target_duration(seg);
                    image_count += 1;
                }
            }
        }
    }

    if image_count == 0 {
        return Ok(vec!["Sync Img Duration: Không có ảnh trong timeline".to_string()]);
    }

    if (total_image_dur - total_audio_dur).abs() <= 1000 {
        return Ok(vec!["Sync Img Duration: Đã khớp".to_string()]);
    }

    // Phân phối đều thời lượng audio cho các ảnh
    let per_image_dur = total_audio_dur / image_count as i64;
    let remainder = total_audio_dur % image_count as i64;
    let mut changes = Vec::new();
    let mut idx = 0usize;

    for track in tracks.iter_mut() {
        if track["type"].as_str() == Some("video") {
            let segs = match track["segments"].as_array_mut() {
                Some(s) => s,
                None => continue,
            };
            for seg in segs.iter_mut() {
                if is_image_segment(seg) {
                    let new_dur = per_image_dur + if idx == 0 { remainder } else { 0 };
                    let old_dur = get_target_duration(seg);
                    if (old_dur - new_dur).abs() > 1000 {
                        set_target_duration(seg, new_dur);
                        changes.push(format!(
                            "Sync Img #{}: {}μs → {}μs",
                            idx, old_dur, new_dur
                        ));
                    }
                    idx += 1;
                }
            }
        }
    }

    if changes.is_empty() {
        changes.push("Sync Img Duration: Không cần thay đổi".to_string());
    }
    Ok(changes)
}

// ──────────────────────────────────────────────
// Sync Subtitles: align video/image segments theo subtitle timing
// ──────────────────────────────────────────────

fn sync_subtitles(draft: &mut Value) -> Result<Vec<String>, String> {
    let tracks = draft["tracks"].as_array_mut().ok_or("Không tìm thấy tracks")?;

    // Thu thập subtitle segments (type = "text" hoặc "subtitle")
    let mut sub_durations: Vec<(i64, i64)> = Vec::new(); // (start, duration) microseconds

    for track in tracks.iter() {
        let t = track["type"].as_str().unwrap_or("");
        if t == "text" || t == "subtitle" {
            for seg in track["segments"].as_array().unwrap_or(&vec![]) {
                let start = seg["target_timerange"]["start"].as_i64().unwrap_or(0);
                let dur = seg["target_timerange"]["duration"].as_i64().unwrap_or(0);
                if dur > 0 {
                    sub_durations.push((start, dur));
                }
            }
        }
    }

    if sub_durations.is_empty() {
        return Ok(vec!["Sync Subtitles: Không có subtitle track".to_string()]);
    }

    // Sort by start time
    sub_durations.sort_by_key(|(s, _)| *s);

    // Align video/image segments theo subtitle timing
    let mut sub_idx = 0usize;
    let mut changes = Vec::new();

    for track in tracks.iter_mut() {
        if track["type"].as_str() == Some("video") {
            let segs = match track["segments"].as_array_mut() {
                Some(s) => s,
                None => continue,
            };
            for seg in segs.iter_mut() {
                if sub_idx >= sub_durations.len() {
                    break;
                }
                let (sub_start, sub_dur) = sub_durations[sub_idx];
                let current_start = seg["target_timerange"]["start"].as_i64().unwrap_or(0);
                let current_dur = get_target_duration(seg);

                if (current_start - sub_start).abs() > 1000 || (current_dur - sub_dur).abs() > 1000 {
                    seg["target_timerange"]["start"] = Value::from(sub_start);
                    set_target_duration(seg, sub_dur);
                    changes.push(format!(
                        "Sync Sub #{}: start {}→{} dur {}→{}",
                        sub_idx, current_start, sub_start, current_dur, sub_dur
                    ));
                }
                sub_idx += 1;
            }
        }
    }

    if changes.is_empty() {
        changes.push("Sync Subtitles: Đã đồng bộ".to_string());
    }
    Ok(changes)
}

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

fn get_target_duration(seg: &Value) -> i64 {
    seg["target_timerange"]["duration"].as_i64().unwrap_or(0)
}

fn set_target_duration(seg: &mut Value, dur: i64) {
    if let Some(obj) = seg["target_timerange"].as_object_mut() {
        obj.insert("duration".to_string(), Value::from(dur));
    }
}

fn is_image_segment(seg: &Value) -> bool {
    // CapCut đánh dấu ảnh qua material_type hoặc extra_material_refs pointing to image
    seg["material_type"].as_str() == Some("photo")
        || seg["media_type"].as_str() == Some("photo")
        || seg["is_placeholder"].as_bool() == Some(false)
            && seg["source_timerange"]["duration"].as_i64().unwrap_or(1) == 0
}
