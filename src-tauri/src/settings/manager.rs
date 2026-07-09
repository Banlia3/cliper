use rusqlite::params;
use std::collections::HashMap;

use crate::db::Database;

/// 设置管理器
pub struct SettingsManager;

impl SettingsManager {
    /// 获取所有设置
    pub fn get_all(db: &Database) -> Result<HashMap<String, String>, String> {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;

        let mut map = HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(|e| e.to_string())?;
            map.insert(k, v);
        }
        Ok(map)
    }

    /// 获取单个设置值
    pub fn get(db: &Database, key: &str) -> Result<Option<String>, String> {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        let result = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .ok();
        Ok(result)
    }

    /// 设置值
    pub fn set(db: &Database, key: &str, value: &str) -> Result<(), String> {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO settings (key, value, updated_at)
             VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
             ON CONFLICT(key) DO UPDATE SET
               value = excluded.value,
               updated_at = excluded.updated_at",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Tauri 命令：获取所有设置
#[tauri::command]
pub fn get_settings(
    db: tauri::State<'_, std::sync::Arc<Database>>,
) -> Result<HashMap<String, String>, String> {
    SettingsManager::get_all(&db)
}

/// Tauri 命令：更新单个设置
#[tauri::command]
pub fn update_settings(
    db: tauri::State<'_, std::sync::Arc<Database>>,
    key: String,
    value: String,
) -> Result<(), String> {
    SettingsManager::set(&db, &key, &value)
}
