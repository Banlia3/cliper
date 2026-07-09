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
