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

    // 创建默认收藏夹
    conn.execute(
        "INSERT OR IGNORE INTO folders (name, is_default, sort_order) VALUES ('收藏夹', 1, 0);",
        [],
    )?;

    // 回填已有 pinned 条目到默认收藏夹
    conn.execute(
        "INSERT OR IGNORE INTO folder_entries (folder_id, entry_id, added_at)
         SELECT (SELECT id FROM folders WHERE is_default = 1), id, captured_at
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
