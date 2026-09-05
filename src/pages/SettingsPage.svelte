<script lang="ts">
  import { onMount } from "svelte";
  import {
    ACCENT_PRESETS,
    getAccentPreference,
    getThemePreference,
    setAccentPreference,
    setThemePreference,
    type AccentPreference,
    type ThemePreference,
  } from "../stores/theme.svelte";
  import { getShortcutInfo, purgeAppData } from "../services/backend";
  import type { ShortcutInfo } from "../types/domain";
  import { pushToast } from "../stores/toast.svelte";
  import {
    clearWallpaper,
    setWallpaperOpacity,
    uploadWallpaper,
    wallpaper,
  } from "../stores/wallpaper.svelte";

  const options: ReadonlyArray<{ value: ThemePreference; label: string }> = [
    { value: "system", label: "跟随系统" },
    { value: "light", label: "浅色" },
    { value: "dark", label: "深色" },
  ];

  // Swatch colors are fixed identities per accent, independent of the
  // currently applied theme shades.
  const SWATCH: Readonly<Record<AccentPreference, string>> = {
    ocean: "#2f6fd0",
    violet: "#7c5cd6",
    grass: "#2f8f4e",
    amber: "#b97a10",
    rose: "#c2455f",
  };

  const current = $derived(getThemePreference());
  const currentAccent = $derived(getAccentPreference());

  let shortcut = $state<ShortcutInfo | null>(null);

  onMount(() => {
    getShortcutInfo()
      .then((v) => {
        shortcut = v;
      })
      .catch(() => {
        shortcut = null;
      });
  });

  function formatBinding(binding: string): string {
    return binding
      .split("+")
      .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
      .join(" + ");
  }

  async function choose(value: ThemePreference) {
    try {
      await setThemePreference(value);
    } catch (err) {
      pushToast("error", `无法保存主题：${String(err)}`);
    }
  }

  async function chooseAccent(value: AccentPreference) {
    try {
      await setAccentPreference(value);
    } catch (err) {
      pushToast("error", `无法保存强调色：${String(err)}`);
    }
  }

  // --- 自定义背景 ---------------------------------------------------------

  let bgBusy = $state(false);
  let fileInput = $state<HTMLInputElement | undefined>(undefined);

  async function onPickImage(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    bgBusy = true;
    try {
      await uploadWallpaper(file);
      pushToast("ok", "背景已更新");
    } catch (err) {
      pushToast("error", `设置背景失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      bgBusy = false;
    }
  }

  function onOpacityInput(e: Event) {
    wallpaper.opacity = Number((e.currentTarget as HTMLInputElement).value) / 100;
  }

  async function onOpacityCommit() {
    try {
      await setWallpaperOpacity(wallpaper.opacity);
    } catch (err) {
      pushToast("error", `保存透明度失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function onClearWallpaper() {
    try {
      await clearWallpaper();
      pushToast("ok", "背景已清除");
    } catch (err) {
      pushToast("error", `清除失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  // --- 数据管理（两步确认的破坏性操作） -------------------------------------

  let purgeArm = $state<"collections" | "all" | null>(null);

  function armPurge(kind: "collections" | "all") {
    if (purgeArm !== kind) {
      purgeArm = kind;
      setTimeout(() => {
        if (purgeArm === kind) purgeArm = null;
      }, 5000);
      return;
    }
    purgeArm = null;
    purgeAppData(kind)
      .then(() => {
        if (kind === "all") {
          pushToast("ok", "已重置，正在刷新界面…");
          setTimeout(() => location.reload(), 800);
        } else {
          pushToast("ok", "集合数据已清空");
        }
      })
      .catch((err: unknown) => {
        pushToast("error", `清理失败：${err instanceof Error ? err.message : String(err)}`);
      });
  }
</script>

<div class="settings">
  <h1>设置</h1>

  <section class="group" aria-label="外观">
    <h2>外观</h2>
    <div class="row">
      <div class="row-text">
        <span class="row-title">主题</span>
        <span class="row-desc">保存在本地；“跟随系统”会自动跟随 Windows。</span>
      </div>
      <div class="segmented" role="radiogroup" aria-label="Theme">
        {#each options as o (o.value)}
          <button
            type="button"
            role="radio"
            aria-checked={current === o.value}
            class:active={current === o.value}
            onclick={() => choose(o.value)}
          >
            {o.label}
          </button>
        {/each}
      </div>
    </div>
    <div class="row row-gap">
      <div class="row-text">
        <span class="row-title">强调色</span>
        <span class="row-desc">用于选中态、高亮与主要操作。</span>
      </div>
      <div class="swatches" role="radiogroup" aria-label="强调色">
        {#each ACCENT_PRESETS as a (a.value)}
          <button
            type="button"
            role="radio"
            aria-checked={currentAccent === a.value}
            aria-label={a.label}
            title={a.label}
            class="swatch"
            class:selected={currentAccent === a.value}
            style={`--swatch: ${SWATCH[a.value]}`}
            onclick={() => chooseAccent(a.value)}
          ></button>
        {/each}
      </div>
    </div>
  </section>

  <section class="group" aria-label="自定义背景">
    <h2>自定义背景</h2>
    <div class="row">
      <div class="row-text">
        <span class="row-title">背景图片</span>
        <span class="row-desc">保存在本机应用数据目录，不会上传或分享。</span>
      </div>
      <span class="btn-row">
        <input
          type="file"
          accept="image/png,image/jpeg,image/webp"
          hidden
          bind:this={fileInput}
          onchange={onPickImage}
        />
        <button type="button" class="btn" onclick={() => fileInput?.click()} disabled={bgBusy}>
          选择图片…
        </button>
        {#if wallpaper.active}
          <button type="button" class="btn" onclick={() => void onClearWallpaper()}>清除</button>
        {/if}
      </span>
    </div>
    {#if wallpaper.active}
      <div class="row row-gap">
        <div class="row-text">
          <span class="row-title">不透明度</span>
          <span class="row-desc">当前 {Math.round(wallpaper.opacity * 100)}%。</span>
        </div>
        <input
          type="range"
          min="0"
          max="100"
          value={Math.round(wallpaper.opacity * 100)}
          oninput={onOpacityInput}
          onchange={() => void onOpacityCommit()}
          aria-label="背景不透明度"
        />
      </div>
    {/if}
  </section>

  <section class="group" aria-label="全局快捷键">
    <h2>全局快捷键</h2>
    <div class="row">
      <div class="row-text">
        <span class="row-title mono">{shortcut ? formatBinding(shortcut.binding) : "…"}</span>
        <span class="row-desc">
          在任意界面显示 / 隐藏 DesktopManager。后续里程碑中它会升级为命令面板热键。
        </span>
      </div>
      {#if shortcut?.registered}
        <span class="pill ok">已注册</span>
      {:else if shortcut?.error}
        <span class="pill error">冲突</span>
      {:else}
        <span class="pill">检测中…</span>
      {/if}
    </div>
    {#if shortcut?.error}
      <p class="error-text">
        该组合键已被其他程序占用（{shortcut.error}）。
        关闭占用程序后重启 DesktopManager 即可重试。
      </p>
    {/if}
  </section>

  <section class="group" aria-label="数据管理">
    <h2>数据管理</h2>
    <div class="row">
      <div class="row-text">
        <span class="row-title">清空集合</span>
        <span class="row-desc">删除所有集合与分配记录，桌面索引保留。清除前自动备份数据库。</span>
      </div>
      <button
        type="button"
        class="danger"
        class:armed={purgeArm === "collections"}
        onclick={() => armPurge("collections")}
      >
        {purgeArm === "collections" ? "再点一次确认" : "清空"}
      </button>
    </div>
    <div class="row row-gap">
      <div class="row-text">
        <span class="row-title">重置全部数据</span>
        <span class="row-desc">
          集合、索引、设置、背景全部清空并恢复初始状态（自动备份；桌面上的真实文件不受影响）。
        </span>
      </div>
      <button
        type="button"
        class="danger"
        class:armed={purgeArm === "all"}
        onclick={() => armPurge("all")}
      >
        {purgeArm === "all" ? "再点一次确认" : "重置"}
      </button>
    </div>
  </section>

  <p class="note">更多设置（自定义强调色、密度、性能模式）将随 M7 到来。</p>
</div>

<style>
  .settings {
    max-width: 720px;
    margin: 0 auto;
  }

  h1 {
    font-size: var(--font-size-xl);
    font-weight: 600;
    margin: 0 0 var(--space-5);
  }

  .group {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow);
    padding: var(--space-4) var(--space-5);
  }

  .group h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--font-size-l);
    font-weight: 600;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  .row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row-title {
    font-weight: 500;
  }

  .row-desc {
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .segmented {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    overflow: hidden;
  }

  .segmented button {
    border: none;
    background: var(--surface);
    padding: 6px 14px;
    cursor: pointer;
    color: var(--text-secondary);
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .segmented button + button {
    border-left: 1px solid var(--border);
  }

  .segmented button:hover {
    background: var(--surface-hover);
  }

  .segmented button.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .note {
    margin-top: var(--space-4);
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }

  .row-gap {
    margin-top: var(--space-4);
  }

  .swatches {
    display: inline-flex;
    gap: var(--space-2);
  }

  .swatch {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2px solid transparent;
    background: var(--swatch);
    cursor: pointer;
    padding: 0;
    transition: transform var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
  }

  .swatch:hover {
    transform: scale(1.1);
  }

  .swatch.selected {
    box-shadow: 0 0 0 2px var(--surface), 0 0 0 4px var(--swatch);
  }

  .pill {
    font-size: var(--font-size-s);
    padding: 2px 10px;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .pill.ok {
    color: var(--ok);
    border-color: color-mix(in srgb, var(--ok) 35%, transparent);
  }

  .pill.error {
    color: var(--error);
    border-color: color-mix(in srgb, var(--error) 35%, transparent);
  }

  .error-text {
    margin: var(--space-3) 0 0;
    color: var(--error);
    font-size: var(--font-size-s);
    overflow-wrap: anywhere;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .btn-row {
    display: inline-flex;
    gap: var(--space-2);
  }

  .btn {
    padding: 6px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-secondary);
    cursor: pointer;
    white-space: nowrap;
  }

  .btn:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  input[type="range"] {
    width: 180px;
    accent-color: var(--accent);
  }

  .danger {
    padding: 6px 14px;
    border: 1px solid color-mix(in srgb, var(--error) 35%, transparent);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--error);
    cursor: pointer;
    white-space: nowrap;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .danger.armed {
    background: var(--error);
    border-color: var(--error);
    color: #fff;
  }
</style>
