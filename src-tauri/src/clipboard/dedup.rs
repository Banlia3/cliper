use sha2::{Sha256, Digest};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// 四层防回环去重引擎
///
/// 层1: 序列号对比 —— 如果当前序列号等于上次写入的序列号，跳过
/// 层2: 内容哈希 —— SHA-256 哈希对比，跳过重复内容
/// 层3: ExcludeClipboardContentFromMonitorProcessing —— 写入前标记
/// 层4: 防抖计时器 —— 自写入后 50ms 内忽略所有事件
pub struct DedupEngine {
    /// 上次本程序写入时的剪贴板序列号
    last_written_seq: AtomicU64,
    /// 上次写入内容的哈希
    last_content_hash: Mutex<String>,
    /// 上次写入的时间戳（用于防抖）
    last_write_time: Mutex<Instant>,
    /// 防抖窗口（毫秒）
    debounce_ms: u64,
}

impl DedupEngine {
    pub fn new(debounce_ms: u64) -> Self {
        Self {
            last_written_seq: AtomicU64::new(0),
            last_content_hash: Mutex::new(String::new()),
            last_write_time: Mutex::new(Instant::now()),
            debounce_ms,
        }
    }

    /// 层1: 检查序列号 —— 是不是自己上次写入的？
    pub fn is_own_write_by_seq(&self, current_seq: u64) -> bool {
        let last = self.last_written_seq.load(Ordering::Relaxed);
        if last != 0 && current_seq == last {
            log::info!("防回环-层1: 序列号相同 ({}), 跳过", current_seq);
            return true;
        }
        false
    }

    /// 层2: 计算内容 SHA-256 哈希
    pub fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// 层2: 检查哈希是否与上次记录相同
    pub fn is_duplicate_hash(&self, hash: &str) -> bool {
        let last = self.last_content_hash.lock().unwrap();
        if !last.is_empty() && *last == hash {
            log::info!("防回环-层2: 内容哈希相同, 跳过");
            return true;
        }
        false
    }

    /// 层3: 调用 ExcludeClipboardContentFromMonitorProcessing
    /// 在写入剪贴板前调用，告诉 Windows 不要发送 WM_CLIPBOARDUPDATE
    /// 注意：此函数必须在支持的系统（Windows 10+）上才有效
    /// 使用 GetProcAddress 动态加载以避免链接器找不到符号
    pub fn set_exclude_from_monitoring() {
        #[cfg(windows)]
        unsafe {
            use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

            type ExcludeFn = unsafe extern "system" fn() -> i32;

            // 加载 user32.dll 获取函数地址（user32 在 GUI 进程中始终已加载）
            let module_name: Vec<u16> = "user32.dll\0".encode_utf16().collect();
            let module = GetModuleHandleW(module_name.as_ptr());
            if module == 0 {
                return;
            }

            let func_name: Vec<u8> = "ExcludeClipboardContentFromMonitorProcessing\0"
                .bytes()
                .collect();
            let proc = GetProcAddress(module, func_name.as_ptr());

            if let Some(func) = proc {
                let result = func();
                if result != 0 {
                    log::info!("防回环-层3: 已调用 ExcludeClipboardContentFromMonitorProcessing");
                } else {
                    log::warn!("防回环-层3: ExcludeClipboardContentFromMonitorProcessing 返回 {}", result);
                }
            } else {
                log::info!("防回环-层3: ExcludeClipboardContentFromMonitorProcessing 不可用（旧系统）");
            }
        }
    }

    /// 层4: 检查是否在防抖窗口内
    pub fn is_in_debounce_window(&self) -> bool {
        let last_write = *self.last_write_time.lock().unwrap();
        let elapsed = last_write.elapsed().as_millis() as u64;
        if elapsed < self.debounce_ms {
            log::info!("防回环-层4: 防抖中 ({}ms < {}ms), 跳过", elapsed, self.debounce_ms);
            return true;
        }
        false
    }

    /// 记录一次写入（由 writer 在成功写入后调用）
    pub fn record_write(&self, seq: u64, hash: String) {
        self.last_written_seq.store(seq, Ordering::Relaxed);
        *self.last_content_hash.lock().unwrap() = hash;
        *self.last_write_time.lock().unwrap() = Instant::now();
    }
}
