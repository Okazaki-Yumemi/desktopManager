<script lang="ts">
  import { onMount } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import {
    ChevronRight,
    Eye,
    EyeOff,
    FileSymlink,
    FileText,
    Folder,
    FolderPlus,
    Layers,
    Pencil,
    Plus,
    RotateCw,
    Search,
    X,
  } from "@lucide/svelte";
  import DesktopIcon from "../components/DesktopIcon.svelte";
  import {
    assignExternalToCollection,
    browseChildren,
    createCollection,
    createScene,
    deleteCollection,
    deleteScene,
    getCollectionItems,
    getDesktopItems,
    getSceneVisibility,
    getSetting,
    listCollections,
    listScenes,
    openCollectionItem,
    openDesktopItem,
    renameCollection,
    rescanDesktop,
    searchDesktopItems,
    setSceneVisibility,
    setSetting,
    unassignFromCollection,
  } from "../services/backend";
  import type { Collection, DesktopItem, PathEntry, Scene } from "../types/domain";
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
  // Parent of the collection being created (null = top level).
  let createParentId = $state<number | null>(null);
  // Rename-in-place: the active chip turns into an input.
  let renamingId = $state<number | null>(null);
  let renameDraft = $state("");

  // Expanded folder reference: the trail of entered directories. An empty
  // trail means the collection/desktop root listing.
  type TrailStep = { path: string; label: string };
  let browseTrail = $state<TrailStep[]>([]);
  let browseItems = $state<DesktopItem[]>([]);
  let browseLoading = $state(false);

  // Scenes: a scene hides some collections from the chips row (pure
  // metadata — files are never touched). Collections without an explicit
  // row are visible.
  let scenes = $state<Scene[]>([]);
  let activeSceneId = $state<number | null>(null);
  let lastSceneId = $state<number | null>(null);
  // SvelteSet is self-reactive (mutations and clear() update derived state),
  // so it must NOT be wrapped in $state.
  let sceneHidden = new SvelteSet<number>();
  let creatingScene = $state(false);
  let newSceneName = $state("");

  // Pointer-based drag of an item card onto a collection chip (HTML5 DnD is
  // not usable here: with Tauri's native drop handler enabled it never fires).
  type DragState = { path: string; label: string; x: number; y: number; over: string | null };
  let drag = $state<DragState | null>(null);
  // True while files are dragged into the window from Explorer (Tauri event).
  let fileDropHover = $state(false);

  const hasQuery = $derived(query.trim().length > 0);
  const activeCollection = $derived(
    collections.find((c) => c.id === activeCollectionId) ?? null,
  );
  const visibleCollections = $derived(
    activeSceneId === null
      ? collections
      : collections.filter((c) => !sceneHidden.has(c.id)),
  );
  const hiddenCollections = $derived(
    activeSceneId === null ? [] : collections.filter((c) => sceneHidden.has(c.id)),
  );

  /** Leave any expanded folder when the collection filter changes. */
  let browseScope: number | null = null;
  $effect(() => {
    if (activeCollectionId !== browseScope) {
      browseScope = activeCollectionId;
      browseTrail = [];
      browseItems = [];
    }
  });

  // Depth-first over parent links so sub-collections render right after
  // (indented under) their parent. Scene-hidden collections drop out with
  // their unreferenced children; anything unreachable still shows flat.
  const orderedCollections = $derived.by(() => {
    const childrenOf: Record<string, Collection[]> = {};
    for (const c of visibleCollections) {
      const key = c.parentId === null ? "" : String(c.parentId);
      (childrenOf[key] ??= []).push(c);
    }
    const out: { col: Collection; depth: number }[] = [];
    const walk = (parentKey: string, depth: number) => {
      for (const col of childrenOf[parentKey] ?? []) {
        out.push({ col, depth });
        if (depth < 4) walk(String(col.id), depth + 1);
      }
    };
    walk("", 0);
    for (const col of visibleCollections) {
      if (!out.some((e) => e.col.id === col.id)) out.push({ col, depth: 0 });
    }
    return out;
  });

  // What the grid shows: the root listing, or the expanded folder's
  // children (client-side filtered by the same search box).
  const browsing = $derived(browseTrail.length > 0);
  const shownItems = $derived.by(() => {
    if (!browsing) return items;
    const needle = query.trim().toLowerCase();
    return needle
      ? browseItems.filter((i) => i.displayName.toLowerCase().includes(needle))
      : browseItems;
  });

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
    void initScenes();
    // The backend pushes desktop:changed only when the index really changed.
    const unlistenP = listen(DESKTOP_CHANGED_EVENT, () => {
      void reload();
    });
    // Files dragged in from Explorer carry real paths (Tauri native drop).
    const dropP = getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "enter" || payload.type === "over") {
        fileDropHover = true;
      } else if (payload.type === "leave") {
        fileDropHover = false;
      } else if (payload.type === "drop") {
        fileDropHover = false;
        void assignExternal(payload.paths);
      }
    });
    return () => {
      void unlistenP.then((fn) => fn());
      void dropP.then((fn) => fn());
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

  /** Enter one folder level of the current listing (expand a reference). */
  async function enterFolder(item: DesktopItem) {
    if (browseLoading) return;
    browseLoading = true;
    try {
      const children = await browseChildren(item.path);
      browseTrail = [...browseTrail, { path: item.path, label: item.displayName }];
      browseItems = children.map(toBrowseItem);
      selectedPath = null;
    } catch (err) {
      pushToast("error", `无法展开文件夹：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      browseLoading = false;
    }
  }

  /** Jump back to a trail level; -1 returns to the root listing. */
  async function jumpToLevel(level: number) {
    if (browseLoading) return;
    if (level < 0) {
      browseTrail = [];
      browseItems = [];
      return;
    }
    const step = browseTrail[level];
    if (!step) return;
    browseLoading = true;
    try {
      const children = await browseChildren(step.path);
      browseTrail = browseTrail.slice(0, level + 1);
      browseItems = children.map(toBrowseItem);
      selectedPath = null;
    } catch (err) {
      pushToast("error", `无法读取文件夹：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      browseLoading = false;
    }
  }

  /** Folder-listing entry → grid card model (external snapshot semantics). */
  function toBrowseItem(entry: PathEntry): DesktopItem {
    const isLink = entry.ext === "lnk" || entry.ext === "url";
    return {
      id: -1,
      path: entry.path,
      source: "external",
      displayName: entry.name,
      kind: entry.isDir ? "folder" : isLink ? "shortcut" : "file",
      ext: entry.ext,
      sizeBytes: entry.sizeBytes,
      modifiedAt: null,
      missing: false,
    };
  }

  async function open(item: DesktopItem) {
    if (item.path.startsWith("\0")) return; // truncated-listing marker
    try {
      if (item.source === "external") await openCollectionItem(item.path);
      else await openDesktopItem(item.path);
    } catch (err) {
      pushToast("error", `无法打开：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  // --- Collections -------------------------------------------------------

  async function assign(collectionId: number, path: string) {
    try {
      // assign_any on the backend: indexed paths stay live, everything
      // else (e.g. expanded-folder children) becomes a snapshot row.
      const created = await assignExternalToCollection(collectionId, path);
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

  async function assignExternal(paths: string[]) {
    if (paths.length === 0) return;
    if (activeCollectionId === null) {
      pushToast("info", "请先点击选中一个集合，再把项目拖进来");
      return;
    }
    let added = 0;
    for (const path of paths) {
      try {
        if (await assignExternalToCollection(activeCollectionId, path)) added += 1;
      } catch (err) {
        pushToast("error", `加入失败：${err instanceof Error ? err.message : String(err)}`);
      }
    }
    if (added > 0) {
      pushToast("ok", `已加入 ${added} 项到「${activeCollection?.name ?? "集合"}」`);
      await reload();
    }
  }

  function startCreating(parentId: number | null) {
    creating = true;
    createParentId = parentId;
    newName = "";
  }

  async function submitCreate() {
    if (!creating) return;
    creating = false;
    const name = newName.trim();
    const parentId = createParentId;
    createParentId = null;
    if (!name) return;
    try {
      const col = await createCollection(name, paletteFor(collections.length), parentId);
      newName = "";
      await loadCollections();
      activeCollectionId = col.id;
      pushToast("ok", `集合「${col.name}」已创建`);
    } catch (err) {
      pushToast("error", `创建失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  function startRename(col: Collection) {
    renamingId = col.id;
    renameDraft = col.name;
  }

  async function submitRename() {
    if (renamingId === null) return;
    const id = renamingId;
    renamingId = null;
    const name = renameDraft.trim();
    if (!name) return;
    try {
      await renameCollection(id, name);
      await loadCollections();
      pushToast("ok", "已重命名");
    } catch (err) {
      pushToast("error", `重命名失败：${err instanceof Error ? err.message : String(err)}`);
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

  // --- Scenes --------------------------------------------------------------

  async function loadScenes() {
    try {
      scenes = await listScenes();
    } catch (err) {
      pushToast("error", `读取场景失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function loadSceneVisibility() {
    sceneHidden.clear();
    if (activeSceneId === null) return;
    try {
      const rows = await getSceneVisibility(activeSceneId);
      for (const r of rows) {
        if (!r.visible) sceneHidden.add(r.collectionId);
      }
    } catch {
      // Treat a failed read as "everything visible"; the chips stay usable.
    }
  }

  async function persistActiveScene() {
    try {
      await setSetting("ui.activeScene", activeSceneId);
    } catch {
      // Persistence is best-effort; the UI state is already correct.
    }
  }

  /** Switch scenes; clicking the active scene restores the previous one. */
  async function applyScene(id: number | null) {
    if (activeSceneId === id) return;
    lastSceneId = activeSceneId;
    activeSceneId = id;
    void persistActiveScene();
    await loadSceneVisibility();
  }

  async function onSceneChip(id: number) {
    await applyScene(activeSceneId === id ? lastSceneId : id);
  }

  async function toggleCollectionVisible(collectionId: number) {
    if (activeSceneId === null) return;
    const wasHidden = sceneHidden.has(collectionId);
    try {
      await setSceneVisibility(activeSceneId, collectionId, wasHidden);
      if (wasHidden) sceneHidden.delete(collectionId);
      else sceneHidden.add(collectionId);
    } catch (err) {
      pushToast("error", `更新可见性失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function submitScene() {
    if (!creatingScene) return;
    creatingScene = false;
    const name = newSceneName.trim();
    if (!name) return;
    try {
      const scene = await createScene(name);
      newSceneName = "";
      await loadScenes();
      await applyScene(scene.id);
      pushToast("ok", `场景「${scene.name}」已创建`);
    } catch (err) {
      pushToast("error", `创建失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function removeScene(id: number) {
    try {
      await deleteScene(id);
      if (activeSceneId === id) await applyScene(null);
      await loadScenes();
      pushToast("ok", "场景已删除");
    } catch (err) {
      pushToast("error", `删除失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function initScenes() {
    await loadScenes();
    try {
      const saved = await getSetting<number | null>("ui.activeScene");
      if (saved !== null && scenes.some((s) => s.id === saved)) {
        activeSceneId = saved;
        await loadSceneVisibility();
      }
    } catch {
      // Fall back to 「全部」.
    }
  }

  // Pointer drag: threshold decides click vs drag; hit-test chips via
  // elementFromPoint so the ghost can stay pointer-events: none.
  function onItemPointerDown(e: PointerEvent, item: DesktopItem) {
    if (e.button !== 0) return;
    if (item.path.startsWith("\0")) return; // truncated-listing marker
    const startX = e.clientX;
    const startY = e.clientY;

    const onMove = (ev: PointerEvent) => {
      if (drag === null) {
        if (Math.hypot(ev.clientX - startX, ev.clientY - startY) < 6) return;
        document.body.classList.add("dragging");
        drag = { path: item.path, label: item.displayName, x: ev.clientX, y: ev.clientY, over: null };
      } else {
        drag.x = ev.clientX;
        drag.y = ev.clientY;
        const el = document.elementFromPoint(ev.clientX, ev.clientY);
        drag.over = el?.closest("[data-drop-id]")?.getAttribute("data-drop-id") ?? null;
      }
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      document.body.classList.remove("dragging");
      const { path, over } = drag ?? { path: null, over: null };
      drag = null;
      if (path === null || over === null) return;
      if (over === "remove") {
        if (activeCollectionId !== null) void unassign(activeCollectionId, path);
      } else {
        void assign(Number(over), path);
      }
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  function paletteFor(index: number): string {
    return PALETTE[index % PALETTE.length] ?? PALETTE[0]!;
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
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

<div class="desktop page-enter">
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

  <div class="chips scene-row" aria-label="场景切换">
    <Layers size={13} class="scene-ico" />
    <button
      type="button"
      class="chip"
      class:active={activeSceneId === null}
      onclick={() => void applyScene(null)}
    >
      全部
    </button>
    {#each scenes as sc (sc.id)}
      <span class="chip-wrap">
        <button
          type="button"
          class="chip"
          class:active={activeSceneId === sc.id}
          title="切换场景；再次点击可回到上一个场景"
          onclick={() => void onSceneChip(sc.id)}
        >
          {sc.name}
        </button>
        {#if activeSceneId === sc.id}
          <button
            type="button"
            class="chip-del"
            title="删除场景"
            onclick={() => void removeScene(sc.id)}
          >
            <X size={12} />
          </button>
        {/if}
      </span>
    {/each}
    {#if creatingScene}
      <span class="chip create-chip">
        <input
          bind:value={newSceneName}
          placeholder="场景名称，回车确认"
          maxlength="30"
          use:focusOnMount
          onkeydown={(e) => {
            if (e.key === "Enter") void submitScene();
            if (e.key === "Escape") creatingScene = false;
          }}
          onblur={() => void submitScene()}
        />
      </span>
    {:else}
      <button
        type="button"
        class="chip add"
        onclick={() => {
          creatingScene = true;
          newSceneName = "";
        }}
      >
        <Plus size={13} />
        新建场景
      </button>
    {/if}
  </div>

  <div class="chips" aria-label="集合筛选">
    <button
      type="button"
      class="chip"
      class:active={activeCollectionId === null}
      onclick={() => (activeCollectionId = null)}
    >
      全部
    </button>
    {#each orderedCollections as { col, depth } (col.id)}
      <span
        class="chip-wrap"
        style:margin-left={depth > 0 ? `${depth * 16}px` : undefined}
      >
        {#if renamingId === col.id}
          <span class="chip create-chip">
            <input
              bind:value={renameDraft}
              placeholder="新名称，回车确认"
              maxlength="30"
              aria-label="重命名集合"
              use:focusOnMount
              onkeydown={(e) => {
                if (e.key === "Enter") void submitRename();
                if (e.key === "Escape") renamingId = null;
              }}
              onblur={() => void submitRename()}
            />
          </span>
        {:else}
          {#if depth > 0}<span class="tree-elbow" aria-hidden="true">└</span>{/if}
          <button
            type="button"
            class="chip"
            class:active={activeCollectionId === col.id}
            class:drop={drag?.over === String(col.id)}
            title="点击筛选；把项目拖到这里加入集合"
            data-drop-id={String(col.id)}
            onclick={() => (activeCollectionId = col.id)}
          >
            <span class="dot" style="background: {col.color}"></span>
            {col.name}
            <span class="count">{col.itemCount}</span>
          </button>
          <!-- Hover reveals the actions; the active collection keeps them
               always visible so rename/sub-collection are discoverable. -->
          <span class="chip-actions" class:show={activeCollectionId === col.id}>
            {#if activeSceneId !== null}
              <button
                type="button"
                class="chip-del"
                title="在当前场景隐藏这个集合"
                onclick={() => void toggleCollectionVisible(col.id)}
              >
                <Eye size={12} />
              </button>
            {/if}
            <button
              type="button"
              class="chip-del"
              title="重命名集合"
              onclick={() => startRename(col)}
            >
              <Pencil size={12} />
            </button>
            <button
              type="button"
              class="chip-del"
              title="新建子集合"
              onclick={() => startCreating(col.id)}
            >
              <FolderPlus size={12} />
            </button>
            <button
              type="button"
              class="chip-del"
              title="删除集合（连同子集合）"
              onclick={() => void removeCollection(col.id)}
            >
              <X size={12} />
            </button>
          </span>
        {/if}
      </span>
    {/each}
    {#each hiddenCollections as col (col.id)}
      <span class="chip-wrap">
        <button
          type="button"
          class="chip dim"
          title="在当前场景中已隐藏"
          onclick={() => (activeCollectionId = col.id)}
        >
          {col.name}
          <span class="count">{col.itemCount}</span>
        </button>
        <button
          type="button"
          class="chip-del"
          title="在当前场景显示这个集合"
          onclick={() => void toggleCollectionVisible(col.id)}
        >
          <EyeOff size={12} />
        </button>
      </span>
    {/each}
    {#if activeCollectionId !== null}
      <span
        class="chip remove-chip"
        class:drop={drag?.over === "remove"}
        title="把项目拖到这里从集合移出"
        data-drop-id="remove"
      >
        移出集合
      </span>
    {/if}
    {#if creating}
      <span class="chip create-chip">
        <input
          bind:value={newName}
          placeholder={createParentId ? "子集合名称，回车确认" : "集合名称，回车确认"}
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
        onclick={() => startCreating(null)}
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

  {#if browsing}
    <nav class="crumbs" aria-label="文件夹路径">
      <button type="button" class="crumb" onclick={() => void jumpToLevel(-1)}>
        {activeCollection?.name ?? "桌面"}
      </button>
      {#each browseTrail as step, i (step.path)}
        <span class="crumb-sep" aria-hidden="true">/</span>
        {#if i === browseTrail.length - 1}
          <span class="crumb current">{step.label}</span>
        {:else}
          <button type="button" class="crumb" onclick={() => void jumpToLevel(i)}>
            {step.label}
          </button>
        {/if}
      {/each}
      {#if browseLoading}
        <span class="crumb-loading">读取中…</span>
      {/if}
    </nav>
  {/if}

  {#if loadError}
    <div class="state">
      <p class="error-text">读取桌面索引失败：{loadError}</p>
      <button type="button" onclick={() => void load(query)}>重试</button>
    </div>
  {:else if loading}
    <div class="state"><p class="muted">正在加载桌面索引…</p></div>
  {:else if browsing && shownItems.length === 0}
    <div class="state">
      <p class="muted">
        {hasQuery ? `没有找到与“${query.trim()}”匹配的项目` : "这个文件夹是空的"}
      </p>
    </div>
  {:else if shownItems.length === 0 && hasQuery}
    <div class="state"><p class="muted">没有找到与“{query.trim()}”匹配的项目</p></div>
  {:else if shownItems.length === 0 && activeCollection}
    <div class="state">
      <p class="muted">
        集合里还没有项目——把下面的项目拖到「{activeCollection.name}」上，或从资源管理器把快捷方式拖进窗口
      </p>
    </div>
  {:else if shownItems.length === 0}
    <div class="state"><p class="muted">桌面上没有可见项目</p></div>
  {:else}
    <ul class="grid" aria-label="桌面项目">
      {#each shownItems as item (item.path)}
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
            onpointerdown={(e) => onItemPointerDown(e, item)}
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
            {:else if item.source === "external"}
              <span class="badge">外部</span>
            {/if}
            {#if item.kind === "folder" && !item.path.startsWith("\0")}
              <span
                class="expand"
                role="button"
                tabindex="-1"
                title="展开文件夹"
                onclick={(e) => {
                  e.stopPropagation();
                  void enterFolder(item);
                }}
                onpointerdown={(e) => e.stopPropagation()}
                onkeydown={(e) => {
                  if (e.key === "Enter") {
                    e.stopPropagation();
                    void enterFolder(item);
                  }
                }}
              >
                <ChevronRight size={14} />
              </span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if drag}
  <div class="drag-ghost" style="left: {drag.x}px; top: {drag.y}px;">{drag.label}</div>
{/if}
{#if fileDropHover}
  <div class="drop-overlay">
    {activeCollection
      ? `松开，加入「${activeCollection.name}」`
      : "先点击选中一个集合，再拖入快捷方式 / 文件"}
  </div>
{/if}

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
    font-size: var(--font-size-2xl);
    font-weight: 600;
    letter-spacing: -0.01em;
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
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out),
      transform var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
  }

  .refresh:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
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
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-secondary);
    font-size: var(--font-size-s);
    cursor: pointer;
    transition: border-color var(--duration-fast) var(--ease-out),
      background var(--duration-fast) var(--ease-out), color var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
  }

  .tree-elbow {
    margin-right: -6px;
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
    user-select: none;
  }

  .chip:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .chip.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--text-primary);
    box-shadow: inset 0 0 0 1px var(--accent);
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
    font-size: 11px;
    line-height: 1;
    padding: 2px 6px;
    border-radius: 999px;
    background: var(--surface-active);
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .chip-actions {
    display: inline-flex;
    align-items: center;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  .chip-wrap:hover .chip-actions,
  .chip-wrap:focus-within .chip-actions,
  .chip-actions.show {
    opacity: 1;
    pointer-events: auto;
  }

  .chip-del {
    display: inline-grid;
    place-items: center;
    width: 18px;
    height: 18px;
    margin-left: 2px;
    border: none;
    border-radius: 999px;
    background: var(--surface-hover);
    color: var(--text-tertiary);
    cursor: pointer;
  }

  .chip-del:hover {
    color: var(--error);
  }

  .scene-row .chip {
    background: color-mix(in srgb, var(--accent-soft) 55%, var(--surface));
  }

  .scene-ico {
    color: var(--text-tertiary);
  }

  .chip.dim {
    opacity: 0.45;
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
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-tertiary);
    max-width: 420px;
    transition: border-color var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
  }

  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-ring);
    color: var(--accent);
  }

  .search:focus-within input {
    color: var(--text-primary);
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
    grid-template-columns: repeat(auto-fill, minmax(var(--desktop-cell, 230px), 1fr));
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
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    cursor: pointer;
    user-select: none;
    transition: border-color var(--duration-fast) var(--ease-out),
      background var(--duration-fast) var(--ease-out),
      transform var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
  }

  .item:hover {
    background: var(--surface-hover);
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
  }

  .item.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent);
  }

  .icon {
    display: grid;
    place-items: center;
    width: var(--desktop-icon, 36px);
    height: var(--desktop-icon, 36px);
    border-radius: var(--radius-m);
    flex-shrink: 0;
    color: var(--text-secondary);
    background: color-mix(in srgb, var(--surface-active) 55%, transparent);
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

  .expand {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin-left: -6px;
    border-radius: var(--radius-s);
    flex-shrink: 0;
    color: var(--text-tertiary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .item:hover .expand,
  .expand:hover {
    color: var(--accent);
    background: var(--accent-soft);
  }

  .crumbs {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
    font-size: var(--font-size-s);
  }

  .crumb {
    padding: 2px 8px;
    border: none;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--accent);
    cursor: pointer;
  }

  .crumb:hover {
    background: var(--accent-soft);
  }

  .crumb.current {
    color: var(--text-primary);
    font-weight: 600;
    cursor: default;
  }

  .crumb-sep {
    color: var(--text-tertiary);
  }

  .crumb-loading {
    color: var(--text-tertiary);
  }

  .state {
    padding: var(--space-6) var(--space-4);
    text-align: center;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-l);
    background: color-mix(in srgb, var(--surface) 45%, transparent);
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

  .drag-ghost {
    position: fixed;
    z-index: 50;
    pointer-events: none;
    transform: translate(-50%, -130%);
    max-width: 260px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 4px 12px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-primary);
    font-size: var(--font-size-s);
    box-shadow: var(--shadow-lg);
  }

  .drop-overlay {
    position: fixed;
    inset: 8px;
    z-index: 40;
    display: grid;
    place-items: center;
    border: 2px dashed var(--accent);
    border-radius: var(--radius-l);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
    font-weight: 600;
    pointer-events: none;
  }

  :global(body.dragging) {
    cursor: grabbing;
    user-select: none;
  }
</style>
