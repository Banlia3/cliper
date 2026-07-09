import { writable } from "svelte/store";

/** 面板是否可见 */
export const panelVisible = writable<boolean>(false);

/** 搜索关键词 */
export const searchQuery = writable<string>("");

/** 当前主题 */
export const theme = writable<"light" | "dark" | "system">("system");

/** 是否正在加载 */
export const isLoading = writable<boolean>(false);

/** 错误信息 */
export const errorMessage = writable<string | null>(null);

/** 当前选中的文件夹ID（null = 全部） */
export const selectedFolderId = writable<number | null>(null);

/** 文件夹数据版本号 — 自增后触发 FolderBar 刷新计数 */
export const folderDataVersion = writable<number>(0);
