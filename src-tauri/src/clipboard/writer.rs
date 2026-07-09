use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, OpenClipboard, SetClipboardData, EmptyClipboard,
    GetClipboardSequenceNumber,
};
use windows_sys::Win32::System::Memory::{
    GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
};
use windows_sys::Win32::Foundation::GetLastError;
use std::ptr;

use super::dedup::DedupEngine;

// GlobalFree 在 windows-sys 0.52 中不存在，自行 extern 声明
extern "system" {
    fn GlobalFree(h: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

/// 将文本写入剪贴板
/// 写入前调用 ExcludeClipboardContentFromMonitorProcessing 防回环
pub fn write_text_to_clipboard(
    text: &str,
    dedup: &DedupEngine,
) -> Result<u64, String> {
    // 层3: 写入前调用排除标记
    DedupEngine::set_exclude_from_monitoring();

    // 打开剪贴板
    if !try_open_clipboard(3) {
        return Err("无法打开剪贴板".to_string());
    }

    let result = write_text_internal(text);

    // 关闭剪贴板
    unsafe { CloseClipboard() };

    match result {
        Ok(()) => {
            // 记录本次写入的序列号和哈希，供防回环使用
            let seq = unsafe { GetClipboardSequenceNumber() as u64 };
            let hash = DedupEngine::compute_hash(text.as_bytes());
            dedup.record_write(seq, hash);
            log::info!("剪贴板写入成功, seq={}", seq);
            Ok(seq)
        }
        Err(e) => Err(e),
    }
}

/// 内部写入文本（在 OpenClipboard 之后调用）
fn write_text_internal(text: &str) -> Result<(), String> {
    // 清空剪贴板
    let empty_result = unsafe { EmptyClipboard() };
    if empty_result == 0 {
        return Err(format!("EmptyClipboard 失败: {}", unsafe { GetLastError() }));
    }

    // 将 Rust 字符串转为 UTF-16 并分配全局内存
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_size = wide.len() * 2;

    // 分配全局内存 — 返回 HGLOBAL (= *mut c_void)
    let hglobal = unsafe {
        GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, byte_size)
    };

    if hglobal.is_null() {
        return Err("GlobalAlloc 失败".to_string());
    }

    // 锁定内存并复制数据
    let locked = unsafe { GlobalLock(hglobal) };
    if locked.is_null() {
        unsafe { GlobalFree(hglobal); }
        return Err("GlobalLock 失败".to_string());
    }

    unsafe {
        ptr::copy_nonoverlapping(wide.as_ptr(), locked as *mut u16, wide.len());
        GlobalUnlock(hglobal);
    }

    // CF_UNICODETEXT = 13
    // SetClipboardData 接受 HANDLE (= isize)，需要转换
    let set_result = unsafe { SetClipboardData(13, hglobal as isize) };
    if set_result == 0 {
        let err = unsafe { GetLastError() };
        unsafe { GlobalFree(hglobal); }
        return Err(format!("SetClipboardData 失败: {}", err));
    }

    // 注意: 设置成功后 SetClipboardData 接管了 hglobal 所有权，不需要 Free
    Ok(())
}

/// 将图片（PNG 字节）写入剪贴板
/// 解码 PNG → RGBA → 构建 BITMAPV5HEADER + BGRA 像素 → CF_DIBV5
pub fn write_image_to_clipboard(
    png_data: &[u8],
    dedup: &DedupEngine,
) -> Result<u64, String> {
    // 层3: 写入前调用排除标记
    DedupEngine::set_exclude_from_monitoring();

    // 解码 PNG
    let img = image::load_from_memory(png_data)
        .map_err(|e| format!("图片解码失败: {}", e))?;
    let rgba = img.to_rgba8();
    let w = rgba.width();
    let h = rgba.height();

    // 每行字节数（32-bit = 4 字节/像素，天然 4 字节对齐）
    let row_bytes = w as usize * 4;
    let pixel_data_size = row_bytes * h as usize;

    // 构建 BITMAPV5HEADER (124 字节) + BGRA 像素数据
    let mut dib = Vec::with_capacity(124 + pixel_data_size);

    // ---- BITMAPV5HEADER ----
    // 1. bV5Size
    dib.extend_from_slice(&124u32.to_le_bytes());
    // 2. bV5Width
    dib.extend_from_slice(&(w as i32).to_le_bytes());
    // 3. bV5Height (正数 = bottom-up)
    dib.extend_from_slice(&(h as i32).to_le_bytes());
    // 4. bV5Planes
    dib.extend_from_slice(&1u16.to_le_bytes());
    // 5. bV5BitCount
    dib.extend_from_slice(&32u16.to_le_bytes());
    // 6. bV5Compression (BI_RGB = 0)
    dib.extend_from_slice(&0u32.to_le_bytes());
    // 7. bV5SizeImage
    dib.extend_from_slice(&(pixel_data_size as u32).to_le_bytes());
    // 8-9. bV5XPelsPerMeter, bV5YPelsPerMeter
    dib.extend_from_slice(&0i32.to_le_bytes());
    dib.extend_from_slice(&0i32.to_le_bytes());
    // 10-11. bV5ClrUsed, bV5ClrImportant
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    // 12-15. 颜色掩码 (BGRA 顺序)
    dib.extend_from_slice(&0x00FF0000u32.to_le_bytes()); // bV5RedMask
    dib.extend_from_slice(&0x0000FF00u32.to_le_bytes()); // bV5GreenMask
    dib.extend_from_slice(&0x000000FFu32.to_le_bytes()); // bV5BlueMask
    dib.extend_from_slice(&0xFF000000u32.to_le_bytes()); // bV5AlphaMask
    // 16. bV5CSType (LCS_sRGB)
    dib.extend_from_slice(&0x73524742u32.to_le_bytes()); // 'sRGB'
    // 17. bV5Endpoints (36 字节，置零)
    dib.extend_from_slice(&[0u8; 36]);
    // 18-20. bV5GammaRed/Green/Blue
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    // 21. bV5Intent
    dib.extend_from_slice(&0u32.to_le_bytes());
    // 22-23. bV5ProfileData, bV5ProfileSize
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    // 24. bV5Reserved
    dib.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(dib.len(), 124, "BITMAPV5HEADER 必须是 124 字节");

    // ---- 像素数据 (BGRA, bottom-up) ----
    for y in (0..h).rev() {
        for x in 0..w {
            let p = rgba.get_pixel(x, y);
            dib.push(p[2]); // B
            dib.push(p[1]); // G
            dib.push(p[0]); // R
            dib.push(p[3]); // A
        }
    }

    debug_assert_eq!(dib.len(), 124 + pixel_data_size);

    // ---- 设置剪贴板 ----
    if !try_open_clipboard(3) {
        return Err("无法打开剪贴板".to_string());
    }

    // 空剪贴板
    let empty_result = unsafe { EmptyClipboard() };
    if empty_result == 0 {
        unsafe { CloseClipboard(); }
        return Err(format!("EmptyClipboard 失败: {}", unsafe { GetLastError() }));
    }

    // 分配全局内存
    let hglobal = unsafe {
        GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, dib.len())
    };
    if hglobal.is_null() {
        unsafe { CloseClipboard(); }
        return Err("GlobalAlloc 失败".to_string());
    }

    let locked = unsafe { GlobalLock(hglobal) };
    if locked.is_null() {
        unsafe { GlobalFree(hglobal); CloseClipboard(); }
        return Err("GlobalLock 失败".to_string());
    }

    unsafe {
        std::ptr::copy_nonoverlapping(dib.as_ptr(), locked as *mut u8, dib.len());
        GlobalUnlock(hglobal);
    }

    // CF_DIBV5 = 17
    let set_result = unsafe { SetClipboardData(17, hglobal as isize) };
    if set_result == 0 {
        let err = unsafe { GetLastError() };
        unsafe { GlobalFree(hglobal); }
        unsafe { CloseClipboard(); }
        return Err(format!("SetClipboardData(CF_DIBV5) 失败: {}", err));
    }

    unsafe { CloseClipboard(); };

    // 记录本次写入的序列号和哈希
    let seq = unsafe { GetClipboardSequenceNumber() as u64 };
    let hash = DedupEngine::compute_hash(png_data);
    dedup.record_write(seq, hash);
    log::info!("图片剪贴板写入成功, seq={}, {}×{}", seq, w, h);

    Ok(seq)
}

/// 尝试打开剪贴板（带重试）
fn try_open_clipboard(max_retries: u32) -> bool {
    for attempt in 1..=max_retries {
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
