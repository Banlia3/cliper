use tauri::State;
use crate::db::Database;
use crate::db::models::{Folder, FolderWithEntryCount, ClipboardEntrySummary};
use crate::db::queries;
use std::sync::Arc;

/// 列出所有文件夹（含条目计数）
#[tauri::command]
pub fn list_folders(
    db: State<'_, Arc<Database>>,
) -> Result<Vec<FolderWithEntryCount>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    queries::list_folders(&conn).map_err(|e| e.to_string())
}

/// 创建文件夹
#[tauri::command]
pub fn create_folder(
    db: State<'_, Arc<Database>>,
    name: String,
) -> Result<Folder, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    queries::create_folder(&conn, &name).map_err(|e| e.to_string())
}

/// 重命名文件夹
#[tauri::command]
pub fn rename_folder(
    db: State<'_, Arc<Database>>,
    id: i64,
    name: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    queries::rename_folder(&conn, id, &name).map_err(|e| e.to_string())
}

/// 删除自定义文件夹（默认收藏夹不会被删除）
#[tauri::command]
pub fn delete_folder(
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    queries::delete_folder(&conn, id).map_err(|e| e.to_string())
}

/// 添加条目到文件夹
#[tauri::command]
pub fn add_to_folder(
    db: State<'_, Arc<Database>>,
    folder_id: i64,
    entry_id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    queries::add_entry_to_folder(&conn, folder_id, entry_id).map_err(|e| e.to_string())
}

/// 从文件夹移除条目
#[tauri::command]
pub fn remove_from_folder(
    db: State<'_, Arc<Database>>,
    folder_id: i64,
    entry_id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    queries::remove_entry_from_folder(&conn, folder_id, entry_id).map_err(|e| e.to_string())
}

/// 分页获取文件夹内的条目
#[tauri::command]
pub fn get_folder_entries(
    db: State<'_, Arc<Database>>,
    folder_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClipboardEntrySummary>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let entries = queries::get_folder_entries(&conn, folder_id, limit, offset)
        .map_err(|e| e.to_string())?;
    Ok(entries.into_iter().map(ClipboardEntrySummary::from).collect())
}

/// 获取条目的所属文件夹ID列表
#[tauri::command]
pub fn get_entry_folders(
    db: State<'_, Arc<Database>>,
    entry_id: i64,
) -> Result<Vec<i64>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    queries::get_entry_folders(&conn, entry_id).map_err(|e| e.to_string())
}
