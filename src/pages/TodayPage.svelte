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

  <section class="card" aria-label="Backend status">
    <div class="card-title-row">
      <h2>Backend status</h2>
      {#if info}
        <span class="pill ok">connected</span>
      {:else if loadError}
        <span class="pill error">error</span>
      {:else}
        <span class="pill">connecting…</span>
      {/if}
    </div>

    {#if info}
      <dl class="kv">
        <div>
          <dt>Version</dt>
          <dd>{info.version}</dd>
        </div>
        <div>
          <dt>Schema</dt>
          <dd>v{info.schemaVersion}</dd>
        </div>
        <div>
          <dt>Platform</dt>
          <dd>{info.os}</dd>
        </div>
        <div>
          <dt>Data dir</dt>
          <dd class="mono">{info.dataDir}</dd>
        </div>
        <div>
          <dt>Database</dt>
          <dd class="mono">{info.dbPath}</dd>
        </div>
        <div>
          <dt>Logs</dt>
          <dd class="mono">{info.logDir}</dd>
        </div>
      </dl>
    {:else if loadError}
      <p class="error-text">
        Could not reach the backend: {loadError}
        <br />
        (If you opened this page in a plain browser, run the app via
        <code>pnpm tauri dev</code> instead.)
      </p>
    {:else}
      <p class="muted">Contacting backend…</p>
    {/if}
  </section>

  <section class="card" aria-label="Quick start">
    <div class="card-title-row">
      <h2>Quick start</h2>
    </div>
    <p class="hint">
      Press <kbd>Alt</kbd>+<kbd>Shift</kbd>+<kbd>D</kbd> anywhere in Windows to
      show or hide this window. Closing the window only hides it to the tray —
      use the tray menu to really quit.
    </p>
    <p class="muted">Desktop organizing and the command palette arrive in upcoming milestones.</p>
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
