use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

/// 文件日志器：将日志写入指定文件（同时保留 stderr 输出）
pub struct FileLogger {
    file: Option<Mutex<File>>,
}

impl FileLogger {
    /// 初始化日志系统
    /// `log_dir`: 日志文件存放目录
    /// `log_file_name`: 日志文件名
    /// `level`: 日志级别
    pub fn init(log_dir: &Path, log_file_name: &str, level: LevelFilter) -> Result<(), SetLoggerError> {
        // 确保日志目录存在
        let _ = std::fs::create_dir_all(log_dir);

        let log_path = log_dir.join(log_file_name);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok()
            .map(|f| Mutex::new(f));

        log_panics(level);

        let logger = Box::new(FileLogger { file });

        // 写一条启动标记（在 leak 之前访问 logger.file）
        if let Some(ref mtx) = logger.file {
            if let Ok(mut f) = mtx.lock() {
                let _ = writeln!(f, "\n--- {} 日志启动 ---",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"));
                let _ = f.flush();
            }
        }

        log::set_logger(Box::leak(logger))?;
        log::set_max_level(level);

        // 日志系统已就绪，可以打印日志了
        log::info!("📝 文件日志已初始化: {:?}", log_path);

        Ok(())
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let msg = format!("[{} {}] {}",
            timestamp,
            record.level(),
            record.args()
        );

        // 始终输出到 stderr（开发控制台可见）
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = std::io::Write::write(
                &mut std::io::stderr(),
                format!("{}\n", msg).as_bytes(),
            );
        }

        // 写入文件
        if let Some(ref mtx) = self.file {
            if let Ok(mut f) = mtx.lock() {
                let _ = writeln!(f, "{}", msg);
                let _ = f.flush();
            }
        }
    }

    fn flush(&self) {
        if let Some(ref mtx) = self.file {
            if let Ok(mut f) = mtx.lock() {
                let _ = f.flush();
            }
        }
    }
}

/// 捕获 panic 并写入日志
fn log_panics(max_level: LevelFilter) {
    if max_level >= LevelFilter::Error {
        std::panic::set_hook(Box::new(|info| {
            let msg = match (info.payload().downcast_ref::<&str>(), info.payload().downcast_ref::<String>()) {
                (Some(&s), _) => s.to_string(),
                (_, Some(s)) => s.clone(),
                _ => format!("{:?}", info.payload()),
            };
            let location = info.location().map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "未知位置".to_string());

            log::error!("💥 PANIC! {} — {}", location, msg);
        }));
    }
}
