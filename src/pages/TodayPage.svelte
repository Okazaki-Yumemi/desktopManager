<script lang="ts">
  import { onMount } from "svelte";
  import { getAppInfo, getFocusSummary } from "../services/backend";
  import type { AppInfo } from "../types/domain";
  import { formatDateLong, greetingForHour } from "../lib/datetime";

  // Ticking clock: one `now` value drives greeting, date, time and seconds.
  let now = $state(new Date());
  let backend = $state<{ info: AppInfo | null; error: string | null }>({
    info: null,
    error: null,
  });
  let focusLine = $state<string | null>(null);

  // Original taglines, picked deterministically per calendar day.
  const MOTTOS: ReadonlyArray<string> = [
    "把桌面整理好，把心情腾出来。",
    "一次只做一件事。",
    "少即是多，慢即是快。",
    "干净的桌面，清醒的头脑。",
    "先完成，再完美。",
    "工具应当隐于无形。",
    "今天的整理，是明天的从容。",
    "专注当下，其余自会就位。",
    "秩序不是束缚，是省下来的力气。",
    "桌面如镜，照见今日所求。",
  ];

  function pad(n: number): string {
    return String(n).padStart(2, "0");
  }

  const time = $derived(`${pad(now.getHours())}:${pad(now.getMinutes())}`);
  const seconds = $derived(pad(now.getSeconds()));
  const dateLine = $derived(`${greetingForHour(now.getHours())} · ${formatDateLong(now)}`);
  const motto = $derived.by(() => {
    const dayIndex = Math.floor(now.getTime() / 86_400_000);
    return MOTTOS[((dayIndex % MOTTOS.length) + MOTTOS.length) % MOTTOS.length] ?? MOTTOS[0]!;
  });

  onMount(() => {
    const timer = setInterval(() => {
      now = new Date();
    }, 1000);
    getAppInfo()
      .then((info) => {
        backend = { info, error: null };
      })
      .catch((err: unknown) => {
        backend = { info: null, error: err instanceof Error ? err.message : String(err) };
      });
    getFocusSummary(1)
      .then((days) => {
        const today = days.at(-1);
        if (today && today.totalS > 0) {
          const h = Math.floor(today.totalS / 3600);
          const m = Math.round((today.totalS % 3600) / 60);
          focusLine =
            h > 0
              ? `今日专注 ${h} 小时 ${m} 分钟 · ${today.sessions} 段`
              : `今日专注 ${m} 分钟 · ${today.sessions} 段`;
        } else {
          focusLine = null;
        }
      })
      .catch(() => {
        focusLine = null;
      });
    return () => clearInterval(timer);
  });
</script>

<div class="today">
  <p class="greeting">{dateLine}</p>
  <h1 class="clock" aria-label="当前时间">
    {time}<span class="seconds">{seconds}</span>
  </h1>
  <p class="motto">「{motto}」</p>
  {#if focusLine}
    <p class="focus-line">{focusLine}</p>
  {/if}

  <footer class="status">
    {#if backend.info}
      <span class="pill ok" title="版本 {backend.info.version} · 数据库结构 v{backend.info.schemaVersion}">
        已连接 · v{backend.info.version}
      </span>
    {:else if backend.error}
      <span class="pill error" title={backend.error}>
        未连接 —— 请用 pnpm tauri dev 启动应用
      </span>
    {:else}
      <span class="pill">连接中…</span>
    {/if}
  </footer>
</div>

<style>
  .today {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: var(--space-2);
    padding: var(--space-6) 0;
  }

  .greeting {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-l);
  }

  .clock {
    margin: var(--space-2) 0;
    font-family: var(--font-mono);
    font-size: clamp(64px, 12vw, 112px);
    font-weight: 600;
    line-height: 1;
    letter-spacing: 0.02em;
    text-shadow: 0 1px 2px rgb(0 0 0 / 0.08);
  }

  .seconds {
    font-size: 0.32em;
    color: var(--text-tertiary);
    margin-left: 0.15em;
    font-weight: 400;
  }

  .motto {
    margin: var(--space-2) 0 0;
    color: var(--text-secondary);
    font-size: var(--font-size-l);
  }

  .focus-line {
    margin: var(--space-1) 0 0;
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }

  .status {
    margin-top: var(--space-6);
  }

  .pill {
    font-size: var(--font-size-s);
    padding: 2px 12px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-tertiary);
  }

  .pill.ok {
    color: var(--ok);
    border-color: color-mix(in srgb, var(--ok) 35%, transparent);
  }

  .pill.error {
    color: var(--error);
    border-color: color-mix(in srgb, var(--error) 35%, transparent);
  }
</style>
