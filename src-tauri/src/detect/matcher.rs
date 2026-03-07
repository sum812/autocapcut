/// matcher.rs — Template matching dùng SSD (Sum of Squared Differences) trên grayscale.
///
/// Thuật toán:
/// 1. Convert ảnh và template sang grayscale (luminance).
/// 2. Trượt template qua toàn bộ ảnh, tính SSD tại mỗi vị trí.
/// 3. Trả về vị trí có SSD nhỏ nhất cùng với score trung bình/pixel.

/// Convert RGBA pixel thành grayscale (luminance).
#[inline]
fn to_gray(r: u8, g: u8, b: u8) -> f32 {
    0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
}

/// Kết quả match tốt nhất.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Tọa độ góc trên-trái của vùng khớp tốt nhất trong ảnh nguồn.
    pub x: u32,
    pub y: u32,
    /// Tâm của vùng khớp.
    pub cx: u32,
    pub cy: u32,
    /// SSD trung bình mỗi pixel (thấp = khớp tốt hơn).
    pub avg_ssd: f32,
}

/// Kích thước template chuẩn (pixels).
pub const TEMPLATE_SIZE: u32 = 48;

/// Ngưỡng avg_ssd tối đa để coi là match thành công.
/// Giá trị 1500.0 tương đương sai lệch trung bình ~38 mức xám/pixel.
pub const SSD_THRESHOLD: f32 = 1500.0;

/// Tìm vị trí template trong ảnh nguồn bằng SSD.
///
/// - `src_rgba`: RGBA bytes của ảnh nguồn
/// - `src_w`, `src_h`: kích thước ảnh nguồn
/// - `tmpl_rgba`: RGBA bytes của template
/// - `tmpl_w`, `tmpl_h`: kích thước template (thường là TEMPLATE_SIZE × TEMPLATE_SIZE)
///
/// Trả về `None` nếu ảnh/template quá nhỏ hoặc không có vùng nào vượt ngưỡng.
pub fn find_template(
    src_rgba: &[u8],
    src_w: u32,
    src_h: u32,
    tmpl_rgba: &[u8],
    tmpl_w: u32,
    tmpl_h: u32,
) -> Option<MatchResult> {
    if tmpl_w == 0 || tmpl_h == 0 || src_w < tmpl_w || src_h < tmpl_h {
        return None;
    }

    // Precompute grayscale cho template
    let tmpl_gray: Vec<f32> = tmpl_rgba
        .chunks_exact(4)
        .map(|p| to_gray(p[0], p[1], p[2]))
        .collect();

    let search_w = src_w - tmpl_w;
    let search_h = src_h - tmpl_h;
    let n_pixels = (tmpl_w * tmpl_h) as f32;

    let mut best_ssd = f64::MAX;
    let mut best_x = 0u32;
    let mut best_y = 0u32;

    for sy in 0..=search_h {
        for sx in 0..=search_w {
            let mut ssd = 0f64;

            'tmpl: for ty in 0..tmpl_h {
                for tx in 0..tmpl_w {
                    let src_idx = ((sy + ty) * src_w + (sx + tx)) as usize * 4;
                    let tmpl_idx = (ty * tmpl_w + tx) as usize;

                    if src_idx + 3 >= src_rgba.len() {
                        break 'tmpl;
                    }

                    let sg = to_gray(src_rgba[src_idx], src_rgba[src_idx + 1], src_rgba[src_idx + 2]) as f64;
                    let tg = tmpl_gray[tmpl_idx] as f64;
                    let diff = sg - tg;
                    ssd += diff * diff;

                    // Early exit: nếu đã vượt best thì bỏ
                    if ssd > best_ssd {
                        break 'tmpl;
                    }
                }
            }

            if ssd < best_ssd {
                best_ssd = ssd;
                best_x = sx;
                best_y = sy;
            }
        }
    }

    let avg_ssd = (best_ssd / n_pixels as f64) as f32;

    Some(MatchResult {
        x: best_x,
        y: best_y,
        cx: best_x + tmpl_w / 2,
        cy: best_y + tmpl_h / 2,
        avg_ssd,
    })
}
