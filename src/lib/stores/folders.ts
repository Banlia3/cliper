import { invoke } from "@tauri-apps/api/core";
import type { ClipboardEntry, Folder, FolderWithEntryCount } from "../types";

/** 列出所有文件夹（含条目计数） */
export async function listFolders(): Promise<FolderWithEntryCount[]> {
  try {
    return await invoke<FolderWithEntryCount[]>("list_folders");
  } catch (err) {
    console.error("加载文件夹列表失败:", err);
    return [];
  }
}

/** 创建文件夹 */
export async function createFolder(name: string): Promise<Folder | null> {
  try {
    return await invoke<Folder>("create_folder", { name });
  } catch (err) {
    console.error("创建文件夹失败:", err);
    return null;
  }
}

/** 重命名文件夹 */
export async function renameFolder(id: number, name: string): Promise<boolean> {
  try {
    await invoke("rename_folder", { id, name });
    return true;
  } catch (err) {
    console.error("重命名文件夹失败:", err);
    return false;
  }
}

/** 删除自定义文件夹 */
export async function deleteFolder(id: number): Promise<boolean> {
  try {
    await invoke("delete_folder", { id });
    return true;
  } catch (err) {
    console.error("删除文件夹失败:", err);
    return false;
  }
}

/** 添加条目到文件夹 */
export async function addToFolder(folderId: number, entryId: number): Promise<boolean> {
  try {
    await invoke("add_to_folder", { folderId, entryId });
    return true;
  } catch (err) {
    console.error("添加到文件夹失败:", err);
    return false;
  }
}

/** 从文件夹移除条目 */
export async function removeFromFolder(folderId: number, entryId: number): Promise<boolean> {
  try {
    await invoke("remove_from_folder", { folderId, entryId });
    return true;
  } catch (err) {
    console.error("从文件夹移除失败:", err);
    return false;
  }
}

/** 获取文件夹内条目（分页） */
export async function getFolderEntries(
  folderId: number,
  offset: number,
  limit: number = 30
): Promise<ClipboardEntry[]> {
  try {
    return await invoke<ClipboardEntry[]>("get_folder_entries", { folderId, limit, offset });
  } catch (err) {
    console.error("加载文件夹内容失败:", err);
    return [];
  }
}

/** 获取条目所属的文件夹ID列表 */
export async function getEntryFolders(entryId: number): Promise<number[]> {
  try {
    return await invoke<number[]>("get_entry_folders", { entryId });
  } catch (err) {
    console.error("获取条目所属文件夹失败:", err);
    return [];
  }
}

/** 清空指定文件夹的所有条目（只删除关联，不删除条目本身） */
export async function clearFolderEntries(folderId: number): Promise<boolean> {
  try {
    await invoke("clear_folder", { folderId });
    return true;
  } catch (err) {
    console.error("清空文件夹内容失败:", err);
    return false;
  }
}
