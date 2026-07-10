use rusqlite::{params, Connection, Result};

use super::models::{ClipboardEntry, ContentType, Folder, FolderWithEntryCount};

/// 插入新条目（如果哈希冲突则更新 last_accessed）
pub fn insert_or_update_entry(
    conn: &Connection,
    hash: &str,
    text_preview: &str,
    content_type: &ContentType,
    raw_content: Option<&[u8]>,
    content_size: i64,
    source_app: &str,
    source_class: &str,
) -> Result<Option<i64>> {
    // 先尝试按哈希查找
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM clipboard_entries WHERE content_hash = ?1 AND is_deleted = 0",
            params![hash],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        // 已存在 -> 更新 last_accessed（移至顶部）
        // 返回 None 表示非新条目，主循环不会发射 new-clip 事件（避免剪贴板风暴）
        conn.execute(
            "UPDATE clipboard_entries SET last_accessed = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
            params![id],
        )?;
        return Ok(None);
    }

    // 不存在 -> 插入新记录（RFC 3339 格式确保 JS 可解析）
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    conn.execute(
        "INSERT INTO clipboard_entries (content_hash, text_preview, content_type, raw_content, content_size, source_app, source_class, captured_at, last_accessed)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![
            hash,
            text_preview,
            content_type.as_str(),
            raw_content,
            content_size,
            source_app,
            source_class,
            now,
        ],
    )?;
    Ok(Some(conn.last_insert_rowid()))
}

