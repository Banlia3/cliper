<script lang="ts">
  import { onMount } from "svelte";
  import { listFolders, addToFolder, removeFromFolder, getEntryFolders } from "../stores/folders";
  import type { FolderWithEntryCount } from "../types";

  interface Props {
    entryId: number;
    onClose: () => void;
  }

  let { entryId, onClose }: Props = $props();

  let folders = $state<FolderWithEntryCount[]>([]);
  let selectedFolderIds = $state<Set<number>>(new Set());
  let loading = $state(true);

  onMount(async () => {
    // 并行加载文件夹列表和条目归属
    const [folderList, entryFolderIds] = await Promise.all([
      listFolders(),
      getEntryFolders(entryId),
    ]);
    folders = folderList;
    selectedFolderIds = new Set(entryFolderIds);
    loading = false;
  });

  async function toggleFolder(folderId: number) {
    if (selectedFolderIds.has(folderId)) {
      selectedFolderIds.delete(folderId);
      await removeFromFolder(folderId, entryId);
    } else {
      selectedFolderIds.add(folderId);
      await addToFolder(folderId, entryId);
    }
    // 触发重新渲染
    selectedFolderIds = new Set(selectedFolderIds);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="popover-overlay" onclick={onClose}></div>
<div class="popover">
  <div class="popover-header">
    <span class="popover-title">分配到文件夹</span>
    <button class="popover-close" onclick={onClose}>✕</button>
  </div>
  <div class="popover-body">
    {#if loading}
      <div class="popover-loading">加载中...</div>
    {:else if folders.length === 0}
      <div class="popover-empty">暂无文件夹</div>
    {:else}
      {#each folders as folder (folder.id)}
        <label class="folder-option" class:checked={selectedFolderIds.has(folder.id)}>
          <input
            type="checkbox"
            checked={selectedFolderIds.has(folder.id)}
            onchange={() => toggleFolder(folder.id)}
          />
          <span class="folder-option-name">{folder.name}</span>
          <span class="folder-option-count">{folder.entry_count}</span>
        </label>
      {/each}
    {/if}
  </div>
</div>

<style>
  .popover-overlay {
    position: fixed;
    inset: 0;
    z-index: 998;
  }

  .popover {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 999;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 10px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.2);
    width: 220px;
    max-height: 280px;
    display: flex;
    flex-direction: column;
  }

  .popover-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px 6px;
  }

  .popover-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .popover-close {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 14px;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .popover-close:hover {
    background: var(--bg-hover);
  }

  .popover-body {
    padding: 4px 8px 8px;
    overflow-y: auto;
    flex: 1;
    max-height: 220px;
  }

  .popover-loading,
  .popover-empty {
    text-align: center;
    padding: 20px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .folder-option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.1s;
  }

  .folder-option:hover {
    background: var(--bg-hover);
  }

  .folder-option input[type="checkbox"] {
    accent-color: var(--accent);
  }

  .folder-option-name {
    flex: 1;
    font-size: 13px;
    color: var(--text-primary);
  }

  .folder-option-count {
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
