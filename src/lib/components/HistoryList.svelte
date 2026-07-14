<script lang="ts">
  import HistoryItem from "./HistoryItem.svelte";
  import type { ClipboardEntry } from "../types";
  import { loadHistory, searchHistory, copyToClipboard, deleteEntry, togglePin, clearHistory } from "../stores/history";
  import { getFolderEntries, clearFolderEntries } from "../stores/folders";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { searchQuery, panelVisible, selectedFolderId, folderDataVersion } from "../stores/ui";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let entries = $state<ClipboardEntry[]>([]);
  let offset = $state(0);
  let hasMore = $state(true);
  let loading = $state(false);
  let searchResults = $state<ClipboardEntry[] | null>(null);
  let unlisten: UnlistenFn | undefined = $state();
  let unlistenFocus: UnlistenFn | undefined = $state();
  let unlistenHistoryCleared: UnlistenFn | undefined = $state();

  // 文件夹视图状态
  let folderEntries = $state<ClipboardEntry[] | null>(null);
  let folderOffset = $state(0);
  let folderHasMore = $state(true);
  let lastFolderLoadKey = $state<string | null>(null); // 防 $effect 双重触发

  const PAGE_SIZE = 30;

  /** 合并新加载的条目到现有列表（按 id 去重，保留顺序） */
  function mergeEntries(current: ClipboardEntry[], newEntries: ClipboardEntry[]): ClipboardEntry[] {
    const seen = new Set(current.map(e => e.id));
    const merged = [...current];
    for (const entry of newEntries) {
      if (!seen.has(entry.id)) {
        merged.push(entry);
        seen.add(entry.id);
      }
    }
    // 按 last_accessed DESC 排序
    merged.sort((a, b) => b.last_accessed.localeCompare(a.last_accessed));
    return merged;
  }

  /** 首次加载 + 监听新事件 + 窗口聚焦时刷新 */
  onMount(async () => {
    await loadMore();

    // 监听后端推送的新剪贴板事件
    unlisten = await listen<{ id: number; text_preview: string; content_type: string }>("new-clip", (event) => {
      const { id, text_preview, content_type } = event.payload;
      // 如果当前没有搜索，把新条目插入到列表顶部
      if (!searchResults) {
        // 检查是否已存在（防重复）
        if (!entries.some(e => e.id === id)) {
          entries = [
            {
              id,
              text_preview,
              content_type: content_type as ClipboardEntry["content_type"],
              content_hash: "",
              content_size: 0,
              source_app: "",
              captured_at: new Date().toISOString(),
              last_accessed: new Date().toISOString(),
              is_pinned: false,
            },
            ...entries,
          ];
        }
      }
    });

    // 窗口聚焦时刷新历史（弥补隐藏期间可能丢失的 new-clip 事件）
    const appWindow = getCurrentWindow();
    unlistenFocus = await appWindow.onFocusChanged(async ({ payload: focused }) => {
      if (!focused) return;        // 只有获得焦点时才刷新
      if (searchResults) return;   // 搜索模式下不自动刷新
      const freshEntries = await loadHistory(0, PAGE_SIZE);
      if (freshEntries.length > 0) {
        entries = mergeEntries(entries, freshEntries);
      }
    });

    // 监听托盘"清空历史"事件（只清除非收藏条目，收藏夹数据不受影响）
    unlistenHistoryCleared = await listen("history-cleared", () => {
      searchResults = null;
      // "全部"视图：重置并重新加载（保留收藏条目）
      offset = 0;
      hasMore = true;
      entries = [];
      // 如果当前在全部视图，重新加载
      if ($selectedFolderId === null) {
        loadMore();
      }
      // 触发文件夹计数和内容的刷新
      folderDataVersion.update(v => v + 1);
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (unlistenFocus) unlistenFocus();
    if (unlistenHistoryCleared) unlistenHistoryCleared();
  });

  /** 加载文件夹内容 */
  async function loadFolderEntries(folderId: number, reset: boolean) {
    console.log(`[loadFolderEntries] 开始 loading=${loading} folderId=${folderId} reset=${reset} offset=${folderOffset}`);
    loading = true;

    if (reset) {
      console.log(`[loadFolderEntries] 重置 folderEntries 为 []`);
      folderOffset = 0;
      folderHasMore = true;
      folderEntries = [];
    }

    const newEntries = await getFolderEntries(folderId, folderOffset, PAGE_SIZE);
    console.log(`[loadFolderEntries] getFolderEntries 返回 ${newEntries.length} 条`, JSON.parse(JSON.stringify(newEntries)));

    // 用户可能已切换视图，丢弃过期结果（防竞态）
    if ($selectedFolderId !== folderId) {
      console.log(`[loadFolderEntries] 视图已切换(${$selectedFolderId} !== ${folderId})，丢弃`);
      loading = false;
      return;
    }

    folderEntries = [...(folderEntries ?? []), ...newEntries];
    console.log(`[loadFolderEntries] 合并后 folderEntries 共 ${folderEntries.length} 条`);
    folderOffset += newEntries.length;
    folderHasMore = newEntries.length === PAGE_SIZE;
    loading = false;
  }

  /** 加载更多（分页） */
  async function loadMore() {
    if (loading) return;
    loading = true;

    const folderId = $selectedFolderId;
    if (folderId !== null) {
      if (!folderHasMore) { loading = false; return; }
      const newEntries = await getFolderEntries(folderId, folderOffset, PAGE_SIZE);
      if ($selectedFolderId !== folderId) { loading = false; return; } // 已切换视图，丢弃
      folderEntries = [...(folderEntries ?? []), ...newEntries];
      folderOffset += newEntries.length;
      folderHasMore = newEntries.length === PAGE_SIZE;
      loading = false;
      return;
    }

    if (!hasMore) { loading = false; return; }
    const newEntries = await loadHistory(offset, PAGE_SIZE);
    entries = [...entries, ...newEntries];
    offset += newEntries.length;
    hasMore = newEntries.length === PAGE_SIZE;
    loading = false;
  }

  /** 文件夹选择监听（带防重入 + 响应 folderDataVersion 变化） */
  $effect(() => {
    const folderId = $selectedFolderId;
    const version = $folderDataVersion;
    const key = folderId === null ? null : `${folderId}-${version}`;
    console.log(`[effect] selectedFolderId=${folderId}, version=${version}, lastKey=${lastFolderLoadKey}, newKey=${key}`);
    if (folderId === null) {
      folderEntries = null;
      lastFolderLoadKey = null;
    } else if (key !== lastFolderLoadKey) {
      lastFolderLoadKey = key;
      loadFolderEntries(folderId, true);
    } else {
      console.log(`[effect] 跳过重复触发 folderId=${folderId}`);
    }
  });

  /** 搜索监听（SearchBar 已做防抖，此处即时响应） */
  $effect(() => {
    const query = $searchQuery;
    if (!query) {
      searchResults = null;
      return;
    }

    searchHistory(query).then(results => {
      searchResults = results;
    });
  });

  /** 复制到剪贴板 */
  async function handleSelect(id: number) {
    const success = await copyToClipboard(id);
    if (success) {
      panelVisible.set(false);  // 关闭面板
    }
  }

  /** 删除 */
  async function handleDelete(id: number) {
    const success = await deleteEntry(id);
    if (success) {
      entries = entries.filter((e) => e.id !== id);
      if (searchResults) {
        searchResults = searchResults.filter((e) => e.id !== id);
      }
      if (folderEntries !== null) {
        folderEntries = folderEntries.filter(e => e.id !== id);
      }
      folderDataVersion.update(v => v + 1);
    }
  }

  /** 收藏切换 */
  async function handlePin(id: number) {
    const newState = await togglePin(id);
    // 更新本地状态
    const updateList = (list: ClipboardEntry[]) =>
      list.map((e) => (e.id === id ? { ...e, is_pinned: newState } : e));
    entries = updateList(entries);
    if (searchResults) {
      searchResults = updateList(searchResults);
    }
    // 如果当前在文件夹视图且取消了收藏 → 从 folderEntries 中移除该条目
    if (!newState && folderEntries !== null) {
      folderEntries = folderEntries.filter(e => e.id !== id);
      // 更新文件夹计数（通知 FolderBar 刷新）
      folderDataVersion.update(v => v + 1);
    } else {
      // 刷新文件夹计数和收藏夹列表
      folderDataVersion.update(v => v + 1);
    }
  }

  /** 滚动到底部时加载更多 */
  function onScroll(e: Event) {
    const target = e.target as HTMLElement;
    if (target.scrollHeight - target.scrollTop - target.clientHeight < 100) {
      loadMore();
    }
  }

  /** 清空当前视图（两步确认）：全部视图→清空所有，文件夹视图→清空文件夹 */
  async function handleClearCurrentView() {
    const folderId = $selectedFolderId;

    // 第一步确认
    const step1 = await confirm("确认全部删除吗？", { title: "清空", kind: "warning" });
    if (!step1) return;

    // 第二步确认
    const step2 = await confirm("真的确认吗？", { title: "确认", kind: "warning" });
    if (!step2) return;

    if (folderId !== null) {
      // 文件夹视图：清空当前文件夹（只删关联，保留条目本身）
      const ok = await clearFolderEntries(folderId);
      if (ok) {
        folderEntries = [];
        folderHasMore = false;
        folderDataVersion.update(v => v + 1);
        // 重新加载主列表，确保 is_pinned 状态与后端同步
        const fresh = await loadHistory(0, PAGE_SIZE);
        if (fresh.length > 0) {
          entries = fresh;
          offset = fresh.length;
          hasMore = fresh.length === PAGE_SIZE;
        }
      }
    } else {
      // 全部视图：清空所有非收藏条目
      const ok = await clearHistory();
      if (ok) {
        entries = [];
        searchResults = null;
        offset = 0;
        hasMore = false;
        folderDataVersion.update(v => v + 1);
      }
    }
  }

  const displayEntries = $derived.by(() => {
    const result = searchResults ?? folderEntries ?? entries;
    console.log(`[displayEntries] searchResults=${searchResults?.length ?? null} folderEntries=${folderEntries?.length ?? null} entries=${entries.length} → ${result.length} 条`);
    return result;
  });

  /** 诊断：追踪 folderEntries 每次变化 */
  $effect(() => {
    const len = folderEntries?.length;
    console.log(`[folderEntries 变化] ${folderEntries === null ? 'null' : len + ' 条'}`);

    // 延迟检查最终状态
    if (folderEntries !== null) {
      setTimeout(() => {
        console.log(`[延迟500ms] folderEntries=${folderEntries?.length ?? 'null'}`);
      }, 500);
    }
  });
</script>

<div class="history-list" onscroll={onScroll}>
  {#if displayEntries.length === 0}
    <div class="empty-state">
      <div class="empty-icon">📋</div>
      <div class="empty-text">
        {folderEntries !== null ? "此文件夹为空" : searchResults ? "没有匹配的结果" : "暂无剪贴板历史"}
      </div>
      {#if !searchResults && folderEntries === null}
        <div class="empty-hint">复制任意内容后将自动显示在这里</div>
      {/if}
    </div>
  {:else}
    <div class="clear-all-row">
      <button class="clear-all-btn" onclick={handleClearCurrentView}>
        🗑️ 全部清除
      </button>
    </div>
    {#each displayEntries as entry (entry.id)}
      <HistoryItem
        {entry}
        onPin={handlePin}
        onDelete={handleDelete}
      />
    {/each}
    {#if loading}
      <div class="loading-indicator">加载中...</div>
    {/if}
  {/if}
</div>

<style>
  .history-list {
    flex: 1;
    overflow-y: auto;
    padding-bottom: 8px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: var(--text-secondary);
  }

  .empty-icon {
    font-size: 48px;
    margin-bottom: 16px;
    opacity: 0.5;
  }

  .empty-text {
    font-size: 16px;
    font-weight: 500;
  }

  .empty-hint {
    font-size: 12px;
    margin-top: 8px;
    opacity: 0.7;
  }

  .loading-indicator {
    text-align: center;
    padding: 12px;
    color: var(--text-secondary);
    font-size: 12px;
  }

  .clear-all-row {
    display: flex;
    justify-content: flex-end;
    padding: 4px 12px;
    border-bottom: 1px solid var(--border-color, #eee);
    flex-shrink: 0;
  }

  .clear-all-btn {
    background: none;
    border: none;
    color: var(--text-danger, #e74c3c);
    font-size: 11px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
    opacity: 0.6;
    transition: opacity 0.15s;
  }

  .clear-all-btn:hover {
    opacity: 1;
    background: var(--bg-hover, rgba(231, 76, 60, 0.08));
  }
</style>
