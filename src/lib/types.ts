/** 剪贴板条目摘要（不含原始内容） */
export interface ClipboardEntry {
  id: number;
  content_hash: string;
  text_preview: string;
  content_type: "text" | "image" | "files" | "html" | "rtf" | "other";
  content_size: number;
  source_app: string;
  captured_at: string;
  last_accessed: string;
  is_pinned: boolean;
  /** 前端使用的图片数据 URL（仅 image 类型，懒加载后填充） */
  image_data_url?: string;
}

/** 来自后端的剪贴板事件 */
export interface NewClipEvent {
  id: number;
  text_preview: string;
  content_type: string;
}

/** 收藏文件夹 */
export interface Folder {
  id: number;
  name: string;
  is_default: boolean;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

/** 带条目计数的收藏文件夹 */
export interface FolderWithEntryCount extends Folder {
  entry_count: number;
}

/** 应用设置 */
export interface AppSettings {
  hotkey_modifiers: string;
  hotkey_key: string;
  max_history: string;
  theme: "light" | "dark" | "system";
  autostart: string;
  clear_on_exit: string;
  max_item_size_mb: string;
}
