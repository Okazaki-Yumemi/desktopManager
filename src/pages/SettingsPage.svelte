<script lang="ts">
  import { onMount } from "svelte";
  import {
    ACCENT_PRESETS,
    DENSITY_OPTIONS,
    GLASS_OPTIONS,
    ICON_SIZE_OPTIONS,
    MOTION_OPTIONS,
    SURFACE_PRESETS,
    densityPref,
    getAccentPreference,
    getCustomAccent,
    getThemePreference,
    glassPref,
    iconSizePref,
    motionPref,
    setAccentPreference,
    setCustomAccent,
    setThemePreference,
    surfacePref,
    type AccentPreference,
    type DensityPreference,
    type GlassPreference,
    type IconSizePreference,
    type MotionPreference,
    type SurfacePreference,
    type ThemePreference,
  } from "../stores/theme.svelte";
  import { isSoundEnabled, setSoundEnabled } from "../lib/chime.svelte";
  import {
    applyLayout,
    captureLayout,
    clearSjtuEvents,
    deleteLayout,
    getAppInfo,
    getShortcutInfo,
    listLayouts,
    openSjtuSync,
    purgeAppData,
  } from "../services/backend";
  import type { AppInfo, LayoutSummary, ShortcutInfo } from "../types/domain";
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
  let info = $state<AppInfo | null>(null);

  onMount(() => {
    getShortcutInfo()
      .then((v) => {
        shortcut = v;
      })
      .catch(() => {
        shortcut = null;
      });
    getAppInfo()
      .then((v) => {
        info = v;
      })
      .catch(() => {
        info = null;
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

  const currentSurface = $derived(surfacePref.get());
  const currentDensity = $derived(densityPref.get());
  const currentGlass = $derived(glassPref.get());
  const currentMotion = $derived(motionPref.get());
  const customAccent = $derived(getCustomAccent());
  const soundOn = $derived(isSoundEnabled());
  const currentIconSize = $derived(iconSizePref.get());

  async function chooseSurface(value: SurfacePreference) {
    try {
      await surfacePref.set(value);
    } catch (err) {
      pushToast("error", `无法保存外观风格：${String(err)}`);
    }
  }

  async function chooseDensity(value: DensityPreference) {
    try {
      await densityPref.set(value);
    } catch (err) {
      pushToast("error", `无法保存密度：${String(err)}`);
    }
  }

  async function chooseGlass(value: GlassPreference) {
    try {
      await glassPref.set(value);
    } catch (err) {
      pushToast("error", `无法保存毛玻璃强度：${String(err)}`);
    }
  }

  async function chooseMotion(value: MotionPreference) {
    try {
      await motionPref.set(value);
    } catch (err) {
      pushToast("error", `无法保存动效偏好：${String(err)}`);
    }
  }

  async function chooseCustomAccent(hex: string) {
    try {
      await setCustomAccent(hex);
    } catch (err) {
      pushToast("error", `无法保存自定义颜色：${String(err)}`);
    }
  }

  async function chooseSound(value: boolean) {
    try {
      await setSoundEnabled(value);
    } catch (err) {
      pushToast("error", `无法保存提示音偏好：${String(err)}`);
    }
  }

  async function chooseIconSize(value: IconSizePreference) {
    try {
      await iconSizePref.set(value);
    } catch (err) {
      pushToast("error", `无法保存图标大小：${String(err)}`);
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

  // --- 桌面布局快照（LVM 通道，只动图标位置，不动文件） ---------------------

  let layouts = $state<LayoutSummary[]>([]);
  let layoutName = $state("");
  let layoutBusy = $state(false);

  function refreshLayouts() {
    listLayouts()
      .then((v) => {
        layouts = v;
      })
      .catch(() => {
        layouts = [];
      });
  }

  onMount(refreshLayouts);

  async function onSaveLayout() {
    const name = layoutName.trim();
    if (!name) {
      pushToast("info", "请先填写布局名称");
      return;
    }
    layoutBusy = true;
    try {
      const saved = await captureLayout(name);
      pushToast("ok", `已保存布局「${saved.name}」（${saved.itemCount} 项）`);
      layoutName = "";
      refreshLayouts();
    } catch (err) {
      pushToast("error", `保存布局失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      layoutBusy = false;
    }
  }

  async function onApplyLayout(id: number) {
    layoutBusy = true;
    try {
      const r = await applyLayout(id);
      if (r.diverged > 0) {
        pushToast("info", `已恢复 ${r.applied} 项，${r.diverged} 项与保存值有偏差（网格吸附）`);
      } else {
        pushToast("ok", `已恢复 ${r.applied} 项图标位置`);
      }
      if (r.missing > 0) pushToast("info", `${r.missing} 项当前不在桌面上，已跳过`);
    } catch (err) {
      pushToast("error", `恢复布局失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      layoutBusy = false;
    }
  }

  async function onDeleteLayout(id: number) {
    try {
      await deleteLayout(id);
      refreshLayouts();
    } catch (err) {
      pushToast("error", `删除失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  // --- 上海交大日程（M12） --------------------------------------------------

  let sjtuBusy = $state(false);
  let sjtuArm = $state(false);

  async function onSjtuSync() {
    sjtuBusy = true;
    try {
      await openSjtuSync();
      pushToast(
        "info",
        "交大日历窗口已打开；若要求登录请在窗口中登录 jAccount，同步完成后会自动关闭。",
        9000,
      );
    } catch (err) {
      pushToast("error", `无法打开同步窗口：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      sjtuBusy = false;
    }
  }

  function armSjtuClear() {
    if (!sjtuArm) {
      sjtuArm = true;
      setTimeout(() => {
        if (sjtuArm) sjtuArm = false;
      }, 5000);
      return;
    }
    sjtuArm = false;
    clearSjtuEvents()
      .then((n) => pushToast("ok", n > 0 ? `已清除 ${n} 条交大日程` : "交大日程本来就是空的"))
      .catch((err: unknown) => {
        pushToast("error", `清除失败：${err instanceof Error ? err.message : String(err)}`);
      });
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
            aria-checked={currentAccent === a.value && customAccent === null}
            aria-label={a.label}
            title={a.label}
            class="swatch"
            class:selected={currentAccent === a.value && customAccent === null}
            style={`--swatch: ${SWATCH[a.value]}`}
            onclick={() => chooseAccent(a.value)}
          ></button>
        {/each}
        <label
          class="swatch swatch-custom"
          class:selected={customAccent !== null}
          title={customAccent ?? "自定义颜色"}
        >
          <input
            type="color"
            value={customAccent ?? "#2f6fd0"}
            oninput={(e) => void chooseCustomAccent((e.currentTarget as HTMLInputElement).value)}
            aria-label="自定义强调色"
          />
        </label>
      </div>
    </div>
    <div class="row row-gap">
      <div class="row-text">
        <span class="row-title">外观风格</span>
        <span class="row-desc">圆角与阴影的观感；“纯黑”只在深色主题下生效。</span>
      </div>
      <div class="segmented" role="radiogroup" aria-label="外观风格">
        {#each SURFACE_PRESETS as s (s.value)}
          <button
            type="button"
            role="radio"
            aria-checked={currentSurface === s.value}
            class:active={currentSurface === s.value}
            onclick={() => chooseSurface(s.value)}
          >
            {s.label}
          </button>
        {/each}
      </div>
    </div>
    <div class="row row-gap">
      <div class="row-text">
        <span class="row-title">密度</span>
        <span class="row-desc">“紧凑”会缩小间距与字号，一屏能看到更多内容。</span>
      </div>
      <div class="segmented" role="radiogroup" aria-label="密度">
        {#each DENSITY_OPTIONS as d (d.value)}
          <button
            type="button"
            role="radio"
            aria-checked={currentDensity === d.value}
            class:active={currentDensity === d.value}
            onclick={() => chooseDensity(d.value)}
          >
            {d.label}
          </button>
        {/each}
      </div>
    </div>
    <div class="row row-gap">
      <div class="row-text">
        <span class="row-title">图标大小</span>
        <span class="row-desc">桌面项目网格的列宽与图标框大小。</span>
      </div>
      <div class="segmented" role="radiogroup" aria-label="图标大小">
        {#each ICON_SIZE_OPTIONS as s (s.value)}
          <button
            type="button"
            role="radio"
            aria-checked={currentIconSize === s.value}
            class:active={currentIconSize === s.value}
            onclick={() => chooseIconSize(s.value)}
          >
            {s.label}
          </button>
        {/each}
      </div>
    </div>
    <div class="row row-gap">
      <div class="row-text">
        <span class="row-title">毛玻璃强度</span>
        <span class="row-desc">卡片透出背景的程度；关闭后变为实色面板。</span>
      </div>
      <div class="segmented" role="radiogroup" aria-label="毛玻璃强度">
        {#each GLASS_OPTIONS as g (g.value)}
          <button
            type="button"
            role="radio"
            aria-checked={currentGlass === g.value}
            class:active={currentGlass === g.value}
            onclick={() => chooseGlass(g.value)}
          >
            {g.label}
          </button>
        {/each}
      </div>
    </div>
    <div class="row row-gap">
      <div class="row-text">
        <span class="row-title">动效</span>
        <span class="row-desc">“减弱”缩短过渡；“关闭”立刻切换所有界面状态。</span>
      </div>
      <div class="segmented" role="radiogroup" aria-label="动效">
        {#each MOTION_OPTIONS as m (m.value)}
          <button
            type="button"
            role="radio"
            aria-checked={currentMotion === m.value}
            class:active={currentMotion === m.value}
            onclick={() => chooseMotion(m.value)}
          >
            {m.label}
          </button>
        {/each}
      </div>
    </div>
  </section>

  <section class="group" aria-label="通知">
    <h2>通知</h2>
    <div class="row">
      <div class="row-text">
        <span class="row-title">完成提示音</span>
        <span class="row-desc">专注倒计时自然结束时播放一声短提示；手动结束不响。</span>
      </div>
      <div class="segmented" role="radiogroup" aria-label="完成提示音">
        <button
          type="button"
          role="radio"
          aria-checked={soundOn}
          class:active={soundOn}
          onclick={() => void chooseSound(true)}
        >
          开
        </button>
        <button
          type="button"
          role="radio"
          aria-checked={!soundOn}
          class:active={!soundOn}
          onclick={() => void chooseSound(false)}
        >
          关
        </button>
      </div>
    </div>
  </section>

  <section class="group" aria-label="上海交大日程">
    <h2>上海交大日程</h2>
    <div class="row">
      <div class="row-text">
        <span class="row-title">课程表同步</span>
        <span class="row-desc">
          打开交大日历窗口并登录 jAccount 后，课程与校历自动同步到「日历」页侧边栏。
          登录凭据只保存在系统 WebView 中，本应用不读取账号密码，数据不上传。
        </span>
      </div>
      <span class="btn-row">
        <button type="button" class="btn" onclick={() => void onSjtuSync()} disabled={sjtuBusy}>
          同步
        </button>
        <button type="button" class="danger" class:armed={sjtuArm} onclick={() => armSjtuClear()}>
          {sjtuArm ? "再点一次确认" : "清除"}
        </button>
      </span>
    </div>
    <p class="row-desc row-gap">
      同步窗口会打开 my.sjtu.edu.cn 的日历页面；登录一次后凭据留在 WebView，下次同步通常免登录。
    </p>
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

  <section class="group" aria-label="桌面布局快照">
    <h2>桌面布局快照</h2>
    <div class="row">
      <div class="row-text">
        <span class="row-title">保存当前布局</span>
        <span class="row-desc">
          通过系统消息读取桌面图标位置并保存；不移动、不改名任何文件。
        </span>
      </div>
      <span class="btn-row">
        <input
          class="layout-name"
          placeholder="布局名称，如：工作布局"
          bind:value={layoutName}
          maxlength="40"
        />
        <button type="button" class="btn" onclick={() => void onSaveLayout()} disabled={layoutBusy}>
          保存
        </button>
      </span>
    </div>
    {#if layouts.length === 0}
      <p class="row-desc row-gap">还没有保存的布局。</p>
    {:else}
      <ul class="layout-list row-gap">
        {#each layouts as l (l.id)}
          <li>
            <span class="layout-title">{l.name}</span>
            <span class="layout-meta">
              {l.itemCount} 项 · {new Date(l.createdAt).toLocaleString()}
            </span>
            <span class="btn-row">
              <button
                type="button"
                class="btn"
                onclick={() => void onApplyLayout(l.id)}
                disabled={layoutBusy}
              >
                应用
              </button>
              <button type="button" class="btn" onclick={() => void onDeleteLayout(l.id)}>
                删除
              </button>
            </span>
          </li>
        {/each}
      </ul>
    {/if}
    <p class="row-desc row-gap">
      应用前会先做一次“金丝雀”探测：若检测到“自动排列图标”开启（位置写入会被系统忽略），将拒绝恢复并提示。
    </p>
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

  <section class="group" aria-label="快速上手">
    <h2>快速上手</h2>
    <p class="row-desc">
      在 Windows 任意位置按 <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>D</kbd>
      即可显示或隐藏本窗口。关闭窗口只是隐藏到托盘 —— 想彻底退出请使用托盘菜单。
    </p>
    <p class="row-desc row-gap">
      桌面页：点击集合筛选，把项目拖到集合上归类（只记元数据，不会移动文件）；
      点集合上的铅笔可重命名，文件夹引用可用箭头在应用内展开。
    </p>
    <p class="row-desc">专注页提供番茄钟与正计时；任务与日历将随 M6 到来。</p>
  </section>

  <section class="group" aria-label="系统信息">
    <h2>系统信息</h2>
    {#if info}
      <dl class="kv">
        <div>
          <dt>版本</dt>
          <dd>{info.version}</dd>
        </div>
        <div>
          <dt>数据库结构</dt>
          <dd>v{info.schemaVersion}</dd>
        </div>
        <div>
          <dt>系统</dt>
          <dd>{info.os}</dd>
        </div>
        <div>
          <dt>数据目录</dt>
          <dd class="mono">{info.dataDir}</dd>
        </div>
        <div>
          <dt>数据库</dt>
          <dd class="mono">{info.dbPath}</dd>
        </div>
        <div>
          <dt>日志</dt>
          <dd class="mono">{info.logDir}</dd>
        </div>
      </dl>
    {:else}
      <p class="row-desc">正在读取后端信息…</p>
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
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow);
    padding: var(--space-4) var(--space-5);
  }

  .kv {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-3) var(--space-5);
  }

  .kv dt {
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
    margin-bottom: 2px;
  }

  .kv dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  kbd {
    display: inline-block;
    padding: 1px 7px;
    border: 1px solid var(--border-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-s);
    background: var(--surface);
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
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
    align-items: center;
  }

  .swatch-custom {
    display: grid;
    place-items: center;
    overflow: hidden;
    background: conic-gradient(#e5484d, #e8b45a, #46a758, #0090ff, #7c5cd6, #e5484d);
  }

  .swatch-custom input {
    width: 14px;
    height: 14px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    cursor: pointer;
    /* Strip the native color-input chrome; the picker opens on click. */
    &::-webkit-color-swatch-wrapper {
      padding: 0;
    }
    &::-webkit-color-swatch {
      border: none;
      border-radius: 50%;
    }
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

  .layout-name {
    width: 200px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-primary);
  }

  .layout-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .layout-list li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
  }

  .layout-title {
    font-weight: 500;
    white-space: nowrap;
  }

  .layout-meta {
    flex: 1;
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
    overflow-wrap: anywhere;
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
