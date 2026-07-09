use serde::{Deserialize, Serialize};

/// 剪贴板条目类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    Image,
    Files,
    Html,
    Rtf,
    Other,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Text => "text",
            ContentType::Image => "image",
            ContentType::Files => "files",
            ContentType::Html => "html",
            ContentType::Rtf => "rtf",
            ContentType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "text" => ContentType::Text,
            "image" => ContentType::Image,
            "files" => ContentType::Files,
            "html" => ContentType::Html,
            "rtf" => ContentType::Rtf,
            _ => ContentType::Other,
        }
    }
}

/// 一条剪贴板历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content_hash: String,
    pub text_preview: String,
    pub content_type: String,
    pub content_size: i64,
    pub source_app: String,
    pub source_class: String,
    pub captured_at: String,       // ISO 8601
    pub last_accessed: String,     // ISO 8601
    pub is_pinned: bool,
    pub is_deleted: bool,
    // raw_content 只在前端请求复制时返回
}

/// 用于 IPC 传输的轻量版（不含 raw_content）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntrySummary {
    pub id: i64,
    pub content_hash: String,
    pub text_preview: String,
    pub content_type: String,
    pub content_size: i64,
    pub source_app: String,
    pub captured_at: String,
    pub last_accessed: String,
    pub is_pinned: bool,
}

impl From<ClipboardEntry> for ClipboardEntrySummary {
    fn from(e: ClipboardEntry) -> Self {
        Self {
            id: e.id,
            content_hash: e.content_hash,
            text_preview: e.text_preview,
            content_type: e.content_type,
            content_size: e.content_size,
            source_app: e.source_app,
            captured_at: e.captured_at,
            last_accessed: e.last_accessed,
            is_pinned: e.is_pinned,
        }
    }
}

/// 收藏文件夹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: i64,
    pub name: String,
    pub is_default: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// 带条目计数的收藏文件夹（前端列表展示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderWithEntryCount {
    pub id: i64,
    pub name: String,
    pub is_default: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
    pub entry_count: i64,
}

/// 设置键值对
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsEntry {
    pub key: String,
    pub value: String,
}
