<script lang="ts">
  import { SvelteDate } from "svelte/reactivity";
  import { GraduationCap, RefreshCw, Trash2 } from "@lucide/svelte";
  import {
    clearSjtu,
    sjtuEvents,
    sjtuLastSyncAt,
    sjtuNext,
    startSjtuSync,
    sjtuSyncing,
  } from "../stores/sjtu.svelte";

  const DAY_MS = 86_400_000;
  const WEEKDAYS = ["日", "一", "二", "三", "四", "五", "六"] as const;

  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(timer);
  });

  const events = $derived(sjtuEvents());
  const lastSync = $derived(sjtuLastSyncAt());
  const syncing = $derived(sjtuSyncing());
  const { running, next } = $derived(sjtuNext(now));

  const dayStart = $derived(new SvelteDate(now).setHours(0, 0, 0, 0));
  const today = $derived(
    events
      .filter((e) => e.startsAt < dayStart + DAY_MS && e.endsAt > dayStart)
      .sort((a, b) => a.startsAt - b.startsAt),
  );

  function timeOf(ms: number): string {
    const d = new Date(ms);
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  }

  function dayOf(ms: number): string {
    const d = new Date(ms);
    return `周${WEEKDAYS[d.getDay()]}`;
  }

  function countdown(ms: number): string {
    const total = Math.max(0, Math.round(ms / 60_000));
    const h = Math.floor(total / 60);
    const m = total % 60;
    return h === 0 ? `${m} 分钟` : `${h} 小时 ${m} 分`;
  }

  function syncLabel(ms: number | null): string {
    if (ms === null) return "尚未同步";
    return new Date(ms).toLocaleString();
  }
</script>

<aside class="sjtu glass" aria-label="交大日程">
  <header class="head">
    <h2><GraduationCap size={15} aria-hidden="true" /> 交大日程</h2>
    <button
      type="button"
      class="icon-btn"
      title="打开交大日历并同步"
      aria-label="打开交大日历并同步"
      disabled={syncing}
      onclick={() => void startSjtuSync()}
    >
      <RefreshCw size={14} class={syncing ? "spin" : ""} />
    </button>
  </header>

  {#if running}
    <div class="card running">
      <span class="tag">正在上课</span>
      <p class="title">{running.title}</p>
      <p class="meta">
        {timeOf(running.startsAt)}–{timeOf(running.endsAt)}{running.location ? ` · ${running.location}` : ""}
      </p>
      <p class="count">还剩 {countdown(running.endsAt - now)}</p>
    </div>
  {:else if next}
    <div class="card">
      <span class="tag">下一节课</span>
      <p class="title">{next.title}</p>
      <p class="meta">
        {dayOf(next.startsAt)} {timeOf(next.startsAt)}{next.location ? ` · ${next.location}` : ""}
      </p>
      <p class="count">{countdown(next.startsAt - now)}后开始</p>
    </div>
  {:else if events.length > 0}
    <p class="muted">接下来没有课程安排。</p>
  {/if}

  {#if today.length > 0}
    <ul class="today">
      {#each today as e (e.id)}
        <li>
          <span class="when">{e.allDay ? "全天" : `${timeOf(e.startsAt)}–${timeOf(e.endsAt)}`}</span>
          <span class="what" title={e.title}>{e.title}</span>
        </li>
      {/each}
    </ul>
  {/if}

  {#if events.length === 0}
    <p class="muted">
      还没有交大日程。点击右上角按钮打开交大日历，登录 jAccount 后课程表会自动同步到这里。
    </p>
  {:else}
    <footer class="foot">
      <span class="muted">{events.length} 条 · {syncLabel(lastSync)}</span>
      <button
        type="button"
        class="icon-btn del"
        title="清除交大日程数据"
        aria-label="清除交大日程数据"
        onclick={() => void clearSjtu()}
      >
        <Trash2 size={13} />
      </button>
    </footer>
  {/if}
</aside>

<style>
  .sjtu {
    width: 264px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow-sm);
  }

  @media (max-width: 1120px) {
    .sjtu {
      width: auto;
    }
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h2 {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: var(--font-size-l);
    font-weight: 600;
  }

  .icon-btn {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-s);
    background: var(--surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out), border-color var(--duration-fast) var(--ease-out);
  }

  .icon-btn:hover:not(:disabled) {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }

  .icon-btn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .icon-btn :global(svg.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-3);
    border: 1px solid var(--border);
    border-left: 3px solid var(--warn);
    border-radius: var(--radius-m);
    background: var(--surface);
    box-shadow: var(--shadow-sm);
  }

  .card.running {
    border-left-color: var(--ok);
  }

  .tag {
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .card.running .tag {
    color: var(--ok);
  }

  .title {
    margin: 0;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .meta {
    margin: 0;
    font-size: var(--font-size-s);
    color: var(--text-secondary);
  }

  .count {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
    color: var(--warn);
  }

  .card.running .count {
    color: var(--ok);
  }

  .today {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .today li {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    padding: 4px 0;
    border-top: 1px solid var(--border);
    font-size: var(--font-size-s);
  }

  .today li:first-child {
    border-top: none;
  }

  .when {
    font-family: var(--font-mono);
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .what {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-top: auto;
  }

  .muted {
    margin: 0;
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }
</style>
