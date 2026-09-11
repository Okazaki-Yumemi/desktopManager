<script lang="ts">
  import { onMount } from "svelte";
  import { CalendarDays, ListTodo, Timer } from "@lucide/svelte";
  import { getAppInfo, getFocusSummary, listEventsRange, listTasks } from "../services/backend";
  import type { AppInfo } from "../types/domain";
  import { formatDateLong, greetingForHour } from "../lib/datetime";
  import { currentPage, navigate } from "../stores/router.svelte";
  import { loadTodayPrefs, todayPrefs, type ClockStyle } from "../stores/today.svelte";

  // Ticking clock: one `now` value drives greeting, date, time and seconds.
  let now = $state(new Date());
  let backend = $state<{ info: AppInfo | null; error: string | null }>({
    info: null,
    error: null,
  });
  let focusLine = $state<string | null>(null);

  // Quick-glance stats for the three homepage cards (null = unknown).
  let statTasks = $state<{ done: number; total: number } | null>(null);
  let statEvents = $state<number | null>(null);
  let statFocusMin = $state<number | null>(null);

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
    const pool = todayPrefs.mottos.length > 0 ? todayPrefs.mottos : MOTTOS;
    const dayIndex = Math.floor(now.getTime() / 86_400_000);
    return pool[((dayIndex % pool.length) + pool.length) % pool.length] ?? MOTTOS[0]!;
  });

  /** Split "HH:MM" into digits + colon flag for the card-style clocks. */
  const hourDigits = $derived.by(() => {
    const chars = time.split("");
    return { h: [chars[0]!, chars[1]!], m: [chars[3]!, chars[4]!], colon: chars[2]! };
  });

  function dayBounds(): [number, number] {
    const d = new Date();
    const start = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
    return [start, start + 86_400_000];
  }

  onMount(() => {
    void loadTodayPrefs();
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
          statFocusMin = Math.round(today.totalS / 60);
        } else {
          focusLine = null;
          statFocusMin = 0;
        }
      })
      .catch(() => {
        focusLine = null;
      });
    listTasks()
      .then((tasks) => {
        const today = tasks.filter((t) => t.status !== "done");
        statTasks = {
          done: tasks.length - today.length,
          total: tasks.length,
        };
      })
      .catch(() => {
        statTasks = null;
      });
    const [from, to] = dayBounds();
    listEventsRange(from, to)
      .then((events) => {
        statEvents = events.length;
      })
      .catch(() => {
        statEvents = null;
      });
    return () => clearInterval(timer);
  });

  function goto(page: ReturnType<typeof currentPage>): void {
    navigate(page);
  }

  const style: ClockStyle = $derived(todayPrefs.clockStyle);
</script>

