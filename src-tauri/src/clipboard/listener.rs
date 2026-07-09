use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::reader;
use super::dedup::DedupEngine;

/// 剪贴板事件（从监听线程发送到主线程）
#[derive(Debug, Clone)]
pub struct ClipboardEvent {
    pub text_preview: String,
    pub content_hash: String,
    pub content_type: String,
    pub raw_content: Option<Vec<u8>>,
    pub content_size: i64,
    pub source_app: String,
    pub source_class: String,
}

/// 剪贴板监听器：后台轮询 GetClipboardSequenceNumber
pub struct ClipboardListener {
    control_tx: Sender<()>,
    event_rx: Arc<Mutex<Receiver<ClipboardEvent>>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl ClipboardListener {
    /// 启动监听器
    pub fn start(dedup: Arc<DedupEngine>) -> Self {
        let (control_tx, control_rx) = mpsc::channel::<()>();
        let (event_tx, event_rx) = mpsc::channel::<ClipboardEvent>();

        let handle = thread::spawn(move || {
            Self::listener_thread(dedup, event_tx, control_rx);
        });

        Self {
            control_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            thread_handle: Some(handle),
        }
    }

    /// 停止监听器
    pub fn stop(&mut self) {
        let _ = self.control_tx.send(());
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// 获取事件接收器
    pub fn get_event_receiver(&self) -> Arc<Mutex<Receiver<ClipboardEvent>>> {
        self.event_rx.clone()
    }

    /// 后台监听线程（轮询模式）
    fn listener_thread(
        dedup: Arc<DedupEngine>,
        event_tx: Sender<ClipboardEvent>,
        control_rx: Receiver<()>,
    ) {
        let mut last_seq = 0u64;
        log::info!("✅ 剪贴板监听器启动完成（轮询模式）");

        loop {
            // 检查停止信号（非阻塞）
            if control_rx.try_recv().is_ok() {
                break;
            }

            let current_seq = reader::current_sequence_number();

            if current_seq != 0 && current_seq != last_seq {
                // 层1: 序列号检查（防回环）
                // 自己的写入 → 永久跳过，消耗序列号
                if dedup.is_own_write_by_seq(current_seq) {
                    last_seq = current_seq;
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // 层4: 防抖检查（不消耗序列号，防抖过期后重试）
                if dedup.is_in_debounce_window() {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // 隐私排除格式检查（不消耗序列号，内容变更后自然跳过）
                if reader::has_privacy_exclusion_format() {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                // 判断剪贴板内容类型并读取
                let read_result = if reader::has_clipboard_image() {
                    log::info!("检测到剪贴板图片");
                    reader::read_clipboard_image()
                } else {
                    reader::read_clipboard_text()
                };

                let Some((preview, content_type, raw_content, content_size)) = read_result
                else {
                    log::warn!("读取剪贴板失败，序列号={}，下次轮询重试", current_seq);
                    thread::sleep(Duration::from_millis(100));
                    continue;
                };

                // ✅ 读取成功，记录序列号防止重复处理
                last_seq = current_seq;

                // 获取源应用信息
                let (source_app, source_class) = Self::get_source_app_info();

                // 计算内容哈希
                let hash = if let Some(ref raw) = raw_content {
                    DedupEngine::compute_hash(raw)
                } else {
                    DedupEngine::compute_hash(preview.as_bytes())
                };

                // 层2: 哈希去重
                if dedup.is_duplicate_hash(&hash) {
                    log::info!("跳过重复内容，hash={}", &hash[..12]);
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                let preview_short: String = preview.chars().take(40).collect();
                log::info!("📋 新剪贴板内容: {} | {} | {}",
                    preview_short,
                    &content_type,
                    &hash[..12]
                );

                // 发送事件给主线程
                let event = ClipboardEvent {
                    text_preview: preview,
                    content_hash: hash,
                    content_type,
                    raw_content,
                    content_size,
                    source_app,
                    source_class,
                };

                if event_tx.send(event).is_err() {
                    log::warn!("发送剪贴板事件失败（接收端已关闭）");
                    break;
                }
            }

            thread::sleep(Duration::from_millis(100));
        }

        log::info!("剪贴板监听器已停止");
    }

    /// 获取当前前台窗口标题和类名（来源应用）
    fn get_source_app_info() -> (String, String) {
        unsafe {
            // 用 extern 声明需要的 Win32 函数（避免 windows-sys 模块路径问题）
            extern "system" {
                fn GetForegroundWindow() -> isize;
                fn GetWindowTextLengthW(hwnd: isize) -> i32;
                fn GetWindowTextW(hwnd: isize, lpstring: *mut u16, nmaxcount: i32) -> i32;
                fn GetClassNameW(hwnd: isize, lpclassname: *mut u16, nmaxcount: i32) -> i32;
            }

            let hwnd = GetForegroundWindow();
            if hwnd == 0 {
                return (String::new(), String::new());
            }

            let len = GetWindowTextLengthW(hwnd);
            if len == 0 {
                return (String::new(), String::new());
            }

            let mut buf = vec![0u16; (len + 1) as usize];
            let actual = GetWindowTextW(hwnd, buf.as_mut_ptr(), len + 1);
            buf.truncate(actual.max(0) as usize);
            let title = String::from_utf16_lossy(&buf);

            // 窗口类名
            let mut class_buf = [0u16; 256];
            let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 256);
            let class_name = if class_len > 0 {
                String::from_utf16_lossy(&class_buf[..class_len as usize])
            } else {
                String::new()
            };

            (title, class_name)
        }
    }
}
