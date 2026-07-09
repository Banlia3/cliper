pub mod migrations;
pub mod models;
pub mod queries;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

/// 数据库管理器：封装 SQLite 连接
pub struct Database {
    pub conn: Mutex<Connection>,
    pub db_path: PathBuf,
}

impl Database {
    /// 初始化数据库（确保目录存在、迁移执行）
    pub fn new(app_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db_dir = Self::get_db_dir(app_name);
        std::fs::create_dir_all(&db_dir)?;

        let db_path = db_dir.join("clipboard_history.db");
        let conn = Connection::open(&db_path)?;

        // 执行迁移
        migrations::run_migrations(&conn)?;

        log::info!("数据库已初始化: {:?}", db_path);

        Ok(Self {
            conn: Mutex::new(conn),
            db_path,
        })
    }

    /// 获取数据库目录路径：%APPDATA%/<app_name>/
    fn get_db_dir(app_name: &str) -> PathBuf {
        let base = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                // 后备：用户主目录
                let home = std::env::var("USERPROFILE")
                    .or_else(|_| std::env::var("HOME"))
                    .unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home)
            });
        base.join(app_name)
    }
}
