<script lang="ts">
  import HistoryItem from "./HistoryItem.svelte";
  import type { ClipboardEntry } from "../types";
  import { loadHistory, searchHistory, copyToClipboard, deleteEntry, togglePin, clearHistory } from "../stores/history";
  import { searchQuery, panelVisible } from "../stores/ui";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import type { UnlistenFn } from "@tauri-apps/api/event";

  let entries = $state<ClipboardEntry[]>([]);
  let offset = $state(0);
  let hasMore = $state(true);
  let loading = $state(false);
  let searchResults = $state<ClipboardEntry[] | null>(null);
  let unlisten: UnlistenFn | undefined = $state();

  const PAGE_SIZE = 30;

  /** 首次加载 + 监听新事件 */
  onMount(async () => {
    await loadMore();

    // 监听后端推送的新剪贴板事件
    unlisten = await listen<{ id: number; text_preview: string; content_type: string }>("new-clip", (event) => {
      const { id, text_preview, content_type } = event.payload;
      // 如果当前没有搜索，把新条目插入到列表顶部
      if (!searchResults) {
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
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  /** 加载更多（分页） */
  async function loadMore() {
    if (loading || !hasMore) return;
    loading = true;

    const newEntries = await loadHistory(offset, PAGE_SIZE);
    entries = [...entries, ...newEntries];
    offset += newEntries.length;
    hasMore = newEntries.length === PAGE_SIZE;
    loading = false;
  }

  /** 搜索监听 */
  $effect(() => {
    const query = $searchQuery;
    if (!query) {
      searchResults = null;
      return;
    }

    const timeout = setTimeout(async () => {
      const results = await searchHistory(query);
      searchResults = results;
    }, 200);

    return () => clearTimeout(timeout);
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
  }

  /** 滚动到底部时加载更多 */
  function onScroll(e: Event) {
    const target = e.target as HTMLElement;
    if (target.scrollHeight - target.scrollTop - target.clientHeight < 100) {
      loadMore();
    }
  }

  /** 清空所有 */
  async function handleClearAll() {
    const success = await clearHistory();
    if (success) {
      entries = [];
      searchResults = null;
      offset = 0;
      hasMore = false;
    }
  }

  const displayEntries = $derived(searchResults ?? entries);
</script>

<div class="history-list" onscroll={onScroll}>
  {#if displayEntries.length === 0}
    <div class="empty-state">
      <div class="empty-icon">📋</div>
      <div class="empty-text">
        {searchResults ? "没有匹配的结果" : "暂无剪贴板历史"}
      </div>
      {#if !searchResults}
        <div class="empty-hint">复制任意内容后将自动显示在这里</div>
      {/if}
    </div>
  {:else}
    {#each displayEntries as entry (entry.id)}
      <HistoryItem
        {entry}
        onSelect={handleSelect}
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
</style>
