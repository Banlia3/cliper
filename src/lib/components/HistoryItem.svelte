<script lang="ts">
  import type { ClipboardEntry } from "../types";
  import { formatRelativeTime, getContentTypeIcon, truncateText } from "../utils";
  import { getEntryContent, createImageUrl } from "../stores/history";
  import { onDestroy } from "svelte";
  import FolderAssignPopover from "./FolderAssignPopover.svelte";

  interface Props {
    entry: ClipboardEntry;
    onSelect: (id: number) => void;
    onPin: (id: number) => void;
    onDelete: (id: number) => void;
  }

  let { entry, onSelect, onPin, onDelete }: Props = $props();
  let showFolderPopover = $state(false);

  let imageUrl = $state<string | null>(null);
  let imageLoading = $state(false);
  let imageError = $state(false);

  /** 如果是图片条目，懒加载原始数据并创建缩略图 */
  $effect(() => {
    if (entry.content_type === "image" && entry.image_data_url) {
      // 已缓存
      imageUrl = entry.image_data_url;
      imageLoading = false;
    } else if (entry.content_type === "image" && !imageLoading) {
      loadImage();
    }
  });

  async function loadImage() {
    imageLoading = true;
    imageError = false;
    const data = await getEntryContent(entry.id);
    if (data && data.length > 0) {
      imageUrl = createImageUrl(data);
      entry.image_data_url = imageUrl; // 缓存到条目上
    } else {
      imageError = true;
    }
    imageLoading = false;
  }

  function handleClick() {
    onSelect(entry.id);
  }

  function handlePin(e: MouseEvent) {
    e.stopPropagation();
    onPin(entry.id);
  }

  function handleDelete(e: MouseEvent) {
    e.stopPropagation();
    // 清理图片 URL
    if (imageUrl) {
      URL.revokeObjectURL(imageUrl);
    }
    onDelete(entry.id);
  }

  function handleFolderAssign(e: MouseEvent) {
    e.stopPropagation();
    showFolderPopover = true;
  }

  onDestroy(() => {
    // 组件销毁时释放 blob URL（仅未缓存的）
    if (imageUrl && !entry.image_data_url) {
      URL.revokeObjectURL(imageUrl);
    }
  });
</script>

<div class="history-item" onclick={handleClick} role="button" tabindex="0">
  <div class="item-icon">
    {#if entry.content_type === "image"}
      {#if imageLoading}
        <span class="image-loading">⏳</span>
      {:else if imageUrl}
        <img src={imageUrl} alt="缩略图" class="item-thumb" />
      {:else if imageError}
        🖼️
      {:else}
        🖼️
      {/if}
    {:else}
      {getContentTypeIcon(entry.content_type)}
    {/if}
  </div>

  <div class="item-content">
    {#if entry.content_type === "image" && imageUrl}
      <div class="item-image-preview">
        <img src={imageUrl} alt="图片预览" class="preview-img" />
      </div>
    {:else}
      <div class="item-preview">
        {truncateText(entry.text_preview || "(空)", 80)}
      </div>
    {/if}
    <div class="item-meta">
      <span class="item-time">{formatRelativeTime(entry.captured_at)}</span>
      {#if entry.source_app}
        <span class="item-source">· {entry.source_app}</span>
      {/if}
      {#if entry.is_pinned}
        <span class="item-pinned">📌</span>
      {/if}
    </div>
  </div>

  <div class="item-actions">
    <button class="action-btn pin-btn" onclick={handlePin} title={entry.is_pinned ? "取消置顶" : "置顶"}>
      {entry.is_pinned ? "★" : "☆"}
    </button>
    <button class="action-btn folder-btn" onclick={handleFolderAssign} title="分配到文件夹">
      📁
    </button>
    <button class="action-btn delete-btn" onclick={handleDelete} title="删除">🗑️</button>
  </div>
</div>

{#if showFolderPopover}
  <FolderAssignPopover
    entryId={entry.id}
    onClose={() => showFolderPopover = false}
  />
{/if}

<style>
  .history-item {
    display: flex;
    align-items: flex-start;
    padding: 10px 12px;
    cursor: pointer;
    border-bottom: 1px solid var(--border-color);
    transition: background 0.15s;
    gap: 10px;
  }

  .history-item:hover {
    background: var(--bg-hover);
  }

  .history-item:active {
    background: var(--bg-active);
  }

  .item-icon {
    font-size: 18px;
    flex-shrink: 0;
    width: 28px;
    text-align: center;
    padding-top: 2px;
  }

  .item-thumb {
    width: 28px;
    height: 28px;
    object-fit: cover;
    border-radius: 4px;
    display: block;
  }

  .image-loading {
    font-size: 14px;
  }

  .item-content {
    flex: 1;
    min-width: 0;
  }

  .item-preview {
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.4;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-image-preview {
    margin-bottom: 4px;
  }

  .preview-img {
    max-width: 100%;
    max-height: 120px;
    border-radius: 6px;
    object-fit: contain;
    background: var(--bg-secondary);
    display: block;
  }

  .item-meta {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 3px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .item-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s;
    flex-shrink: 0;
  }

  .history-item:hover .item-actions {
    opacity: 1;
  }

  .action-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 4px;
    font-size: 14px;
    color: var(--text-secondary);
  }

  .action-btn:hover {
    background: var(--bg-hover-secondary);
    color: var(--text-primary);
  }

  .pin-btn {
    font-size: 16px;
  }
</style>