<div class="today page-enter">
  <p class="greeting">{dateLine}</p>

  {#if style === "flip"}
    <div class="flip" role="timer" aria-label="当前时间 {time}">
      {#each hourDigits.h as d, i (`h-${i}-${d}`)}
        <span class="flip-card">{d}</span>
      {/each}
      <span class="colon blink">{hourDigits.colon}</span>
      {#each hourDigits.m as d, i (`m-${i}-${d}`)}
        <span class="flip-card">{d}</span>
      {/each}
    </div>
  {:else if style === "blocks"}
    <div class="blocks" role="timer" aria-label="当前时间 {time}">
      {#each hourDigits.h as d, i (`h-${i}`)}
        <span class="tile">{d}</span>
      {/each}
      <span class="colon accent">{hourDigits.colon}</span>
      {#each hourDigits.m as d, i (`m-${i}`)}
        <span class="tile">{d}</span>
      {/each}
      <span class="blocks-seconds">{seconds}</span>
    </div>
  {:else if style === "minimal"}
    <h1 class="clock minimal" aria-label="当前时间">{time}</h1>
  {:else}
    <h1 class="clock classic" aria-label="当前时间">
      {time}<span class="seconds">{seconds}</span>
    </h1>
  {/if}

  <p class="motto">「{motto}」</p>
  {#if focusLine}
    <p class="focus-line">{focusLine}</p>
  {/if}

  <div class="stats" aria-label="今日概览">
    <button type="button" class="stat" onclick={() => goto("tasks")}>
      <ListTodo size={15} aria-hidden="true" />
      <span class="stat-num">{statTasks ? `${statTasks.done}/${statTasks.total}` : "—"}</span>
      <span class="stat-label">任务完成</span>
    </button>
    <button type="button" class="stat" onclick={() => goto("calendar")}>
      <CalendarDays size={15} aria-hidden="true" />
      <span class="stat-num">{statEvents ?? "—"}</span>
      <span class="stat-label">今日日程</span>
    </button>
    <button type="button" class="stat" onclick={() => goto("focus")}>
      <Timer size={15} aria-hidden="true" />
      <span class="stat-num">{statFocusMin ?? "—"}</span>
      <span class="stat-label">专注分钟</span>
    </button>
  </div>

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
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: var(--space-2);
    padding: var(--space-6) 0;
  }

  /* Ambient accent glows behind the clock — purely decorative. */
  .today::before,
  .today::after {
    content: "";
    position: absolute;
    width: 440px;
    height: 440px;
    border-radius: 50%;
    filter: blur(90px);
    opacity: 0.22;
    pointer-events: none;
    z-index: -1;
  }

  .today::before {
    background: radial-gradient(circle, var(--accent), transparent 70%);
    top: 6%;
    left: 14%;
  }

  .today::after {
    background: radial-gradient(circle, var(--accent), transparent 70%);
    bottom: 2%;
    right: 12%;
    opacity: 0.14;
  }

  .greeting {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-l);
  }

  .clock {
    margin: var(--space-2) 0;
    font-family: var(--font-mono);
    font-weight: 600;
    line-height: 1;
    font-variant-numeric: tabular-nums;
  }

  .clock.classic {
    font-size: clamp(64px, 12vw, 112px);
    letter-spacing: 0.02em;
    background: linear-gradient(
      180deg,
      var(--text-primary) 30%,
      color-mix(in srgb, var(--text-primary) 55%, var(--accent))
    );
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  @supports not ((-webkit-background-clip: text) or (background-clip: text)) {
    .clock.classic {
      background: none;
      -webkit-text-fill-color: initial;
      color: var(--text-primary);
    }
  }

  .clock.classic .seconds {
    font-size: 0.32em;
    margin-left: 0.15em;
    font-weight: 400;
    -webkit-text-fill-color: var(--text-tertiary);
  }

  .clock.minimal {
    font-size: clamp(56px, 9vw, 88px);
    font-weight: 200;
    letter-spacing: 0.06em;
    color: var(--text-primary);
  }

  /* 翻页：digits roll in like split-flap cards. */
  .flip {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: var(--space-2) 0;
    perspective: 400px;
  }

  .flip-card {
    display: inline-grid;
    place-items: center;
    width: 1.35em;
    height: 1.9em;
    font-family: var(--font-mono);
    font-size: clamp(44px, 7vw, 72px);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    box-shadow: var(--shadow-md), inset 0 -1.5px 0 color-mix(in srgb, var(--border-strong) 70%, transparent);
    color: var(--text-primary);
    animation: flip-in 420ms var(--ease-out);
  }

  @keyframes flip-in {
    from {
      transform: rotateX(80deg);
      opacity: 0.25;
    }
  }

  .colon {
    font-family: var(--font-mono);
    font-size: clamp(40px, 6vw, 60px);
    font-weight: 600;
    color: var(--text-tertiary);
  }

  .colon.blink {
    animation: blink 2s steps(1) infinite;
  }

  @keyframes blink {
    50% {
      opacity: 0.25;
    }
  }

  /* 卡片：each digit sits in its own tile. */
  .blocks {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: var(--space-2) 0;
  }

  .tile {
    display: inline-grid;
    place-items: center;
    width: 1.25em;
    height: 1.8em;
    font-family: var(--font-mono);
    font-size: clamp(44px, 7vw, 72px);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    background: color-mix(in srgb, var(--accent) 8%, var(--glass));
    color: var(--text-primary);
    box-shadow: var(--shadow-sm);
  }

  .blocks .colon.accent {
    color: var(--accent);
  }

  .blocks-seconds {
    align-self: flex-end;
    margin-left: 4px;
    margin-bottom: 0.4em;
    font-family: var(--font-mono);
    font-size: clamp(14px, 1.6vw, 18px);
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
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

  .stats {
    display: flex;
    gap: var(--space-3);
    margin-top: var(--space-5);
  }

  .stat {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 8px 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-secondary);
    font-size: var(--font-size-s);
    cursor: pointer;
    transition: transform var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out), color var(--duration-fast) var(--ease-out);
  }

  .stat :global(svg) {
    color: var(--accent);
    flex-shrink: 0;
  }

  .stat:hover {
    transform: translateY(-1px);
    box-shadow: var(--shadow-sm);
    color: var(--text-primary);
  }

  .stat-num {
    font-weight: 600;
    font-size: var(--font-size-l);
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }

  .status {
    margin-top: var(--space-5);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--font-size-s);
    padding: 3px 12px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-tertiary);
  }

  .pill::before {
    content: "";
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: currentColor;
    flex-shrink: 0;
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
