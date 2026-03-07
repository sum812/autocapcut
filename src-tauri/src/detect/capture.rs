/// capture.rs — Chụp vùng màn hình bằng Windows GDI (BitBlt / PrintWindow).
/// Dùng raw FFI giống pattern trong automation/window.rs.

#[cfg(target_os = "windows")]
mod platform {
    use std::ptr;

    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct BITMAPINFOHEADER {
        bi_size: u32,
        bi_width: i32,
        bi_height: i32,
        bi_planes: u16,
        bi_bit_count: u16,
        bi_compression: u32,
        bi_size_image: u32,
        bi_x_pels_per_meter: i32,
        bi_y_pels_per_meter: i32,
        bi_clr_used: u32,
        bi_clr_important: u32,
    }

    #[repr(C)]
    struct RGBQUAD {
        rgb_blue: u8,
        rgb_green: u8,
        rgb_red: u8,
        rgb_reserved: u8,
    }

    // BITMAPINFO = BITMAPINFOHEADER + 1 RGBQUAD (palette)
    #[repr(C)]
    struct BITMAPINFO {
        bmi_header: BITMAPINFOHEADER,
        bmi_colors: [RGBQUAD; 1],
    }

    const BI_RGB: u32 = 0;
    const DIB_RGB_COLORS: u32 = 0;
    const SRCCOPY: u32 = 0x00CC0020;
    const PW_RENDERFULLCONTENT: u32 = 0x00000002;

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateCompatibleDC(hdc: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn CreateCompatibleBitmap(
            hdc: *mut std::ffi::c_void,
            cx: i32,
            cy: i32,
        ) -> *mut std::ffi::c_void;
        fn SelectObject(
            hdc: *mut std::ffi::c_void,
            h: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
        fn BitBlt(
            hdc: *mut std::ffi::c_void,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            hdc_src: *mut std::ffi::c_void,
            x1: i32,
            y1: i32,
            rop: u32,
        ) -> i32;
        fn GetDIBits(
            hdc: *mut std::ffi::c_void,
            hbm: *mut std::ffi::c_void,
            start: u32,
            c_lines: u32,
            lp_v_bits: *mut std::ffi::c_void,
            lp_bmi: *mut BITMAPINFO,
            usage: u32,
        ) -> i32;
        fn DeleteDC(hdc: *mut std::ffi::c_void) -> i32;
        fn DeleteObject(ho: *mut std::ffi::c_void) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetWindowDC(hwnd: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn ReleaseDC(hwnd: *mut std::ffi::c_void, hdc: *mut std::ffi::c_void) -> i32;
        fn GetWindowRect(hwnd: *mut std::ffi::c_void, lp_rect: *mut RECT) -> i32;
        fn PrintWindow(
            hwnd: *mut std::ffi::c_void,
            hdc_blt: *mut std::ffi::c_void,
            n_flags: u32,
        ) -> i32;
    }

    fn make_bmi(width: i32, height: i32) -> BITMAPINFO {
        BITMAPINFO {
            bmi_header: BITMAPINFOHEADER {
                bi_size: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                bi_width: width,
                bi_height: -height, // top-down
                bi_planes: 1,
                bi_bit_count: 32,
                bi_compression: BI_RGB,
                bi_size_image: 0,
                bi_x_pels_per_meter: 0,
                bi_y_pels_per_meter: 0,
                bi_clr_used: 0,
                bi_clr_important: 0,
            },
            bmi_colors: [RGBQUAD { rgb_blue: 0, rgb_green: 0, rgb_red: 0, rgb_reserved: 0 }],
        }
    }

    /// Chụp vùng (x, y, width, height) trên màn hình bằng BitBlt.
    pub fn capture_screen_region(x: i32, y: i32, width: i32, height: i32) -> Option<(Vec<u8>, u32, u32)> {
        if width <= 0 || height <= 0 {
            return None;
        }
        unsafe {
            let screen_dc = GetWindowDC(ptr::null_mut());
            if screen_dc.is_null() {
                return None;
            }
            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_null() {
                ReleaseDC(ptr::null_mut(), screen_dc);
                return None;
            }
            let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
            if bitmap.is_null() {
                DeleteDC(mem_dc);
                ReleaseDC(ptr::null_mut(), screen_dc);
                return None;
            }
            let old_obj = SelectObject(mem_dc, bitmap);
            let blt_ok = BitBlt(mem_dc, 0, 0, width, height, screen_dc, x, y, SRCCOPY);

            let result = if blt_ok != 0 {
                let mut bmi = make_bmi(width, height);
                let mut buf = vec![0u8; (width * height) as usize * 4];
                let lines = GetDIBits(
                    mem_dc,
                    bitmap,
                    0,
                    height as u32,
                    buf.as_mut_ptr() as *mut _,
                    &mut bmi,
                    DIB_RGB_COLORS,
                );
                if lines > 0 {
                    // GDI trả BGRA → RGBA
                    for chunk in buf.chunks_exact_mut(4) {
                        chunk.swap(0, 2);
                        chunk[3] = 255;
                    }
                    Some((buf, width as u32, height as u32))
                } else {
                    None
                }
            } else {
                None
            };

            SelectObject(mem_dc, old_obj);
            DeleteObject(bitmap);
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
            result
        }
    }

    /// Chụp toàn bộ cửa sổ CapCut bằng PrintWindow (hoạt động kể cả khi bị che).
    pub fn capture_window(hwnd_isize: isize) -> Option<(Vec<u8>, u32, u32)> {
        unsafe {
            let hwnd = hwnd_isize as *mut std::ffi::c_void;
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            if GetWindowRect(hwnd, &mut rect) == 0 {
                return None;
            }
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            if width <= 0 || height <= 0 {
                return None;
            }

            let screen_dc = GetWindowDC(ptr::null_mut());
            let mem_dc = CreateCompatibleDC(screen_dc);
            let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
            let old_obj = SelectObject(mem_dc, bitmap);

            PrintWindow(hwnd, mem_dc, PW_RENDERFULLCONTENT);

            let mut bmi = make_bmi(width, height);
            let mut buf = vec![0u8; (width * height) as usize * 4];
            let lines = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                buf.as_mut_ptr() as *mut _,
                &mut bmi,
                DIB_RGB_COLORS,
            );

            let result = if lines > 0 {
                for chunk in buf.chunks_exact_mut(4) {
                    chunk.swap(0, 2);
                    chunk[3] = 255;
                }
                Some((buf, width as u32, height as u32))
            } else {
                None
            };

            SelectObject(mem_dc, old_obj);
            DeleteObject(bitmap);
            DeleteDC(mem_dc);
            ReleaseDC(ptr::null_mut(), screen_dc);
            result
        }
    }
}

/// Chụp vùng màn hình (x, y, w, h) — trả về (RGBA bytes, width, height).
pub fn capture_screen_region(x: i32, y: i32, width: i32, height: i32) -> Option<(Vec<u8>, u32, u32)> {
    #[cfg(target_os = "windows")]
    return platform::capture_screen_region(x, y, width, height);
    #[cfg(not(target_os = "windows"))]
    { let _ = (x, y, width, height); None }
}

/// Chụp cửa sổ bằng HWND (PrintWindow).
pub fn capture_window(hwnd: isize) -> Option<(Vec<u8>, u32, u32)> {
    #[cfg(target_os = "windows")]
    return platform::capture_window(hwnd);
    #[cfg(not(target_os = "windows"))]
    { let _ = hwnd; None }
}
