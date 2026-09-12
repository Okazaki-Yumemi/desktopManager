<script lang="ts">
  import { onMount } from "svelte";
  import type { Component } from "svelte";
  import {
    CalendarDays,
    ClipboardList,
    Download,
    Image as ImageIcon,
    LayoutGrid,
    ListTodo,
    Moon,
    Settings,
    Sun,
    Timer,
    Zap,
  } from "@lucide/svelte";
  import {
    exportCalendarIcs,
    startFocus,
  } from "../services/backend";
  import { navigate, PAGES, type PageId } from "../stores/router.svelte";
  import { setPaletteOpen } from "../stores/palette.svelte";
  import { pushToast } from "../stores/toast.svelte";
  import { setThemePreference } from "../stores/theme.svelte";
  import { startSjtuSync } from "../stores/sjtu.svelte";
  import { rotateWallpaper, wallpaperLib } from "../stores/wallpaper.svelte";

  interface Action {
    id: string;
    label: string;
    keywords: string;
    icon: Component;
    run: () => void | Promise<void>;
  }

  let query = $state("");
  let activeIdx = $state(0);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);
  let listEl = $state<HTMLUListElement | undefined>(undefined);

  const NAV_ICON: Record<PageId, Component> = {
    today: Sun,
    desktop: LayoutGrid,
    focus: Timer,
    calendar: CalendarDays,
    tasks: ListTodo,
    assignments: ClipboardList,
    settings: Settings,
  };

  const actions: Action[] = $derived.by(() => {
    const out: Action[] = PAGES.map((p) => ({
      id: `nav-${p.id}`,
      label: `打开：${p.label}`,
      keywords: `go open ${p.id}`,
      icon: NAV_ICON[p.id]!,
      run: () => navigate(p.id),
    }));
    out.push(
      {
        id: "focus-25",
        label: "开始专注：番茄 25 分钟",
        keywords: "focus pomodoro start",
        icon: Timer,
        run: async () => {
          try {
            await startFocus("pomodoro", 25 * 60, null, null);
            pushToast("info", "已开始番茄专注（25 分钟），可在专注页查看");
            navigate("focus");
          } catch (err) {
            pushToast("error", `无法开始：${err instanceof Error ? err.message : String(err)}`);
          }
        },
      },
      {
        id: "focus-50",
        label: "开始专注：深度 50 分钟",
        keywords: "focus deep start",
        icon: Zap,
        run: async () => {
          try {
            await startFocus("pomodoro", 50 * 60, null, null);
            pushToast("info", "已开始深度专注（50 分钟），可在专注页查看");
            navigate("focus");
          } catch (err) {
            pushToast("error", `无法开始：${err instanceof Error ? err.message : String(err)}`);
          }
        },
      },
      {
        id: "new-task",
        label: "去记一个任务",
        keywords: "task new todo",
        icon: ListTodo,
        run: () => navigate("tasks"),
      },
      {
        id: "theme-light",
        label: "主题：浅色",
        keywords: "theme light",
        icon: Sun,
        run: () => void setThemePreference("light"),
      },
      {
        id: "theme-dark",
        label: "主题：深色",
        keywords: "theme dark",
        icon: Moon,
        run: () => void setThemePreference("dark"),
      },
      {
        id: "theme-system",
        label: "主题：跟随系统",
        keywords: "theme system auto",
        icon: Settings,
        run: () => void setThemePreference("system"),
      },
      {
        id: "sjtu-sync",
        label: "同步交大日程",
        keywords: "sjtu sync calendar",
        icon: CalendarDays,
        run: async () => {
          try {
            await startSjtuSync();
            pushToast(
              "info",
              "交大日历窗口已打开；若要求登录请在窗口中登录 jAccount。",
              9000,
            );
          } catch (err) {
            pushToast("error", `无法打开同步窗口：${err instanceof Error ? err.message : String(err)}`);
          }
        },
      },
      {
        id: "export-ics",
        label: "导出日历 ICS",
        keywords: "export ics calendar",
        icon: Download,
        run: async () => {
          try {
            const r = await exportCalendarIcs();
            pushToast("ok", `已导出 ${r.count} 条日程到 ${r.path}`);
          } catch (err) {
            pushToast("error", `导出失败：${err instanceof Error ? err.message : String(err)}`);
          }
        },
      },
    );
    if (wallpaperLib.names.length >= 2) {
      out.push({
        id: "wallpaper-next",
        label: "换一张壁纸",
        keywords: "wallpaper next switch",
        icon: ImageIcon,
        run: async () => {
          if (await rotateWallpaper()) pushToast("ok", "已换一张壁纸");
        },
      });
    }
    return out;
  });

  const filtered = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return actions;
    return actions.filter(
      (a) => a.label.toLowerCase().includes(needle) || a.keywords.includes(needle),
    );
  });

  $effect(() => {
    // Keep the selection inside the filtered list.
    if (activeIdx >= filtered.length) activeIdx = Math.max(0, filtered.length - 1);
    const el = listEl?.children[activeIdx];
    el?.scrollIntoView({ block: "nearest" });
  });

  function runAction(action: Action | undefined): void {
    if (!action) return;
    setPaletteOpen(false);
    void action.run();
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      setPaletteOpen(false);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      activeIdx = Math.min(filtered.length - 1, activeIdx + 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      activeIdx = Math.max(0, activeIdx - 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      runAction(filtered[activeIdx]);
    }
  }

  function closeOnBackdrop(e: MouseEvent): void {
    if (e.target === e.currentTarget) setPaletteOpen(false);
  }

  onMount(() => {
    query = "";
    activeIdx = 0;
    queueMicrotask(() => inputEl?.focus());
  });
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  aria-label="命令面板"
  tabindex="-1"
  onkeydown={onKeydown}
  onclick={closeOnBackdrop}
>
  <div class="palette">
    <input
      class="query"
      type="text"
      placeholder="输入命令或页面名…（↑↓ 选择，回车执行，Esc 关闭）"
      bind:value={query}
      bind:this={inputEl}
      oninput={() => (activeIdx = 0)}
    />
    <ul class="list" bind:this={listEl}>
      {#each filtered as action, i (action.id)}
        <li>
          <button
            type="button"
            class="item"
            class:active={i === activeIdx}
            onmouseenter={() => (activeIdx = i)}
            onclick={() => runAction(action)}
          >
            <span class="item-icon"><action.icon size={15} aria-hidden="true" /></span>
            <span class="item-label">{action.label}</span>
          </button>
        </li>
      {:else}
        <li class="no-match">没有匹配的命令</li>
      {/each}
    </ul>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 14vh;
    background: color-mix(in srgb, var(--bg) 45%, transparent);
    backdrop-filter: blur(2px);
  }

  .palette {
    width: min(560px, calc(100vw - 48px));
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-l);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }

  .query {
    padding: 13px 16px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--text-primary);
    font-size: var(--font-size-l);
    outline: none;
  }

  .query::placeholder {
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: var(--space-2);
    overflow-y: auto;
  }

  .item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    padding: 8px 10px;
    border: none;
    border-radius: var(--radius-m);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    text-align: left;
  }

  .item.active {
    background: var(--accent-soft);
    color: var(--text-primary);
  }

  .item-icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-s);
    background: var(--surface-active);
    color: var(--accent);
    flex-shrink: 0;
  }

  .item-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .no-match {
    padding: var(--space-4);
    text-align: center;
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }
</style>
