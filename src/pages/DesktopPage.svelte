<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { FileSymlink, FileText, Folder, RotateCw, Search } from "@lucide/svelte";
  import {
    getDesktopItems,
    openDesktopItem,
    rescanDesktop,
    searchDesktopItems,
  } from "../services/backend";
  import type { DesktopItem } from "../types/domain";
  import { pushToast } from "../stores/toast.svelte";
  import { formatDateShort } from "../lib/datetime";
  import { DESKTOP_CHANGED_EVENT } from "../lib/events";

  let items = $state<DesktopItem[]>([]);
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let query = $state("");
  let refreshing = $state(false);
  let selectedPath = $state<string | null>(null);

  const hasQuery = $derived(query.trim().length > 0);

  async function load(q: string) {
    loadError = null;
    try {
      items = q.trim() ? await searchDesktopItems(q.trim()) : await getDesktopItems();
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  // Debounced, race-safe reload whenever the search box changes. Also runs
  // once on mount for the initial list.
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let requestId = 0;
  $effect(() => {
    const q = query;
    const id = ++requestId;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      if (id === requestId) void load(q);
    }, 250);
    return () => clearTimeout(searchTimer);
  });

  onMount(() => {
    // The backend pushes desktop:changed only when the index really changed.
    const unlisten = listen(DESKTOP_CHANGED_EVENT, () => {
      void load(query);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  });

  async function refresh() {
    refreshing = true;
    try {
      await rescanDesktop();
      await load(query);
    } catch (err) {
      pushToast("error", `刷新失败：${String(err)}`);
    } finally {
      refreshing = false;
    }
  }

  async function open(item: DesktopItem) {
    try {
      await openDesktopItem(item.path);
    } catch (err) {
      pushToast("error", `无法打开：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  function subline(item: DesktopItem): string {
    const parts: string[] = [];
    if (item.ext) parts.push(item.ext.toUpperCase());
    if (item.sizeBytes != null) parts.push(formatSize(item.sizeBytes));
    if (item.modifiedAt != null) parts.push(formatDateShort(item.modifiedAt));
    return parts.join(" · ");
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(0)} KB`;
    if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
    return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
  }
</script>

<div class="desktop">
  <header class="head">
    <div>
      <h1>桌面</h1>
      <p class="muted">
        {#if loading}
          正在读取…
        {:else}
          共 {items.length} 项 · 元数据索引，不会移动你的文件
        {/if}
      </p>
    </div>
    <button type="button" class="refresh" onclick={() => void refresh()} disabled={refreshing}>
      <RotateCw size={15} class={refreshing ? "spin" : ""} />
      刷新
    </button>
  </header>

  <div class="toolbar">
    <div class="search">
      <Search size={15} />
      <input
        type="search"
        placeholder="搜索桌面项目…"
        bind:value={query}
        aria-label="搜索桌面项目"
      />
    </div>
  </div>

  {#if loadError}
    <div class="state">
      <p class="error-text">读取桌面索引失败：{loadError}</p>
      <button type="button" onclick={() => void load(query)}>重试</button>
    </div>
  {:else if loading}
    <div class="state"><p class="muted">正在加载桌面索引…</p></div>
  {:else if items.length === 0 && hasQuery}
    <div class="state"><p class="muted">没有找到与“{query.trim()}”匹配的项目</p></div>
  {:else if items.length === 0}
    <div class="state"><p class="muted">桌面上没有可见项目</p></div>
  {:else}
    <ul class="grid" aria-label="桌面项目">
      {#each items as item (item.id)}
        <li>
          <button
            type="button"
            aria-pressed={selectedPath === item.path}
            class="item"
            class:selected={selectedPath === item.path}
            title={item.path}
            onclick={() => (selectedPath = item.path)}
            ondblclick={() => void open(item)}
            onkeydown={(e) => {
              if (e.key === "Enter") void open(item);
            }}
          >
            <span class="icon kind-{item.kind}" aria-hidden="true">
              {#if item.kind === "folder"}
                <Folder size={22} />
              {:else if item.kind === "shortcut"}
                <FileSymlink size={22} />
              {:else}
                <FileText size={22} />
              {/if}
            </span>
            <span class="meta">
              <span class="name">{item.displayName}</span>
              <span class="sub">{subline(item)}</span>
            </span>
            {#if item.source === "public_desktop"}
              <span class="badge">公用</span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .desktop {
    max-width: 960px;
    margin: 0 auto;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }

  h1 {
    margin: 0 0 var(--space-1);
    font-size: var(--font-size-xl);
    font-weight: 600;
  }

  .muted {
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }

  .refresh {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .refresh:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .refresh:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .spin {
    animation: rotate 0.9s linear infinite;
  }

  @keyframes rotate {
    to {
      transform: rotate(360deg);
    }
  }

  .toolbar {
    margin-bottom: var(--space-4);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-tertiary);
    max-width: 420px;
  }

  .search:focus-within {
    border-color: var(--accent);
  }

  .search input {
    flex: 1;
    border: none;
    background: transparent;
    padding: 8px 0;
    color: var(--text-primary);
    outline: none;
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: var(--space-3);
  }

  .item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    text-align: left;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    cursor: pointer;
    transition: border-color var(--duration-fast) var(--ease-out),
      background var(--duration-fast) var(--ease-out);
  }

  .item:hover {
    background: var(--surface-hover);
  }

  .item.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .icon {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-s);
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .icon.kind-folder {
    color: var(--accent);
  }

  .icon.kind-shortcut {
    color: color-mix(in srgb, var(--accent) 60%, var(--text-secondary));
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .name {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .sub {
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge {
    margin-left: auto;
    flex-shrink: 0;
    font-size: var(--font-size-s);
    padding: 1px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--text-tertiary);
  }

  .state {
    padding: var(--space-6) 0;
    text-align: center;
  }

  .state button {
    margin-top: var(--space-3);
    padding: 6px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    cursor: pointer;
    color: var(--text-secondary);
  }

  .error-text {
    color: var(--error);
  }
</style>
