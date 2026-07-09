<script lang="ts">
  import { selectedFolderId, folderDataVersion } from "../stores/ui";
  import { listFolders, createFolder, renameFolder, deleteFolder } from "../stores/folders";
  import type { Folder, FolderWithEntryCount } from "../types";

  let folders = $state<FolderWithEntryCount[]>([]);
  let showCreateInput = $state(false);
  let createName = $state("");
  let contextMenuFolder = $state<Folder | null>(null);
  let contextMenuPos = $state({ x: 0, y: 0 });
  let renamingFolder = $state<Folder | null>(null);
  let renameName = $state("");

  /** 加载文件夹列表 */
  async function load() {
    folders = await listFolders();
  }

  // 监听 folderDataVersion 变化时刷新（例如从 FolderAssignPopover 修改后）
  // 也会在首次挂载时自动加载
  $effect(() => {
    $folderDataVersion;
    load();
  });

  /** 选择文件夹 */
  function select(folderId: number | null) {
    selectedFolderId.set(folderId);
  }

  /** 新建文件夹 */
  async function handleCreate() {
    const name = createName.trim();
    if (!name) return;
    const folder = await createFolder(name);
    if (folder) {
      await load();
      select(folder.id);
    }
    createName = "";
    showCreateInput = false;
  }

  /** 右键菜单 */
  function handleContextMenu(e: MouseEvent, folder: Folder) {
    if (folder.is_default) return; // 默认收藏夹不可删除
    e.preventDefault();
    contextMenuFolder = folder;
    contextMenuPos = { x: e.clientX, y: e.clientY };
  }

  function closeContextMenu() {
    contextMenuFolder = null;
  }

  /** 重命名 */
  function startRename(folder: Folder) {
    renamingFolder = folder;
    renameName = folder.name;
    contextMenuFolder = null;
  }

  async function confirmRename() {
    const name = renameName.trim();
    if (!name || !renamingFolder) return;
    await renameFolder(renamingFolder.id, name);
    renamingFolder = null;
    await load();
  }

  /** 删除 */
  async function handleDelete() {
    if (!contextMenuFolder) return;
    const folderId = contextMenuFolder.id;
    closeContextMenu();
    const ok = await deleteFolder(folderId);
    if (ok) {
      // 如果删除的是当前选中的文件夹，切换到全部
      let currentId: number | null = null;
      const unsub = selectedFolderId.subscribe(v => currentId = v)();
      if (currentId === folderId) {
        selectedFolderId.set(null);
      }
      await load();
    }
  }
</script>

<div class="folder-bar">
  <div class="folder-bar-inner">
    <!-- "全部" 芯片 -->
    <button
      class="folder-chip"
      class:active={$selectedFolderId === null}
      onclick={() => select(null)}
    >
      全部
    </button>

    <!-- 文件夹芯片列表 -->
    {#each folders as folder (folder.id)}
      {#if renamingFolder?.id === folder.id}
        <div class="rename-input-wrap">
          <input
            type="text"
            class="rename-input"
            bind:value={renameName}
            onkeydown={(e) => { if (e.key === "Enter") confirmRename(); if (e.key === "Escape") renamingFolder = null; }}
            onblur={confirmRename}
            autofocus
          />
        </div>
      {:else}
        <button
          class="folder-chip"
          class:active={$selectedFolderId === folder.id}
          onclick={() => select(folder.id)}
          oncontextmenu={(e) => handleContextMenu(e, folder)}
        >
          {folder.name}
          <span class="folder-count">{folder.entry_count}</span>
        </button>
      {/if}
    {/each}

    <!-- 新建按钮/输入框 -->
    {#if showCreateInput}
      <div class="create-input-wrap">
        <input
          type="text"
          class="create-input"
          placeholder="文件夹名称"
          bind:value={createName}
          onkeydown={(e) => { if (e.key === "Enter") handleCreate(); if (e.key === "Escape") { showCreateInput = false; createName = ""; } }}
          onblur={handleCreate}
          autofocus
        />
      </div>
    {:else}
      <button class="folder-chip add-btn" onclick={() => showCreateInput = true}>
        +
      </button>
    {/if}
  </div>
</div>

<!-- 右键上下文菜单 -->
{#if contextMenuFolder}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="context-overlay" onclick={closeContextMenu} oncontextmenu={(e) => e.preventDefault()}></div>
  <div
    class="context-menu"
    style="left: {contextMenuPos.x}px; top: {contextMenuPos.y}px;"
  >
    <button class="context-item" onclick={() => startRename(contextMenuFolder!)}>
      ✏️ 重命名
    </button>
    <button class="context-item danger" onclick={handleDelete}>
      🗑️ 删除
    </button>
  </div>
{/if}

<style>
  .folder-bar {
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
    padding: 4px 8px;
  }

  .folder-bar-inner {
    display: flex;
    gap: 4px;
    align-items: center;
    overflow-x: auto;
    white-space: nowrap;
    scrollbar-width: thin;
  }

  .folder-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 10px;
    border-radius: 12px;
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    flex-shrink: 0;
    transition: all 0.12s;
  }

  .folder-chip:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .folder-chip.active {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }

  .folder-count {
    font-size: 10px;
    opacity: 0.7;
  }

  .add-btn {
    font-size: 14px;
    font-weight: 600;
    padding: 3px 8px;
  }

  .create-input-wrap,
  .rename-input-wrap {
    display: inline-flex;
    flex-shrink: 0;
  }

  .create-input,
  .rename-input {
    width: 100px;
    padding: 3px 8px;
    border-radius: 12px;
    border: 1px solid var(--accent);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 12px;
    outline: none;
  }

  .context-overlay {
    position: fixed;
    inset: 0;
    z-index: 999;
  }

  .context-menu {
    position: fixed;
    z-index: 1000;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.15);
    padding: 4px;
    min-width: 120px;
  }

  .context-item {
    display: block;
    width: 100%;
    padding: 6px 12px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    border-radius: 4px;
  }

  .context-item:hover {
    background: var(--bg-hover);
  }

  .context-item.danger {
    color: #e74c3c;
  }
</style>