/// 分页获取历史记录（倒序）
pub fn get_entries(
    conn: &Connection,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClipboardEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, content_hash, text_preview, content_type, content_size,
                source_app, source_class, captured_at, last_accessed, is_pinned, is_deleted
         FROM clipboard_entries
         WHERE is_deleted = 0
         ORDER BY last_accessed DESC, id DESC
         LIMIT ?1 OFFSET ?2",
    )?;

    let rows = stmt.query_map(params![limit, offset], |row| {
        Ok(ClipboardEntry {
            id: row.get(0)?,
            content_hash: row.get(1)?,
            text_preview: row.get(2)?,
            content_type: row.get(3)?,
            content_size: row.get(4)?,
            source_app: row.get(5)?,
            source_class: row.get(6)?,
            captured_at: row.get(7)?,
            last_accessed: row.get(8)?,
            is_pinned: row.get::<_, i32>(9)? != 0,
            is_deleted: row.get::<_, i32>(10)? != 0,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// 搜索历史（LIKE 模糊匹配 + FTS）
pub fn search_entries(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> Result<Vec<ClipboardEntry>> {
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT id, content_hash, text_preview, content_type, content_size,
                source_app, source_class, captured_at, last_accessed, is_pinned, is_deleted
         FROM clipboard_entries
         WHERE is_deleted = 0 AND text_preview LIKE ?1
         ORDER BY last_accessed DESC, id DESC
         LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![pattern, limit], |row| {
        Ok(ClipboardEntry {
            id: row.get(0)?,
            content_hash: row.get(1)?,
            text_preview: row.get(2)?,
            content_type: row.get(3)?,
            content_size: row.get(4)?,
            source_app: row.get(5)?,
            source_class: row.get(6)?,
            captured_at: row.get(7)?,
            last_accessed: row.get(8)?,
            is_pinned: row.get::<_, i32>(9)? != 0,
            is_deleted: row.get::<_, i32>(10)? != 0,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// 获取单个条目（含 raw_content）
pub fn get_entry_by_id(conn: &Connection, id: i64) -> Result<Option<ClipboardEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, content_hash, text_preview, content_type, content_size,
                source_app, source_class, captured_at, last_accessed, is_pinned, is_deleted
         FROM clipboard_entries
         WHERE id = ?1 AND is_deleted = 0",
    )?;

    let mut rows = stmt.query_map(params![id], |row| {
        Ok(ClipboardEntry {
            id: row.get(0)?,
            content_hash: row.get(1)?,
            text_preview: row.get(2)?,
            content_type: row.get(3)?,
            content_size: row.get(4)?,
            source_app: row.get(5)?,
            source_class: row.get(6)?,
            captured_at: row.get(7)?,
            last_accessed: row.get(8)?,
            is_pinned: row.get::<_, i32>(9)? != 0,
            is_deleted: row.get::<_, i32>(10)? != 0,
        })
    })?;

    match rows.next() {
        Some(Ok(entry)) => Ok(Some(entry)),
        _ => Ok(None),
    }
}

/// 获取条目的原始内容（BLOB）
pub fn get_entry_raw_content(conn: &Connection, id: i64) -> Result<Option<Vec<u8>>> {
    let mut stmt = conn.prepare(
        "SELECT raw_content FROM clipboard_entries WHERE id = ?1 AND is_deleted = 0",
    )?;

    let mut rows = stmt.query_map(params![id], |row| row.get::<_, Vec<u8>>(0))?;
    match rows.next() {
        Some(Ok(data)) => Ok(Some(data)),
        _ => Ok(None),
    }
}

/// 切换收藏状态（同时自动更新默认收藏夹）
pub fn toggle_pin(conn: &Connection, id: i64) -> Result<bool> {
    conn.execute_batch("BEGIN")?;

    // 使用闭包封装操作，确保 ? 运算符能正确传播错误
    let result = (|| -> Result<bool> {
        conn.execute(
            "UPDATE clipboard_entries SET is_pinned = CASE WHEN is_pinned = 0 THEN 1 ELSE 0 END WHERE id = ?1",
            params![id],
        )?;
        let pinned: bool = conn
            .query_row(
                "SELECT is_pinned FROM clipboard_entries WHERE id = ?1",
                params![id],
                |row| row.get::<_, i32>(0).map(|v| v != 0),
            )
            .unwrap_or(false);

        // 同步默认收藏夹（同一事务内）
        sync_pinned_to_default_folder(conn, id, pinned)?;
        Ok(pinned)
    })();

    match result {
        Ok(pinned) => {
            conn.execute_batch("COMMIT")?;
            Ok(pinned)
        }
        Err(e) => {
            // 回滚事务，忽略回滚本身的错误
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

// ========== 文件夹相关函数 ==========

/// 同步 pinned 状态到默认收藏夹
/// 先获取默认文件夹 ID，避免子查询返回 NULL 导致 INSERT OR IGNORE 静默失败
fn sync_pinned_to_default_folder(conn: &Connection, entry_id: i64, is_pinned: bool) -> Result<()> {
    // 先获取默认收藏夹 ID（独立查询确保值有效）
    let folder_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM folders WHERE is_default = 1 LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;

    let folder_id = match folder_id {
        Some(id) => id,
        None => {
            log::error!(
                "[sync_pinned_to_default_folder] 默认收藏夹不存在！entry_id={}, is_pinned={}",
                entry_id,
                is_pinned
            );
            return Ok(());
        }
    };

    if is_pinned {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        conn.execute(
            "INSERT OR IGNORE INTO folder_entries (folder_id, entry_id, added_at) VALUES (?1, ?2, ?3)",
            params![folder_id, entry_id, now],
        )?;
        log::info!(
            "[sync_pinned_to_default_folder] 已添加 entry_id={} 到 folder_id={}",
            entry_id,
            folder_id
        );
    } else {
        conn.execute(
            "DELETE FROM folder_entries WHERE folder_id = ?1 AND entry_id = ?2",
            params![folder_id, entry_id],
        )?;
        log::info!(
            "[sync_pinned_to_default_folder] 已从 folder_id={} 移除 entry_id={}",
            folder_id,
            entry_id
        );
    }
    Ok(())
}

/// 创建文件夹
pub fn create_folder(conn: &Connection, name: &str) -> Result<Folder> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    // 计算新的 sort_order（末尾）
    let max_order: i64 = conn
        .query_row("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM folders", [], |row| row.get(0))
        .unwrap_or(0);

    conn.execute(
        "INSERT INTO folders (name, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
        params![name, max_order, now, now],
    )?;

    let id = conn.last_insert_rowid();
    Ok(Folder {
        id,
        name: name.to_string(),
        is_default: false,
        sort_order: max_order,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// 重命名文件夹
pub fn rename_folder(conn: &Connection, id: i64, new_name: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    conn.execute(
        "UPDATE folders SET name = ?1, updated_at = ?2 WHERE id = ?3",
        params![new_name, now, id],
    )?;
    Ok(())
}

/// 删除自定义文件夹（禁止删除默认收藏夹）
pub fn delete_folder(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM folders WHERE id = ?1 AND is_default = 0",
        params![id],
    )?;
    Ok(())
}

/// 列出所有文件夹（含条目计数，排除软删除条目）
pub fn list_folders(conn: &Connection) -> Result<Vec<FolderWithEntryCount>> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.name, f.is_default, f.sort_order, f.created_at, f.updated_at,
                COUNT(e.id) AS entry_count
         FROM folders f
         LEFT JOIN folder_entries fe ON fe.folder_id = f.id
         LEFT JOIN clipboard_entries e ON fe.entry_id = e.id AND e.is_deleted = 0
         GROUP BY f.id
         ORDER BY f.sort_order ASC, f.id ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(FolderWithEntryCount {
            id: row.get(0)?,
            name: row.get(1)?,
            is_default: row.get::<_, i32>(2)? != 0,
            sort_order: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            entry_count: row.get(6)?,
        })
    })?;

    let mut folders = Vec::new();
    for row in rows {
        folders.push(row?);
    }
    Ok(folders)
}

/// 添加条目到文件夹
pub fn add_entry_to_folder(conn: &Connection, folder_id: i64, entry_id: i64) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    conn.execute(
        "INSERT OR IGNORE INTO folder_entries (folder_id, entry_id, added_at) VALUES (?1, ?2, ?3)",
        params![folder_id, entry_id, now],
    )?;
    Ok(())
}

/// 从文件夹移除条目
pub fn remove_entry_from_folder(conn: &Connection, folder_id: i64, entry_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM folder_entries WHERE folder_id = ?1 AND entry_id = ?2",
        params![folder_id, entry_id],
    )?;
    Ok(())
}

/// 分页获取文件夹内的条目
pub fn get_folder_entries(
    conn: &Connection,
    folder_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClipboardEntry>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.content_hash, e.text_preview, e.content_type, e.content_size,
                e.source_app, e.source_class, e.captured_at, e.last_accessed, e.is_pinned, e.is_deleted
         FROM clipboard_entries e
         INNER JOIN folder_entries fe ON fe.entry_id = e.id
         WHERE fe.folder_id = ?1 AND e.is_deleted = 0
         ORDER BY fe.added_at DESC, e.id DESC
         LIMIT ?2 OFFSET ?3",
    )?;

    let rows = stmt.query_map(params![folder_id, limit, offset], |row| {
        Ok(ClipboardEntry {
            id: row.get(0)?,
            content_hash: row.get(1)?,
            text_preview: row.get(2)?,
            content_type: row.get(3)?,
            content_size: row.get(4)?,
            source_app: row.get(5)?,
            source_class: row.get(6)?,
            captured_at: row.get(7)?,
            last_accessed: row.get(8)?,
            is_pinned: row.get::<_, i32>(9)? != 0,
            is_deleted: row.get::<_, i32>(10)? != 0,
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

/// 获取条目所属的所有文件夹ID
pub fn get_entry_folders(conn: &Connection, entry_id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT folder_id FROM folder_entries WHERE entry_id = ?1",
    )?;

    let rows = stmt.query_map(params![entry_id], |row| row.get::<_, i64>(0))?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

/// 软删除单条（同时清理文件夹关联）
pub fn soft_delete_entry(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clipboard_entries SET is_deleted = 1 WHERE id = ?1",
        params![id],
    )?;
    // 清理文件夹关联（软删除不会触发 ON DELETE CASCADE）
    conn.execute(
        "DELETE FROM folder_entries WHERE entry_id = ?1",
        params![id],
    )?;
    Ok(())
}

/// 清空所有非收藏条目（同时清理文件夹关联）
pub fn clear_unpinned(conn: &Connection) -> Result<usize> {
    // 先记录将要被软删除的 ID
    let mut stmt = conn.prepare(
        "SELECT id FROM clipboard_entries WHERE is_pinned = 0 AND is_deleted = 0",
    )?;
    let ids: Vec<i64> = stmt.query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let count = conn.execute(
        "UPDATE clipboard_entries SET is_deleted = 1 WHERE is_pinned = 0 AND is_deleted = 0",
        [],
    )?;

    // 清理这些条目的文件夹关联（软删除不会触发 ON DELETE CASCADE）
    if count > 0 {
        for chunk in ids.chunks(100) {
            let placeholders: Vec<String> = chunk.iter().map(|_| "?".to_string()).collect();
            let sql = format!(
                "DELETE FROM folder_entries WHERE entry_id IN ({})",
                placeholders.join(",")
            );
            let mut stmt = conn.prepare(&sql)?;
            let params: Vec<&dyn rusqlite::types::ToSql> =
                chunk.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
            stmt.execute(params.as_slice())?;
        }
    }

    Ok(count)
}

/// 清理超出上限的旧条目（保留收藏项）
pub fn cleanup_old_entries(conn: &Connection, max_count: i64) -> Result<usize> {
    // 查询当前非收藏条目数
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM clipboard_entries WHERE is_deleted = 0 AND is_pinned = 0",
        [],
        |row| row.get(0),
    )?;

    if count <= max_count {
        return Ok(0);
    }

    let to_delete = count - max_count;
    // 删除最早的 N 条非收藏条目
    conn.execute(
        "DELETE FROM clipboard_entries WHERE id IN (
            SELECT id FROM clipboard_entries
            WHERE is_deleted = 0 AND is_pinned = 0
            ORDER BY last_accessed ASC
            LIMIT ?1
        )",
        params![to_delete],
    )?;
    Ok(to_delete as usize)
}

/// 清空指定文件夹的所有条目（保留条目本身，只删除关联）
///
/// 如果是默认收藏夹，还会同时取消条目的收藏状态（is_pinned = 0），
/// 避免在"全部"视图中仍显示星标。
pub fn clear_folder_entries(conn: &Connection, folder_id: i64) -> Result<usize> {
    // 先检查是否默认收藏夹
    let is_default: bool = conn
        .query_row(
            "SELECT is_default FROM folders WHERE id = ?1",
            params![folder_id],
            |row| row.get::<_, i32>(0).map(|v| v != 0),
        )
        .unwrap_or(false);

    // 如果是默认收藏夹，先取出条目 ID（删掉关联后查不到了）
    let entry_ids: Vec<i64> = if is_default {
        let mut stmt = conn.prepare(
            "SELECT entry_id FROM folder_entries WHERE folder_id = ?1",
        )?;
        let ids: Vec<i64> = stmt.query_map(params![folder_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        ids
    } else {
        vec![]
    };

    // 删除所有关联
    let count = conn.execute(
        "DELETE FROM folder_entries WHERE folder_id = ?1",
        params![folder_id],
    )?;

    // 默认收藏夹：同时取消这些条目的收藏状态
    if is_default && !entry_ids.is_empty() {
        for entry_id in &entry_ids {
            conn.execute(
                "UPDATE clipboard_entries SET is_pinned = 0 WHERE id = ?1 AND is_pinned = 1",
                params![entry_id],
            )?;
        }
        log::info!(
            "[clear_folder_entries] 默认收藏夹: 已取消 {} 条条目的收藏",
            entry_ids.len()
        );
    }

    Ok(count)
}

// 为 Result 提供 optional() 辅助方法
trait Optional<T> {
    fn optional(self) -> Result<Option<T>>;
}

impl<T> Optional<T> for Result<T> {
    fn optional(self) -> Result<Option<T>> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
