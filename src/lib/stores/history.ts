import { invoke } from "@tauri-apps/api/core";
import type { ClipboardEntry } from "../types";

const PAGE_SIZE = 30;

/** 分页加载历史记录 */
export async function loadHistory(
  offset: number,
  limit: number = PAGE_SIZE
): Promise<ClipboardEntry[]> {
  try {
    const entries = await invoke<ClipboardEntry[]>("get_history", {
      limit,
      offset,
    });
    return entries;
  } catch (err) {
    console.error("加载历史失败:", err);
    return [];
  }
}

/** 搜索历史 */
export async function searchHistory(
  query: string
): Promise<ClipboardEntry[]> {
  if (!query.trim()) return [];

  try {
    const entries = await invoke<ClipboardEntry[]>("search_history", {
      query: query.trim(),
      limit: 100,
    });
    return entries;
  } catch (err) {
    console.error("搜索历史失败:", err);
    return [];
  }
}

/** 将指定条目复制到剪贴板 */
export async function copyToClipboard(id: number): Promise<boolean> {
  try {
    await invoke("copy_to_clipboard", { id });
    return true;
  } catch (err) {
    console.error("复制到剪贴板失败:", err);
    return false;
  }
}

/** 清空历史 */
export async function clearHistory(): Promise<boolean> {
  try {
    await invoke("clear_history");
    return true;
  } catch (err) {
    console.error("清空历史失败:", err);
    return false;
  }
}

/** 删除单条 */
export async function deleteEntry(id: number): Promise<boolean> {
  try {
    await invoke("delete_entry", { id });
    return true;
  } catch (err) {
    console.error("删除条目失败:", err);
    return false;
  }
}

/** 切换收藏状态 */
export async function togglePin(id: number): Promise<boolean> {
  try {
    const newState = await invoke<boolean>("toggle_pin", { id });
    return newState;
  } catch (err) {
    console.error("切换收藏失败:", err);
    return false;
  }
}

/** 获取条目的原始二进制内容（图片→PNG 字节，文本→UTF-8 字节） */
export async function getEntryContent(id: number): Promise<Uint8Array | null> {
  try {
    const data = await invoke<number[]>("get_raw_content", { id });
    return new Uint8Array(data);
  } catch (err) {
    console.error("获取原始内容失败:", err);
    return null;
  }
}

/** 从 Uint8Array 创建可释放的对象 URL */
export function createImageUrl(data: Uint8Array): string {
  const blob = new Blob([data], { type: "image/png" });
  return URL.createObjectURL(blob);
}
