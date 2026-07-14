<script lang="ts">
  import type { ClipboardEntry } from "../types";
  import { getEntryById, getEntryContent, createImageUrl } from "../stores/history";
  import { formatRelativeTime, getContentTypeLabel, formatSize } from "../utils";
  import { onMount } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  interface Props {
    entryId: number;
  }

  let { entryId }: Props = $props();

  let entry = $state<ClipboardEntry | null>(null);
  let fullText = $state<string | null>(null);
  let imageUrl = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    // 加载条目元数据
    const meta = await getEntryById(entryId);
    if (!meta) {
      error = "条目不存在或已被删除";
      loading = false;
      return;
    }
    entry = meta;

    // 加载完整内容
    if (meta.content_type === "image") {
      const data = await getEntryContent(entryId);
      if (data && data.length > 0) {
        imageUrl = createImageUrl(data);
      }
    } else {
      // 文本/HTML/RTF 等类型：加载原始内容
      const data = await getEntryContent(entryId);
      if (data && data.length > 0) {
        // 尝试以 UTF-8 解码
        try {
          fullText = new TextDecoder("utf-8", { fatal: true }).decode(data);
        } catch {
          // 非 UTF-8 文本则显示预览
          fullText = null;
        }
      } else {
        // 无原始内容时 fallback 到 text_preview
        fullText = meta.text_preview || "(空)";
      }
    }

    loading = false;
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      closeWindow();
    }
  }

  async function closeWindow() {
    try {
      const win = getCurrentWebviewWindow();
      await win.close();
    } catch (err) {
      console.error("关闭窗口失败:", err);
    }
  }

  async function handleCopyContent() {
    if (!entry) return;
    const { copyToClipboard } = await import("../stores/history");
    await copyToClipboard(entry.id);
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="detail-view">
  <!-- 顶部栏 -->
  <div class="detail-header">
    <span class="detail-title">查看详情</span>
    <button class="close-btn" onclick={closeWindow}>✕</button>
  </div>

  {#if loading}
    <div class="detail-loading">加载中...</div>
  {:else if error}
    <div class="detail-error">{error}</div>
  {:else if entry}
    <!-- 元数据信息 -->
    <div class="detail-meta">
      <span class="meta-badge type">{getContentTypeLabel(entry.content_type)}</span>
      {#if entry.source_app}
        <span class="meta-item">来源: {entry.source_app}</span>
      {/if}
      <span class="meta-item">大小: {formatSize(entry.content_size)}</span>
      <span class="meta-item">捕获: {formatRelativeTime(entry.captured_at)}</span>
      <span class="meta-item">访问: {formatRelativeTime(entry.last_accessed)}</span>
      {#if entry.is_pinned}
        <span class="meta-badge pinned">已收藏</span>
      {/if}
    </div>

    <!-- 工具栏 -->
    <div class="detail-toolbar">
      <button class="toolbar-btn" onclick={handleCopyContent}>
        📋 复制内容
      </button>
    </div>

    <!-- 内容区 -->
    <div class="detail-body">
      {#if entry.content_type === "image" && imageUrl}
        <div class="detail-image-container">
          <img src={imageUrl} alt="图片详情" class="detail-image" />
        </div>
      {:else if fullText !== null}
        <pre class="detail-text">{fullText}</pre>
      {:else}
        <div class="detail-fallback">
          <p>此内容类型暂不支持完整预览</p>
          <p class="fallback-hint">{entry.text_preview}</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .detail-view {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary, #ffffff);
    color: var(--text-primary, #1a1a1a);
    overflow: hidden;
  }

  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: var(--bg-secondary, #f5f5f5);
    flex-shrink: 0;
  }

  .detail-title {
    font-size: 14px;
    font-weight: 600;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-secondary, #888);
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 14px;
  }

  .close-btn:hover {
    background: var(--bg-hover, #e8e8e8);
    color: var(--text-primary, #1a1a1a);
  }

  .detail-loading,
  .detail-error {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    font-size: 14px;
    color: var(--text-secondary, #888);
  }

  .detail-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-bottom: 1px solid var(--border-color, #eaeaea);
    font-size: 12px;
    color: var(--text-secondary, #888);
    flex-shrink: 0;
  }

  .meta-badge {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-weight: 500;
  }

  .meta-badge.type {
    background: var(--accent, #0078d4);
    color: #fff;
  }

  .meta-badge.pinned {
    background: #fff3cd;
    color: #856404;
  }

  .meta-item {
    white-space: nowrap;
  }

  .detail-toolbar {
    display: flex;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border-color, #eaeaea);
    flex-shrink: 0;
  }

  .toolbar-btn {
    background: var(--bg-secondary, #f5f5f5);
    border: 1px solid var(--border-color, #eaeaea);
    cursor: pointer;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 12px;
    color: var(--text-primary, #1a1a1a);
    transition: background 0.15s;
  }

  .toolbar-btn:hover {
    background: var(--bg-hover, #e8e8e8);
  }

  .detail-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  .detail-text {
    font-family: "SF Mono", "Cascadia Code", "Consolas", monospace;
    font-size: 13px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
    color: var(--text-primary, #1a1a1a);
  }

  .detail-image-container {
    display: flex;
    align-items: flex-start;
    justify-content: center;
  }

  .detail-image {
    max-width: 100%;
    max-height: 70vh;
    border-radius: 8px;
    object-fit: contain;
    box-shadow: 0 2px 12px rgba(0,0,0,0.1);
  }

  .detail-fallback {
    text-align: center;
    padding: 40px 20px;
    color: var(--text-secondary, #888);
  }

  .fallback-hint {
    margin-top: 12px;
    font-size: 12px;
    opacity: 0.7;
  }
</style>
