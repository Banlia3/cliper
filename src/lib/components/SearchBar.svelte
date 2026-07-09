<script lang="ts">
  import { searchQuery } from "../stores/ui";

  let inputValue = $state("");
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  /** 输入处理：防抖 150ms */
  function onInput(e: Event) {
    const target = e.target as HTMLInputElement;
    inputValue = target.value;

    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      searchQuery.set(inputValue);
    }, 150);
  }

  /** 清空搜索 */
  function clearSearch() {
    inputValue = "";
    searchQuery.set("");
  }

  /** 快捷键：Escape 清空搜索 */
  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      clearSearch();
    }
  }
</script>

<div class="search-bar">
  <span class="search-icon">🔍</span>
  <input
    type="text"
    placeholder="搜索剪贴板历史..."
    value={inputValue}
    oninput={onInput}
    onkeydown={onKeydown}
    class="search-input"
    autofocus
  />
  {#if inputValue}
    <button class="clear-btn" onclick={clearSearch}>✕</button>
  {/if}
</div>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border-radius: 8px;
    margin: 8px;
    gap: 8px;
  }

  .search-icon {
    font-size: 14px;
    opacity: 0.6;
  }

  .search-input {
    flex: 1;
    border: none;
    outline: none;
    background: transparent;
    font-size: 14px;
    color: var(--text-primary);
    font-family: inherit;
  }

  .search-input::placeholder {
    color: var(--text-secondary);
  }

  .clear-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 2px 6px;
    font-size: 14px;
    border-radius: 4px;
  }

  .clear-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
