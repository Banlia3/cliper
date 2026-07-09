use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, OpenClipboard,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable,
    RegisterClipboardFormatW,
};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock, GlobalSize};
use windows_sys::Win32::Foundation::GetLastError;

use image::ImageEncoder;

/// 从剪贴板读取文本内容
/// 返回: (预览文本, 内容类型, 原始字节, 字节大小)
pub fn read_clipboard_text() -> Option<(String, String, Option<Vec<u8>>, i64)> {
    // 尝试打开剪贴板（最多重试 3 次）
    if !try_open_clipboard(3) {
        log::warn!("打开剪贴板失败，其他程序可能正占用");
        return None;
    }

    let result = read_text_internal();

    // 关闭剪贴板
    unsafe { CloseClipboard() };

    result
}

/// 打开剪贴板后读取 CF_UNICODETEXT
fn read_text_internal() -> Option<(String, String, Option<Vec<u8>>, i64)> {
    // CF_UNICODETEXT = 13
    // GetClipboardData 返回 HANDLE (= isize)
    let handle: isize = unsafe { GetClipboardData(13) };
    if handle == 0 {
        let err = unsafe { GetLastError() };
        log::warn!("获取 CF_UNICODETEXT 失败, error: {}", err);
        return None;
    }

    // 转为 HGLOBAL (*mut c_void) 给 GlobalSize / GlobalLock
    let hglobal = handle as *mut std::ffi::c_void;
    let size = unsafe { GlobalSize(hglobal) };
    if size == 0 {
        return None;
    }

    let locked = unsafe { GlobalLock(hglobal) };
    if locked.is_null() {
        return None;
    }

    // UTF-16LE 转 Rust String
    let utf16_slice = unsafe {
        std::slice::from_raw_parts(locked as *const u16, size as usize / 2)
    };

    // 去除末尾空字符
    let effective_len = utf16_slice.iter().position(|&c| c == 0).unwrap_or(utf16_slice.len());
    let text = String::from_utf16_lossy(&utf16_slice[..effective_len]);

    unsafe { GlobalUnlock(hglobal); }

    // 预览取前 200 字符
    let preview = text.chars().take(200).collect::<String>();
    let raw_bytes = Some(text.as_bytes().to_vec());

    Some((preview, "text".to_string(), raw_bytes, size as i64))
}

/// 尝试打开剪贴板（指数退避重试）
fn try_open_clipboard(max_retries: u32) -> bool {
    for attempt in 1..=max_retries {
        // OpenClipboard 接受 HWND (= isize)，传递 0 表示 NULL
        let result = unsafe { OpenClipboard(0) };
        if result != 0 {
            return true;
        }
        if attempt < max_retries {
            std::thread::sleep(std::time::Duration::from_millis(10 * u64::from(attempt)));
        }
    }
    false
}

/// 获取当前剪贴板序列号（用于防回环）
pub fn current_sequence_number() -> u64 {
    // windows-sys 返回 u32，转 u64 以匹配 DedupEngine 的 AtomicU64
    unsafe { GetClipboardSequenceNumber() as u64 }
}

/// 检查剪贴板是否包含图片格式（CF_DIBV5 / CF_DIB）
pub fn has_clipboard_image() -> bool {
    unsafe {
        // CF_DIBV5 = 17, CF_DIB = 8
        IsClipboardFormatAvailable(17) != 0 || IsClipboardFormatAvailable(8) != 0
    }
}

/// 从剪贴板读取图片内容
/// 返回: (预览文本, "image", Some(PNG字节), 字节大小)
pub fn read_clipboard_image() -> Option<(String, String, Option<Vec<u8>>, i64)> {
    if !try_open_clipboard(3) {
        log::warn!("打开剪贴板失败，无法读取图片");
        return None;
    }

    let result = read_image_internal();
    unsafe { CloseClipboard() };
    result
}

/// 打开剪贴板后读取图片（优先 CF_DIBV5，回退 CF_DIB）
fn read_image_internal() -> Option<(String, String, Option<Vec<u8>>, i64)> {
    for &fmt in &[17, 8] {
        let handle: isize = unsafe { GetClipboardData(fmt) };
        if handle == 0 {
            continue;
        }

        let hglobal = handle as *mut std::ffi::c_void;
        let size = unsafe { GlobalSize(hglobal) };
        if size == 0 {
            continue;
        }

        let locked = unsafe { GlobalLock(hglobal) };
        if locked.is_null() {
            continue;
        }

        let dib_data = unsafe {
            std::slice::from_raw_parts(locked as *const u8, size as usize)
        }
        .to_vec();

        unsafe { GlobalUnlock(hglobal); }

        // 尝试将 DIB 数据转为 PNG
        if let Some((preview, png_data)) = dib_to_png(&dib_data) {
            let png_size = png_data.len() as i64;
            return Some((preview, "image".to_string(), Some(png_data), png_size));
        }
    }

    None
}

