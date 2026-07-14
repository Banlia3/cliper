use tauri::State;
use crate::db::Database;
use crate::db::models::ClipboardEntrySummary;
use crate::db::queries;
use crate::clipboard::writer;
use crate::clipboard::dedup::DedupEngine;
use std::sync::Arc;

/// 获取历史记录（分页）
#[tauri::command]
pub fn get_history(
    db: State<'_, Arc<Database>>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClipboardEntrySummary>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let entries = queries::get_entries(&conn, limit, offset)
        .map_err(|e| e.to_string())?;
    Ok(entries.into_iter().map(ClipboardEntrySummary::from).collect())
}

/// 搜索历史
#[tauri::command]
pub fn search_history(
    db: State<'_, Arc<Database>>,
    query: String,
    limit: i64,
) -> Result<Vec<ClipboardEntrySummary>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let entries = queries::search_entries(&conn, &query, limit)
        .map_err(|e| e.to_string())?;
    Ok(entries.into_iter().map(ClipboardEntrySummary::from).collect())
}

/// 将指定条目写回剪贴板（支持文本和图片）
#[tauri::command]
pub fn copy_to_clipboard(
    db: State<'_, Arc<Database>>,
    dedup: State<'_, Arc<DedupEngine>>,
    id: i64,
) -> Result<(), String> {
    // 读取条目信息
    let entry = {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        queries::get_entry_by_id(&conn, id)
            .map_err(|e| e.to_string())?
            .ok_or("条目不存在".to_string())?
    };

    match entry.content_type.as_str() {
        "image" => {
            let raw = {
                let conn = db.conn.lock().map_err(|e| e.to_string())?;
                queries::get_entry_raw_content(&conn, id)
                    .map_err(|e| e.to_string())?
                    .ok_or("图片原始数据不存在".to_string())?
            };
            writer::write_image_to_clipboard(&raw, &dedup)
                .map_err(|e| e.to_string())?;
            log::info!("已写入图片到剪贴板: id={}", id);
        }
        _ => {
            writer::write_text_to_clipboard(&entry.text_preview, &dedup)
                .map_err(|e| e.to_string())?;
            log::info!("已写入文本到剪贴板: id={}", id);
        }
    }

    Ok(())
}

/// 获取条目的原始二进制内容（图片返回 PNG 字节，文本返回 UTF-8 字节）
#[tauri::command]
pub fn get_raw_content(
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<Vec<u8>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let raw = queries::get_entry_raw_content(&conn, id)
        .map_err(|e| e.to_string())?
        .ok_or("条目不存在或无原始内容".to_string())?;
    log::info!("获取原始内容: id={}, size={}", id, raw.len());
    Ok(raw)
}

/// 清空历史（非收藏条目）
#[tauri::command]
pub fn clear_history(
    db: State<'_, Arc<Database>>,
) -> Result<usize, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let count = queries::clear_unpinned(&conn)
        .map_err(|e| e.to_string())?;
    log::info!("已清空 {} 条历史", count);
    Ok(count)
}

/// 删除单条记录
#[tauri::command]
pub fn delete_entry(
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    queries::soft_delete_entry(&conn, id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 切换收藏状态
#[tauri::command]
pub fn toggle_pin(
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<bool, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let new_state = queries::toggle_pin(&conn, id)
        .map_err(|e| e.to_string())?;
    Ok(new_state)
}

/// 获取单个条目的元数据（不含原始内容）
#[tauri::command]
pub fn get_entry_by_id(
    db: State<'_, Arc<Database>>,
    id: i64,
) -> Result<ClipboardEntrySummary, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let entry = queries::get_entry_by_id(&conn, id)
        .map_err(|e| e.to_string())?
        .ok_or("条目不存在".to_string())?;
    Ok(ClipboardEntrySummary::from(entry))
}
