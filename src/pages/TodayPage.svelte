<script lang="ts">
  import { onMount } from "svelte";
  import { getAppInfo } from "../services/backend";
  import type { AppInfo } from "../types/domain";
  import { formatDateLong, greetingForHour } from "../lib/datetime";

  let info = $state<AppInfo | null>(null);
  let loadError = $state<string | null>(null);

  const now = new Date();
  const dateLine = formatDateLong(now);
  const greeting = greetingForHour(now.getHours());

  onMount(() => {
    getAppInfo()
      .then((v) => {
        info = v;
      })
      .catch((err: unknown) => {
        loadError = err instanceof Error ? err.message : String(err);
      });
  });
</script>

<div class="today">
  <header>
    <p class="greeting">{greeting}</p>
    <h1>{dateLine}</h1>
  </header>

  <section class="card" aria-label="后端状态">
    <div class="card-title-row">
      <h2>后端状态</h2>
      {#if info}
        <span class="pill ok">已连接</span>
      {:else if loadError}
        <span class="pill error">出错</span>
      {:else}
        <span class="pill">连接中…</span>
      {/if}
    </div>

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
    {:else if loadError}
      <p class="error-text">
        无法连接后端：{loadError}
        <br />
        （如果你是在普通浏览器里打开的页面，请改用 <code>pnpm tauri dev</code> 启动应用。）
      </p>
    {:else}
      <p class="muted">正在连接后端…</p>
    {/if}
  </section>

  <section class="card" aria-label="快速上手">
    <div class="card-title-row">
      <h2>快速上手</h2>
    </div>
    <p class="hint">
      在 Windows 任意位置按 <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>D</kbd> 即可显示或隐藏本窗口。
      关闭窗口只是隐藏到托盘 —— 想彻底退出请使用托盘菜单。
    </p>
    <p class="muted">桌面整理与命令面板将在后续里程碑中到来。</p>
  </section>
</div>

<style>
  .today {
    max-width: 720px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  header .greeting {
    margin: 0 0 var(--space-1);
    color: var(--text-secondary);
  }

  header h1 {
    margin: 0;
    font-size: var(--font-size-xl);
    font-weight: 600;
  }

  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow);
    padding: var(--space-4) var(--space-5);
  }

  .card-title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-3);
  }

  .card-title-row h2 {
    margin: 0;
    font-size: var(--font-size-l);
    font-weight: 600;
  }

  .pill {
    font-size: var(--font-size-s);
    padding: 2px 10px;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .pill.ok {
    color: var(--ok);
    border-color: color-mix(in srgb, var(--ok) 35%, transparent);
  }

  .pill.error {
    color: var(--error);
    border-color: color-mix(in srgb, var(--error) 35%, transparent);
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

  .mono {
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
  }

  .muted {
    color: var(--text-tertiary);
  }

  .hint {
    margin: 0 0 var(--space-2);
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

  .error-text {
    color: var(--error);
    overflow-wrap: anywhere;
  }
</style>