/// 将 DIB 裸数据（BITMAPV5HEADER / BITMAPINFOHEADER + 像素）转为 PNG 字节
///
/// 手动解析 DIB 头，提取 BGRA/BGR 像素，转为 RGBA，用 image crate 编码为 PNG。
/// 只处理 32-bit 和 24-bit 位图（最常见的剪贴板格式）。
fn dib_to_png(dib_data: &[u8]) -> Option<(String, Vec<u8>)> {
    if dib_data.len() < 40 {
        return None;
    }

    let header_size = u32::from_le_bytes(dib_data[0..4].try_into().ok()?) as usize;
    if header_size < 40 || header_size > dib_data.len() {
        return None;
    }

    let width_raw = i32::from_le_bytes(dib_data[4..8].try_into().ok()?);
    if width_raw <= 0 {
        return None;
    }
    let width = width_raw as u32;

    let height_raw = i32::from_le_bytes(dib_data[8..12].try_into().ok()?);
    let height_abs = height_raw.unsigned_abs();
    let bit_count = u16::from_le_bytes(dib_data[14..16].try_into().ok()?);

    // 只支持 24-bit (BGR) 和 32-bit (BGRA)
    if bit_count != 24 && bit_count != 32 {
        return None;
    }

    let bottom_up = height_raw > 0; // 正高度 = bottom-up
    let bpp = bit_count as u32;
    let bytes_per_pixel = (bpp / 8) as usize;

    // 每行字节数（DIB 行对齐到 4 字节）
    let row_bytes = ((u64::from(width) * u64::from(bpp) + 31) / 32 * 4) as usize;

    let pixel_offset = header_size; // 24/32-bit 没有调色板
    let available = dib_data.len().saturating_sub(pixel_offset);
    if available < row_bytes {
        // 连一行都不够，放弃
        return None;
    }

    let mut rgba = Vec::with_capacity((width * height_abs * 4) as usize);

    for y in 0..height_abs {
        // bottom-up：DIB 的行 0 是底行
        let src_y = if bottom_up {
            height_abs - 1 - y
        } else {
            y
        };
        let row_start = pixel_offset + (src_y as usize) * row_bytes;
        let row_end = (row_start + width as usize * bytes_per_pixel).min(dib_data.len());
        let row = &dib_data[row_start..row_end];

        for x in 0..width as usize {
            let px = x * bytes_per_pixel;
            if px + bytes_per_pixel <= row.len() {
                // DIB 像素顺序是 B, G, R [, A]
                // image crate 需要 R, G, B, A
                rgba.push(row[px + 2]); // R
                rgba.push(row[px + 1]); // G
                rgba.push(row[px]);     // B
                rgba.push(if bit_count == 32 { row[px + 3] } else { 255 }); // A
            }
        }
    }

    // 用 image crate 编码为 PNG
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    encoder
        .write_image(&rgba, width, height_abs, image::ColorType::Rgba8.into())
        .ok()?;

    let preview = format!("{}×{} 图片", width, height_abs);
    Some((preview, png))
}

/// 检测是否存在隐私排除格式（某些应用标记"请勿记录"）
///
/// 移除了 `CanIncludeInClipboardHistory` — 这个格式是 Windows 剪贴板历史 API
/// 用的权限标志（0=排除, 1=包含），大多数应用设 1（允许记录），不应该跳过。
/// 真正需要跳过的内容会使用 `ExcludeClipboardContentFromMonitorProcessing`。
pub fn has_privacy_exclusion_format() -> bool {
    let names = [
        "Clipboard Viewer Ignore\0",
        "ExcludeClipboardContentFromMonitorProcessing\0",
    ];

    for name in &names {
        let wide: Vec<u16> = name.encode_utf16().collect();
        let fmt = unsafe { RegisterClipboardFormatW(wide.as_ptr()) };
        if fmt != 0 {
            let available = unsafe { IsClipboardFormatAvailable(fmt) };
            if available != 0 {
                log::info!("检测到隐私排除格式: {}", name.trim_end_matches('\0'));
                return true;
            }
        }
    }
    false
}
