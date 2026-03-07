use enigo::{Direction, Enigo, Key, Keyboard};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use super::logger::emit_log;
use super::AutomationState;

// ─── Windows API: focus, read title, list windows ────────────────────────────

#[cfg(target_os = "windows")]
pub mod win_focus {
    use std::ptr;

    #[link(name = "user32")]
    extern "system" {
        fn SetForegroundWindow(hWnd: *mut std::ffi::c_void) -> i32;
        fn GetForegroundWindow() -> *mut std::ffi::c_void;
        fn ShowWindow(hWnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
        fn EnumWindows(
            lpEnumFunc: extern "system" fn(*mut std::ffi::c_void, isize) -> i32,
            lParam: isize,
        ) -> i32;
        fn GetWindowTextW(hWnd: *mut std::ffi::c_void, lpString: *mut u16, nMaxCount: i32) -> i32;
        fn IsWindowVisible(hWnd: *mut std::ffi::c_void) -> i32;
        fn IsZoomed(hWnd: *mut std::ffi::c_void) -> i32;
        fn IsIconic(hWnd: *mut std::ffi::c_void) -> i32;
        fn PostMessageW(
            hWnd: *mut std::ffi::c_void,
            Msg: u32,
            wParam: usize,
            lParam: isize,
        ) -> i32;
        fn GetWindowRect(hWnd: *mut std::ffi::c_void, lpRect: *mut RECT) -> i32;
        fn GetSystemMetrics(nIndex: i32) -> i32;
        fn GetWindowThreadProcessId(hWnd: *mut std::ffi::c_void, lpdwProcessId: *mut u32) -> u32;
        fn FindWindowW(
            lpClassName: *const u16,
            lpWindowName: *const u16,
        ) -> *mut std::ffi::c_void;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(
            dwDesiredAccess: u32,
            bInheritHandle: i32,
            dwProcessId: u32,
        ) -> *mut std::ffi::c_void;
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
        fn QueryFullProcessImageNameW(
            hProcess: *mut std::ffi::c_void,
            dwFlags: u32,
            lpExeName: *mut u16,
            lpdwSize: *mut u32,
        ) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn OpenClipboard(hWndNewOwner: *mut std::ffi::c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(
            uFormat: u32,
            hMem: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalAlloc(uFlags: u32, dwBytes: usize) -> *mut std::ffi::c_void;
        fn GlobalLock(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GlobalUnlock(hMem: *mut std::ffi::c_void) -> i32;
    }

    const CF_UNICODETEXT: u32 = 13;
    const GMEM_MOVEABLE: u32 = 0x0002;

    /// Copy text vào clipboard (Windows API). Xử lý Unicode đúng.
    pub fn set_clipboard_text(text: &str) -> bool {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let byte_len = wide.len() * 2;

        unsafe {
            if OpenClipboard(ptr::null_mut()) == 0 {
                return false;
            }
            EmptyClipboard();

            let hmem = GlobalAlloc(GMEM_MOVEABLE, byte_len);
            if hmem.is_null() {
                CloseClipboard();
                return false;
            }
            let lock = GlobalLock(hmem);
            if lock.is_null() {
                CloseClipboard();
                return false;
            }
            std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, lock as *mut u8, byte_len);
            GlobalUnlock(hmem);

            let result = SetClipboardData(CF_UNICODETEXT, hmem);
            CloseClipboard();
            !result.is_null()
        }
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    /// Kiểm tra HWND có thuộc process CapCut.exe không.
    /// Dùng thay cho title-based filter để tránh bắt nhầm browser tab có "CapCut" trong title.
    unsafe fn is_capcut_exe_window(hwnd: *mut std::ffi::c_void) -> bool {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return false;
        }
        let h_process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h_process.is_null() {
            return false;
        }
        let mut buf = [0u16; 260];
        let mut size = 260u32;
        let ok = QueryFullProcessImageNameW(h_process, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(h_process);
        if ok != 0 && size > 0 {
            let path = String::from_utf16_lossy(&buf[..size as usize]);
            let lower = path.to_ascii_lowercase();
            lower.ends_with("capcut.exe") || lower.contains("\\capcut\\")
        } else {
            false
        }
    }

    const SW_RESTORE: i32 = 9;
    const WM_CLOSE: u32 = 0x0010;
    const SM_CXSCREEN: i32 = 0;
    const SM_CYSCREEN: i32 = 1;

    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    struct EnumData {
        hwnd: *mut std::ffi::c_void,
        title: String,
        area: i32,
    }

    extern "system" fn enum_callback(hwnd: *mut std::ffi::c_void, lparam: isize) -> i32 {
        unsafe {
            if IsWindowVisible(hwnd) == 0 {
                return 1;
            }
            let mut title = [0u16; 256];
            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 256);
            if len > 0 {
                let title_str = String::from_utf16_lossy(&title[..len as usize]);
                let is_tool_or_installer =
                    title_str.contains("Tool") || title_str.contains("Installer");
                if title_str.contains("CapCut")
                    && !is_tool_or_installer
                    && is_capcut_exe_window(hwnd)
                {
                    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    let area = if GetWindowRect(hwnd, &mut rect) != 0 {
                        ((rect.right - rect.left) * (rect.bottom - rect.top)).max(0)
                    } else {
                        0
                    };
                    let data = &mut *(lparam as *mut EnumData);
                    if area > data.area {
                        data.hwnd = hwnd;
                        data.title = title_str;
                        data.area = area;
                    }
                }
            }
        }
        1
    }

    /// Focus CapCut Desktop (bỏ qua Auto Tool và Installer).
    pub fn focus_capcut() -> Option<String> {
        unsafe {
            let mut data = EnumData { hwnd: ptr::null_mut(), title: String::new(), area: 0 };
            EnumWindows(enum_callback, &mut data as *mut _ as isize);
            if !data.hwnd.is_null() {
                if IsIconic(data.hwnd) != 0 {
                    ShowWindow(data.hwnd, SW_RESTORE);
                }
                SetForegroundWindow(data.hwnd);
                return Some(data.title);
            }
        }
        None
    }

    /// Đọc title cửa sổ CapCut Desktop mà không focus.
    pub fn get_capcut_title() -> Option<String> {
        unsafe {
            let mut data = EnumData { hwnd: ptr::null_mut(), title: String::new(), area: 0 };
            EnumWindows(enum_callback, &mut data as *mut _ as isize);
            if !data.hwnd.is_null() {
                return Some(data.title);
            }
        }
        None
    }

    /// Kiểm tra CapCut có đang "Not Responding" không.
    pub fn is_capcut_not_responding() -> bool {
        get_capcut_title()
            .map(|t| t.contains("Not Responding"))
            .unwrap_or(false)
    }

    struct EnumListData {
        titles: Vec<String>,
    }

    extern "system" fn enum_list_callback(hwnd: *mut std::ffi::c_void, lparam: isize) -> i32 {
        unsafe {
            if IsWindowVisible(hwnd) == 0 {
                return 1;
            }
            let mut title = [0u16; 256];
            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 256);
            if len > 0 {
                let title_str = String::from_utf16_lossy(&title[..len as usize]);
                if title_str.contains("CapCut") {
                    let data = &mut *(lparam as *mut EnumListData);
                    data.titles.push(title_str);
                }
            }
        }
        1
    }

    /// Debug: liệt kê tất cả window có "CapCut".
    pub fn list_capcut_windows() -> Vec<String> {
        unsafe {
            let mut data = EnumListData { titles: Vec::new() };
            EnumWindows(enum_list_callback, &mut data as *mut _ as isize);
            data.titles
        }
    }

    /// Kiểm tra cửa sổ CapCut Desktop có đang maximize không.
    pub fn is_capcut_maximized() -> bool {
        unsafe {
            let mut data = EnumData { hwnd: ptr::null_mut(), title: String::new(), area: 0 };
            EnumWindows(enum_callback, &mut data as *mut _ as isize);
            if !data.hwnd.is_null() {
                return IsZoomed(data.hwnd) != 0;
            }
        }
        false
    }

    /// Maximize CapCut Desktop qua ShowWindow API (SW_MAXIMIZE = 3).
    pub fn maximize_capcut() -> bool {
        const SW_MAXIMIZE: i32 = 3;
        unsafe {
            let mut data = EnumData { hwnd: ptr::null_mut(), title: String::new(), area: 0 };
            EnumWindows(enum_callback, &mut data as *mut _ as isize);
            if !data.hwnd.is_null() {
                ShowWindow(data.hwnd, SW_MAXIMIZE);
                return true;
            }
        }
        false
    }

    /// Lấy bounding rect của cửa sổ CapCut Desktop.
    pub fn get_capcut_rect() -> Option<(i32, i32, i32, i32)> {
        unsafe {
            let mut data = EnumData { hwnd: ptr::null_mut(), title: String::new(), area: 0 };
            EnumWindows(enum_callback, &mut data as *mut _ as isize);
            if !data.hwnd.is_null() {
                let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                if GetWindowRect(data.hwnd, &mut rect) != 0 {
                    return Some((rect.left, rect.top, rect.right, rect.bottom));
                }
            }
        }
        None
    }

    /// Trả về HWND của cửa sổ CapCut chính dưới dạng isize.
    pub fn get_capcut_hwnd() -> Option<isize> {
        unsafe {
            let mut data = EnumData { hwnd: ptr::null_mut(), title: String::new(), area: 0 };
            EnumWindows(enum_callback, &mut data as *mut _ as isize);
            if !data.hwnd.is_null() {
                return Some(data.hwnd as isize);
            }
        }
        None
    }

    /// Lấy kích thước màn hình chính.
    pub fn get_screen_size() -> Option<(i32, i32)> {
        unsafe {
            let w = GetSystemMetrics(SM_CXSCREEN);
            let h = GetSystemMetrics(SM_CYSCREEN);
            if w > 0 && h > 0 { Some((w, h)) } else { None }
        }
    }

    struct EnumAllData {
        windows: Vec<(*mut std::ffi::c_void, i32)>,
    }

    extern "system" fn enum_all_callback(hwnd: *mut std::ffi::c_void, lparam: isize) -> i32 {
        unsafe {
            if IsWindowVisible(hwnd) == 0 {
                return 1;
            }
            if !is_capcut_exe_window(hwnd) {
                return 1;
            }
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            let area = if GetWindowRect(hwnd, &mut rect) != 0 {
                ((rect.right - rect.left) * (rect.bottom - rect.top)).max(0)
            } else {
                0
            };
            if area > 0 {
                let data = &mut *(lparam as *mut EnumAllData);
                data.windows.push((hwnd, area));
            }
        }
        1
    }

    /// Đóng các cửa sổ CapCut phụ (popup, dialog), giữ lại cửa sổ chính (lớn nhất).
    pub fn close_extra_capcut_windows() -> usize {
        unsafe {
            let mut data = EnumAllData { windows: Vec::new() };
            EnumWindows(enum_all_callback, &mut data as *mut _ as isize);
            if data.windows.len() <= 1 {
                return 0;
            }
            let main_hwnd = data
                .windows
                .iter()
                .max_by_key(|&&(_, area)| area)
                .map(|&(hwnd, _)| hwnd);
            let mut closed = 0;
            for &(hwnd, _) in &data.windows {
                if Some(hwnd) != main_hwnd {
                    PostMessageW(hwnd, WM_CLOSE, 0, 0);
                    closed += 1;
                }
            }
            closed
        }
    }

    /// Tìm cửa sổ CapCut phụ đầu tiên (không phải main window).
    pub fn get_capcut_extra_window() -> Option<*mut std::ffi::c_void> {
        unsafe {
            let mut data = EnumAllData { windows: Vec::new() };
            EnumWindows(enum_all_callback, &mut data as *mut _ as isize);
            if data.windows.len() <= 1 {
                return None;
            }
            let main_hwnd = data
                .windows
                .iter()
                .max_by_key(|&&(_, area)| area)
                .map(|&(hwnd, _)| hwnd);
            data.windows
                .iter()
                .find(|&&(hwnd, _)| Some(hwnd) != main_hwnd)
                .map(|&(hwnd, _)| hwnd)
        }
    }

    /// Kiểm tra CapCut có đang là foreground window không.
    pub fn is_capcut_foreground() -> bool {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.is_null() {
                return false;
            }
            is_capcut_exe_window(fg)
        }
    }

    /// Focus một hwnd cụ thể.
    pub fn focus_hwnd(hwnd: *mut std::ffi::c_void) {
        unsafe {
            SetForegroundWindow(hwnd);
        }
    }

    /// Lấy title của foreground window hiện tại.
    pub fn get_foreground_title() -> String {
        unsafe {
            let fg = GetForegroundWindow();
            if fg.is_null() {
                return "(none)".to_string();
            }
            let mut title = [0u16; 256];
            let len = GetWindowTextW(fg, title.as_mut_ptr(), 256);
            if len > 0 {
                String::from_utf16_lossy(&title[..len as usize])
            } else {
                "(untitled)".to_string()
            }
        }
    }

    /// Lấy window rect bằng title.
    pub fn get_window_rect_by_title(title: &str) -> Option<(i32, i32, i32, i32)> {
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let hwnd = unsafe { FindWindowW(std::ptr::null(), title_wide.as_ptr()) };
        if hwnd.is_null() {
            return None;
        }
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
        if ok != 0 {
            Some((rect.left, rect.top, rect.right, rect.bottom))
        } else {
            None
        }
    }

    /// Lấy window rect bằng nhiều title (thử từng cái).
    pub fn get_window_rect_by_any_title(titles: &[&str]) -> Option<(i32, i32, i32, i32)> {
        for title in titles {
            if let Some(rect) = get_window_rect_by_title(title) {
                return Some(rect);
            }
        }
        None
    }

    /// Chụp trạng thái diagnostic CapCut.
    pub fn diagnostic_snapshot() -> String {
        let fg_title = get_foreground_title();
        let capcut_fg = is_capcut_foreground();
        let rect_str = match get_capcut_rect() {
            Some((l, t, r, b)) => format!("{}×{} @({},{})", r - l, b - t, l, t),
            None => "NOT_FOUND".to_string(),
        };
        let maximized = is_capcut_maximized();
        let win_count = list_capcut_windows().len();
        format!(
            "FG='{}' | CapCut={} max={} fg={} | wins={}",
            fg_title, rect_str, maximized, capcut_fg, win_count
        )
    }

    /// Poll cho đến khi BẤT KỲ cửa sổ nào trong titles xuất hiện hoặc timeout.
    pub fn wait_for_window_any_title(titles: &[&str], timeout_ms: u64) -> Option<usize> {
        let titles_wide: Vec<Vec<u16>> = titles
            .iter()
            .map(|t| t.encode_utf16().chain(std::iter::once(0)).collect())
            .collect();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        loop {
            for (i, tw) in titles_wide.iter().enumerate() {
                let hwnd = unsafe { FindWindowW(std::ptr::null(), tw.as_ptr()) };
                if !hwnd.is_null() {
                    return Some(i);
                }
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    /// Poll cho đến khi TẤT CẢ cửa sổ trong titles đều ĐÓNG hoặc timeout.
    pub fn wait_for_window_any_close(titles: &[&str], timeout_ms: u64) -> bool {
        let titles_wide: Vec<Vec<u16>> = titles
            .iter()
            .map(|t| t.encode_utf16().chain(std::iter::once(0)).collect())
            .collect();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        loop {
            let any_open = titles_wide.iter().any(|tw| {
                let hwnd = unsafe { FindWindowW(std::ptr::null(), tw.as_ptr()) };
                !hwnd.is_null()
            });
            if !any_open {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

// ─── Window HWND / Rect helpers (dùng bởi detect module) ─────────────────────

/// Trả về HWND của cửa sổ CapCut chính (lớn nhất) dưới dạng isize.
pub fn get_capcut_main_hwnd() -> Option<isize> {
    #[cfg(target_os = "windows")]
    {
        // Dùng get_capcut_rect logic nhưng trả về hwnd thay vì rect
        use win_focus::*;
        return get_capcut_hwnd();
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Trả về (left, top, right, bottom) của cửa sổ CapCut chính.
pub fn get_capcut_window_rect() -> Option<(i32, i32, i32, i32)> {
    #[cfg(target_os = "windows")]
    {
        return win_focus::get_capcut_rect();
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

// ─── Window count helper ──────────────────────────────────────────────────────

/// Đếm số CapCut windows hiện tại. Dùng để detect project open/close.
pub fn get_capcut_window_count() -> usize {
    #[cfg(target_os = "windows")]
    {
        win_focus::list_capcut_windows().len()
    }
    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

// ─── Focus helpers ────────────────────────────────────────────────────────────

/// Focus CapCut Desktop. Trả về true nếu tìm thấy window.
pub fn focus_capcut() -> bool {
    #[cfg(target_os = "windows")]
    {
        return win_focus::focus_capcut().is_some();
    }
    #[cfg(not(target_os = "windows"))]
    {
        return false;
    }
}

/// Focus CapCut Desktop và log kết quả chi tiết.
pub fn focus_capcut_log(app: &AppHandle) -> bool {
    #[cfg(target_os = "windows")]
    {
        match win_focus::focus_capcut() {
            Some(title) => {
                emit_log(app, format!("  [focus] ✓ → \"{}\"", title));
                return true;
            }
            None => {
                let all = win_focus::list_capcut_windows();
                if all.is_empty() {
                    emit_log(app, "  [focus] ✗ Không có cửa sổ CapCut nào");
                } else {
                    emit_log(
                        app,
                        format!(
                            "  [focus] ✗ Có {} window CapCut nhưng đều bị filter: {:?}",
                            all.len(),
                            all
                        ),
                    );
                }
                return false;
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        emit_log(app, "  [focus] SKIP (non-Windows)");
        return false;
    }
}

/// Chụp diagnostic snapshot và log tại các thời điểm then chốt.
pub fn log_diagnostic(app: &AppHandle, label: &str) {
    #[cfg(target_os = "windows")]
    {
        let snap = win_focus::diagnostic_snapshot();
        emit_log(app, format!("  [diag] {} | {}", label, snap));
    }
    #[cfg(not(target_os = "windows"))]
    {
        emit_log(app, format!("  [diag] {} | (non-Windows)", label));
    }
}

// ─── Kill / Launch ────────────────────────────────────────────────────────────

fn kill_capcut() {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "CapCut.exe"])
            .creation_flags(0x08000000)
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("pkill").args(["-f", "CapCut"]).output();
    }
    thread::sleep(Duration::from_secs(1));
}

/// Kill CapCut, poll đến khi window biến mất.
pub fn kill_capcut_verified(app: &AppHandle) {
    emit_log(app, "  [kill] Gửi taskkill /F /IM CapCut.exe...");
    kill_capcut();
    for i in 0..6 {
        thread::sleep(Duration::from_millis(500));
        if !focus_capcut() {
            emit_log(app, format!("  [kill] ✓ Window biến mất sau {}ms", (i + 1) * 500));
            return;
        }
    }
    emit_log(app, "  [kill] ⚠ Vẫn còn window sau 3s → kill lần 2...");
    kill_capcut();
    emit_log(app, "  [kill] Kill lần 2 xong");
}

/// Tìm và spawn CapCut.exe. Thử 3 phương pháp.
pub fn launch_capcut(project_path: &str) -> bool {
    let path = Path::new(project_path);
    let mut current = Some(path.as_ref() as &Path);
    while let Some(p) = current {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.eq_ignore_ascii_case("CapCut") {
            for subpath in &["Apps/CapCut.exe", "CapCut.exe", "Apps/capcut.exe"] {
                let exe = p.join(subpath);
                if exe.exists() && std::process::Command::new(&exe).spawn().is_ok() {
                    return true;
                }
            }
        }
        current = p.parent();
    }

    if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
        let capcut_root = Path::new(&local_app).join("CapCut");
        for subpath in &["Apps/CapCut.exe", "CapCut.exe"] {
            let exe = capcut_root.join(subpath);
            if exe.exists() && std::process::Command::new(&exe).spawn().is_ok() {
                return true;
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if std::process::Command::new("cmd")
            .args(["/C", "start", "", "CapCut"])
            .creation_flags(0x08000000)
            .spawn()
            .is_ok()
        {
            return true;
        }
    }

    false
}

// ─── Window wait ──────────────────────────────────────────────────────────────

/// Poll cho đến khi cửa sổ CapCut Desktop xuất hiện, log progress mỗi 2s.
pub fn wait_for_capcut_window(app: &AppHandle, timeout_secs: u64, state: &AutomationState) -> bool {
    use std::sync::atomic::Ordering;
    let start = Instant::now();
    let mut last_log_sec = 0u64;
    while start.elapsed().as_secs() < timeout_secs {
        if state.should_stop.load(Ordering::SeqCst) {
            emit_log(app, "  [wait] Bị dừng bởi user");
            return false;
        }
        #[cfg(target_os = "windows")]
        {
            if let Some(title) = win_focus::focus_capcut() {
                let elapsed = start.elapsed().as_secs();
                emit_log(
                    app,
                    format!("  [wait] ✓ Tìm thấy \"{}\" sau {}s", title, elapsed),
                );
                return true;
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if focus_capcut() {
                return true;
            }
        }
        let elapsed = start.elapsed().as_secs();
        if elapsed >= last_log_sec + 2 {
            emit_log(
                app,
                format!("  [wait] Đang chờ CapCut... {}s/{}s", elapsed, timeout_secs),
            );
            last_log_sec = elapsed;
        }
        thread::sleep(Duration::from_millis(500));
    }
    emit_log(app, format!("  [wait] ✗ Timeout sau {}s", timeout_secs));
    false
}

/// Đóng cửa sổ CapCut: focus → Alt+F4.
#[allow(dead_code)]
pub fn close_capcut(app: &AppHandle, enigo: &mut Enigo) {
    emit_log(app, "  [close] Focus CapCut trước Alt+F4...");
    focus_capcut_log(app);
    thread::sleep(Duration::from_millis(200));
    emit_log(app, "  [close] Gửi Alt+F4...");
    let _ = enigo.key(Key::Alt, Direction::Press);
    let _ = enigo.key(Key::F4, Direction::Click);
    let _ = enigo.key(Key::Alt, Direction::Release);
}
