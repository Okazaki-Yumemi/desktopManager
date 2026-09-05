<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    FileSymlink,
    FileText,
    Folder,
    Plus,
    RotateCw,
    Search,
    X,
  } from "@lucide/svelte";
  import DesktopIcon from "../components/DesktopIcon.svelte";
  import {
    assignToCollection,
    createCollection,
    deleteCollection,
    getCollectionItems,
    getDesktopItems,
    listCollections,
    openDesktopItem,
    rescanDesktop,
    searchDesktopItems,
    unassignFromCollection,
  } from "../services/backend";
  import type { Collection, DesktopItem } from "../types/domain";
  import { pushToast } from "../stores/toast.svelte";
  import { formatDateShort } from "../lib/datetime";
  import { DESKTOP_CHANGED_EVENT } from "../lib/events";

  const PALETTE = ["#4f8cff", "#8b5cf6", "#22c55e", "#f59e0b", "#ef4444"];

  let items = $state<DesktopItem[]>([]);
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let query = $state("");
  let refreshing = $state(false);
  let selectedPath = $state<string | null>(null);

  let collections = $state<Collection[]>([]);
  let activeCollectionId = $state<number | null>(null);
  let creating = $state(false);
  let newName = $state("");
  let dropTargetId = $state<number | null>(null);

  const hasQuery = $derived(query.trim().length > 0);
  const activeCollection = $derived(
    collections.find((c) => c.id === activeCollectionId) ?? null,
  );

  async function loadCollections() {
    try {
      collections = await listCollections();
    } catch (err) {
      pushToast("error", `读取集合失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function load(q: string, collectionId: number | null = activeCollectionId) {
    loadError = null;
    try {
      if (collectionId !== null) {
        const all = await getCollectionItems(collectionId);
        const needle = q.trim().toLowerCase();
        items = needle
          ? all.filter((i) => i.displayName.toLowerCase().includes(needle))
          : all;
      } else {
        items = q.trim() ? await searchDesktopItems(q.trim()) : await getDesktopItems();
      }
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  async function reload() {
    await Promise.all([loadCollections(), load(query)]);
  }

  // Debounced, race-safe reload whenever the search box or the active
  // collection changes. Also runs once on mount for the initial list.
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let requestId = 0;
  $effect(() => {
    const q = query;
    const colId = activeCollectionId;
    const id = ++requestId;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      if (id === requestId) void load(q, colId);
    }, 250);
    return () => clearTimeout(searchTimer);
  });

  onMount(() => {
    void loadCollections();
    // The backend pushes desktop:changed only when the index really changed.
    const unlisten = listen(DESKTOP_CHANGED_EVENT, () => {
      void reload();
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  });

  async function refresh() {
    refreshing = true;
    try {
      await rescanDesktop();
      await reload();
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

  // --- Collections -------------------------------------------------------

  async function assign(collectionId: number, path: string) {
    try {
      const created = await assignToCollection(collectionId, path);
      if (created) pushToast("ok", "已加入集合");
      await reload();
    } catch (err) {
      pushToast("error", `加入集合失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function unassign(collectionId: number, path: string) {
    try {
      await unassignFromCollection(collectionId, path);
      pushToast("ok", "已从集合移出");
      await reload();
    } catch (err) {
      pushToast("error", `移出失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function submitCreate() {
    if (!creating) return;
    creating = false;
    const name = newName.trim();
    if (!name) return;
    try {
      const col = await createCollection(name, paletteFor(collections.length));
      newName = "";
      await loadCollections();
      activeCollectionId = col.id;
      pushToast("ok", `集合「${col.name}」已创建`);
    } catch (err) {
      pushToast("error", `创建失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function removeCollection(id: number) {
    try {
      await deleteCollection(id);
      if (activeCollectionId === id) activeCollectionId = null;
      await loadCollections();
      pushToast("ok", "集合已删除");
    } catch (err) {
      pushToast("error", `删除失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  function onDragStart(e: DragEvent, path: string) {
    e.dataTransfer?.setData("text/plain", path);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
  }

  function onChipDrop(e: DragEvent, collectionId: number) {
    e.preventDefault();
    dropTargetId = null;
    const path = e.dataTransfer?.getData("text/plain");
    if (path) void assign(collectionId, path);
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
  }

  function paletteFor(index: number): string {
    return PALETTE[index % PALETTE.length] ?? PALETTE[0]!;
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
        {:else if activeCollection}
          集合「{activeCollection.name}」· {items.length} 项
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

  <div class="chips" aria-label="集合筛选">
    <button
      type="button"
      class="chip"
      class:active={activeCollectionId === null}
      onclick={() => (activeCollectionId = null)}
    >
      全部
    </button>
    {#each collections as col (col.id)}
      <span class="chip-wrap">
        <button
          type="button"
          class="chip"
          class:active={activeCollectionId === col.id}
          class:drop={dropTargetId === col.id}
          title="点击筛选；把项目拖到这里加入集合"
          onclick={() => (activeCollectionId = col.id)}
          ondragover={(e) => e.preventDefault()}
          ondragenter={() => (dropTargetId = col.id)}
          ondragleave={() => (dropTargetId = null)}
          ondrop={(e) => onChipDrop(e, col.id)}
        >
          <span class="dot" style="background: {col.color}"></span>
          {col.name}
          <span class="count">{col.itemCount}</span>
        </button>
        {#if activeCollectionId === col.id}
          <button
            type="button"
            class="chip-del"
            title="删除集合"
            onclick={() => void removeCollection(col.id)}
          >
            <X size={12} />
          </button>
        {/if}
      </span>
    {/each}
    {#if activeCollectionId !== null}
      <span
        class="chip remove-chip"
        class:drop={dropTargetId === -1}
        title="把项目拖到这里从集合移出"
        role="button"
        tabindex="0"
        ondragover={(e) => e.preventDefault()}
        ondragenter={() => (dropTargetId = -1)}
        ondragleave={() => (dropTargetId = null)}
        ondrop={(e) => {
          e.preventDefault();
          dropTargetId = null;
          const path = e.dataTransfer?.getData("text/plain");
          if (path && activeCollectionId !== null) void unassign(activeCollectionId, path);
        }}
      >
        移出集合
      </span>
    {/if}
    {#if creating}
      <span class="chip create-chip">
        <input
          bind:value={newName}
          placeholder="集合名称，回车确认"
          maxlength="30"
          use:focusOnMount
          onkeydown={(e) => {
            if (e.key === "Enter") void submitCreate();
            if (e.key === "Escape") creating = false;
          }}
          onblur={() => void submitCreate()}
        />
      </span>
    {:else}
      <button
        type="button"
        class="chip add"
        onclick={() => {
          creating = true;
          newName = "";
        }}
      >
        <Plus size={13} />
        新建集合
      </button>
    {/if}
  </div>

  <div class="toolbar">
    <div class="search">
      <Search size={15} />
      <input
        type="search"
        placeholder={activeCollection ? `在「${activeCollection.name}」中搜索…` : "搜索桌面项目…"}
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
  {:else if items.length === 0 && activeCollection}
    <div class="state">
      <p class="muted">集合里还没有项目——把下面的项目拖到上方的「{activeCollection.name}」标签上</p>
    </div>
  {:else if items.length === 0}
    <div class="state"><p class="muted">桌面上没有可见项目</p></div>
  {:else}
    <ul class="grid" aria-label="桌面项目">
      {#each items as item (item.id)}
        <li>
          <button
            type="button"
            draggable="true"
            aria-pressed={selectedPath === item.path}
            class="item"
            class:selected={selectedPath === item.path}
            title={item.path}
            onclick={() => (selectedPath = item.path)}
            ondblclick={() => void open(item)}
            onkeydown={(e) => {
              if (e.key === "Enter") void open(item);
            }}
            ondragstart={(e) => onDragStart(e, item.path)}
          >
            <span class="icon kind-{item.kind}" aria-hidden="true">
              <DesktopIcon path={item.path} size={22}>
                {#snippet fallback()}
                  {#if item.kind === "folder"}
                    <Folder size={22} />
                  {:else if item.kind === "shortcut"}
                    <FileSymlink size={22} />
                  {:else}
                    <FileText size={22} />
                  {/if}
                {/snippet}
              </DesktopIcon>
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

  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }

  .chip-wrap {
    display: inline-flex;
    align-items: center;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-secondary);
    font-size: var(--font-size-s);
    cursor: pointer;
    transition: border-color var(--duration-fast) var(--ease-out),
      background var(--duration-fast) var(--ease-out), color var(--duration-fast) var(--ease-out);
  }

  .chip:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .chip.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--text-primary);
  }

  .chip.drop {
    border-style: dashed;
    border-color: var(--accent);
    color: var(--accent);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    flex-shrink: 0;
  }

  .count {
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .chip-del {
    display: inline-grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-left: -8px;
    border: none;
    border-radius: 999px;
    background: var(--surface-hover);
    color: var(--text-tertiary);
    cursor: pointer;
  }

  .chip-del:hover {
    color: var(--error);
  }

  .remove-chip {
    border-style: dashed;
    color: var(--text-tertiary);
    cursor: default;
  }

  .create-chip {
    padding: 2px 10px;
    cursor: default;
  }

  .create-chip input {
    border: none;
    background: transparent;
    outline: none;
    color: var(--text-primary);
    font-size: var(--font-size-s);
    width: 150px;
    padding: 3px 0;
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
