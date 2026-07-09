use rusqlite::Connection;

/// 初始化数据库表结构
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    // 开启 WAL 模式提升并发读取性能
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA auto_vacuum=INCREMENTAL;")?;

    // 剪贴板条目表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_entries (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            content_hash    TEXT NOT NULL UNIQUE,
            text_preview    TEXT NOT NULL DEFAULT '',
            content_type    TEXT NOT NULL DEFAULT 'text'
                            CHECK(content_type IN ('text','image','files','html','rtf','other')),
            raw_content     BLOB,
            content_size    INTEGER NOT NULL DEFAULT 0,
            source_app      TEXT NOT NULL DEFAULT '',
            source_class    TEXT NOT NULL DEFAULT '',
            captured_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            last_accessed   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            is_pinned       INTEGER NOT NULL DEFAULT 0,
            is_deleted      INTEGER NOT NULL DEFAULT 0
        );",
        [],
    )?;

    // 索引
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_entries_captured_at
            ON clipboard_entries(captured_at DESC) WHERE is_deleted = 0;
         CREATE INDEX IF NOT EXISTS idx_entries_text_preview
            ON clipboard_entries(text_preview) WHERE is_deleted = 0;
         CREATE INDEX IF NOT EXISTS idx_entries_content_hash
            ON clipboard_entries(content_hash);
         CREATE INDEX IF NOT EXISTS idx_entries_cleanup
            ON clipboard_entries(captured_at ASC) WHERE is_deleted = 0 AND is_pinned = 0;",
    )?;

    // 设置表（键值存储）
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key         TEXT PRIMARY KEY,
            value       TEXT NOT NULL,
            updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );",
        [],
    )?;

    // 文件夹表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS folders (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            is_default  INTEGER NOT NULL DEFAULT 0,
            sort_order  INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );",
        [],
    )?;

    // 文件夹-条目关联表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS folder_entries (
            folder_id   INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
            entry_id    INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
            added_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            PRIMARY KEY (folder_id, entry_id)
        );",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_folder_entries_entry ON folder_entries(entry_id);",
        [],
    )?;

    // 唯一索引：确保只有一个默认收藏夹
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_folders_unique_default
         ON folders(is_default) WHERE is_default = 1;",
        [],
    )?;

    // 清理重复的默认收藏夹（修复第一次迁移的 bug：没有 UNIQUE 约束导致重复插入）
    conn.execute_batch(
        "DELETE FROM folder_entries WHERE folder_id IN (
            SELECT id FROM folders WHERE is_default = 1
            AND id NOT IN (SELECT MIN(id) FROM folders WHERE is_default = 1)
        );
        DELETE FROM folders WHERE is_default = 1
        AND id NOT IN (SELECT MIN(id) FROM folders WHERE is_default = 1);",
    )?;

    // 创建默认收藏夹（仅在不存在时插入）
    conn.execute(
        "INSERT INTO folders (name, is_default, sort_order)
         SELECT '收藏夹', 1, 0
         WHERE NOT EXISTS (SELECT 1 FROM folders WHERE is_default = 1);",
        [],
    )?;

    // 回填已有 pinned 条目到默认收藏夹（使用 MIN(id) 防多行）
    conn.execute(
        "INSERT OR IGNORE INTO folder_entries (folder_id, entry_id, added_at)
         SELECT (SELECT MIN(id) FROM folders WHERE is_default = 1), id, captured_at
         FROM clipboard_entries WHERE is_pinned = 1 AND is_deleted = 0;",
        [],
    )?;

    // 插入默认设置（如果不存在）
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('hotkey_modifiers', 'Ctrl+Shift')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('hotkey_key', 'V')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('max_history', '500')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('theme', 'system')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('autostart', 'false')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('clear_on_exit', 'false')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES ('max_item_size_mb', '10')",
        [],
    )?;

    Ok(())
}
