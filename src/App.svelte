<script lang="ts">
  import "./app.css";
  import SearchBar from "./lib/components/SearchBar.svelte";
  import HistoryList from "./lib/components/HistoryList.svelte";
  import FolderBar from "./lib/components/FolderBar.svelte";
  import { panelVisible, searchQuery } from "./lib/stores/ui";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { invoke } from "@tauri-apps/api/core";

  const appWindow = getCurrentWindow();

  /** 监听 Escape 键关闭面板，Ctrl+Shift+I/F12 打开 DevTools */
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      closePanel();
    }
    if ((e.key === "F12") || (e.ctrlKey && e.shiftKey && (e.key === "I" || e.key === "i"))) {
      e.preventDefault();
      invoke("open_devtools").catch((err) => console.error("打开 DevTools 失败:", err));
    }
  }

  /** 关闭面板 */
  function closePanel() {
    panelVisible.set(false);
    searchQuery.set("");
    // 通知后端隐藏窗口
    appWindow.hide().catch(() => {});
  }

  /** 鼠标按下标题栏时启动窗口拖拽 */
  function onTitleBarMouseDown(e: MouseEvent) {
    // 如果点击的是关闭按钮，不启动拖拽
    const target = e.target as HTMLElement;
    if (target.closest("button")) return;
    appWindow.startDragging().catch(() => {});
  }

  /** 启动窗口缩放（边框/角拖拽） */
  function onResizeStart(direction: string) {
    return (e: MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      appWindow.startResizeDragging(direction).catch(() => {});
    };
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="panel">
  <!-- 缩放边框：四边 -->
  {#each ["North","South","West","East","NorthWest","NorthEast","SouthWest","SouthEast"] as dir}
    <div class="resize-handle {dir.toLowerCase()}" role="presentation" onmousedown={onResizeStart(dir)}></div>
  {/each}

  <!-- 标题栏（拖拽区域） -->
  <div class="title-bar" role="toolbar" onmousedown={onTitleBarMouseDown}>
    <span class="title-text">剪贴板历史</span>
    <button class="close-btn" onclick={closePanel}>✕</button>
  </div>

  <!-- 文件夹栏 -->
  <FolderBar />

  <!-- 搜索栏 -->
  <SearchBar />

  <!-- 历史列表 -->
  <HistoryList />
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    overflow: hidden;
    position: relative;
  }

  .title-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: var(--bg-secondary);
    cursor: grab;
    user-select: none;
    flex-shrink: 0;
  }

  .title-bar:active {
    cursor: grabbing;
  }

  .title-text {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 14px;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  /* ===== 缩放手柄 ===== */
  .resize-handle {
    position: fixed;
    z-index: 100;
  }

  .resize-handle.top {
    top: 0; left: 4px; right: 4px;
    height: 4px;
    cursor: n-resize;
  }
  .resize-handle.bottom {
    bottom: 0; left: 4px; right: 4px;
    height: 4px;
    cursor: s-resize;
  }
  .resize-handle.left {
    left: 0; top: 4px; bottom: 4px;
    width: 4px;
    cursor: w-resize;
  }
  .resize-handle.right {
    right: 0; top: 4px; bottom: 4px;
    width: 4px;
    cursor: e-resize;
  }

  .resize-handle.top-left {
    top: 0; left: 0;
    width: 8px; height: 8px;
    cursor: nw-resize;
  }
  .resize-handle.top-right {
    top: 0; right: 0;
    width: 8px; height: 8px;
    cursor: ne-resize;
  }
  .resize-handle.bottom-left {
    bottom: 0; left: 0;
    width: 8px; height: 8px;
    cursor: sw-resize;
  }
  .resize-handle.bottom-right {
    bottom: 0; right: 0;
    width: 8px; height: 8px;
    cursor: se-resize;
  }
</style>
