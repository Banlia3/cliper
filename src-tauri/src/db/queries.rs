use rusqlite::{params, Connection, Result};

use super::models::{ClipboardEntry, ContentType};

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
        conn.execute(
            "UPDATE clipboard_entries SET last_accessed = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?1",
            params![id],
        )?;
        return Ok(Some(id));
    }

    // 不存在 -> 插入新记录
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%f").to_string() + "Z";
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

/// 切换收藏状态
pub fn toggle_pin(conn: &Connection, id: i64) -> Result<bool> {
    conn.execute(
        "UPDATE clipboard_entries SET is_pinned = CASE WHEN is_pinned = 0 THEN 1 ELSE 0 END WHERE id = ?1",
        params![id],
    )?;
    // 返回新状态
    let pinned: bool = conn
        .query_row(
            "SELECT is_pinned FROM clipboard_entries WHERE id = ?1",
            params![id],
            |row| row.get::<_, i32>(0).map(|v| v != 0),
        )
        .unwrap_or(false);
    Ok(pinned)
}

/// 软删除单条
pub fn soft_delete_entry(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE clipboard_entries SET is_deleted = 1 WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

/// 清空所有非收藏条目
pub fn clear_unpinned(conn: &Connection) -> Result<usize> {
    let count = conn.execute(
        "UPDATE clipboard_entries SET is_deleted = 1 WHERE is_pinned = 0 AND is_deleted = 0",
        [],
    )?;
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
