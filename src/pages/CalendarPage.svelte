<script lang="ts">
  import { onMount } from "svelte";
  import { ChevronLeft, ChevronRight, X } from "@lucide/svelte";
  import SjtuSidebar from "../components/SjtuSidebar.svelte";
  import {
  createEvent,
  deleteEvent,
  exportCalendarIcs,
  getSetting,
  listEventsRange,
  listTasks,
  setSetting,
} from "../services/backend";
  import type { CalendarEvent, Task } from "../types/domain";
  import { pushToast } from "../stores/toast.svelte";
  import {
    loadSjtu,
    sjtuEvents,
    startSjtuReminder,
    watchSjtuSynced,
    watchSjtuWindowClosed,
  } from "../stores/sjtu.svelte";

  const DAY_MS = 86_400_000;
  const HOUR_H = 40; // px per hour in the grid

  const WEEKDAYS = ["一", "二", "三", "四", "五", "六", "日"] as const;
  const HOURS = Array.from({ length: 24 }, (_, i) => i);

  let events = $state<CalendarEvent[]>([]);
  let tasks = $state<Task[]>([]);
  // Anchor day inside the displayed week/month (any day works; the rest is
  // derived). `view` is restored from settings in onMount.
  let anchor = $state(startOfDay(new Date()));
  let selectedDay = $state(startOfDay(new Date()));
  let view = $state<"week" | "month">("week");

  // Inline creator, prefilled from the clicked grid slot.
  let creating = $state(false);
  let createTitle = $state("");
  let createStart = $state("");
  let createDur = $state(60);
  let createAllDay = $state(false);
  let createTaskId = $state<number | null>(null);

  function startOfDay(d: Date): Date {
    return new Date(d.getFullYear(), d.getMonth(), d.getDate());
  }

  /** Monday 00:00 of the week containing `d`. */
  function weekStart(d: Date): Date {
    const day = startOfDay(d);
    const shift = (day.getDay() + 6) % 7;
    return new Date(day.getTime() - shift * DAY_MS);
  }

  const weekStartsAt = $derived(weekStart(anchor));
  const days = $derived(
    Array.from({ length: 7 }, (_, i) => new Date(weekStartsAt.getTime() + i * DAY_MS)),
  );
  const weekEnd = $derived(weekStartsAt.getTime() + 7 * DAY_MS);
  const rangeLabel = $derived.by(() => {
    const a = days[0]!;
    const b = days[6]!;
    return `${a.getMonth() + 1}月${a.getDate()}日 – ${b.getMonth() + 1}月${b.getDate()}日`;
  });

  /**
   * Per-day render model for the week grid. Items are computed once per day
   * (instead of per row) and the weekend/today flags drive the column tint.
   */
  const weekDays = $derived(
    days.map((d, i) => ({
      d,
      i,
      weekend: d.getDay() === 0 || d.getDay() === 6,
      today: isToday(d),
      all: dayItems(d).filter((e) => e.allDay),
      timed: dayItems(d).filter((e) => !e.allDay),
    })),
  );

  // Month view: 42 cells starting from the Monday on/before the 1st.
  const monthStart = $derived(new Date(anchor.getFullYear(), anchor.getMonth(), 1));
  const monthCells = $derived.by(() => {
    const first = weekStart(monthStart);
    return Array.from({ length: 42 }, (_, i) => new Date(first.getTime() + i * DAY_MS));
  });
  const monthLabel = $derived(
    `${monthStart.getFullYear()}年${monthStart.getMonth() + 1}月`,
  );
  const headLabel = $derived(view === "week" ? rangeLabel : monthLabel);

  function isToday(d: Date): boolean {
    return startOfDay(new Date()).getTime() === d.getTime();
  }

  /**
   * One renderable entry for a calendar day: local events and the read-only
   * SJTU projection merged. `kind` drives the color and whether the agenda
   * offers a delete button.
   */
  type DayItem = {
    key: string;
    id: number;
    title: string;
    startsAt: number;
    endsAt: number;
    allDay: boolean;
    location: string | null;
    taskId: number | null;
    kind: "local" | "sjtu";
  };

  function dayItems(d: Date): DayItem[] {
    const from = d.getTime();
    const to = from + DAY_MS;
    const local = events
      .filter((e) => e.startsAt < to && e.endsAt > from)
      .map(
        (e): DayItem => ({
          key: `local-${e.id}`,
          id: e.id,
          title: e.title,
          startsAt: e.startsAt,
          endsAt: e.endsAt,
          allDay: e.allDay,
          location: null,
          taskId: e.taskId,
          kind: "local",
        }),
      );
    const sjtu = sjtuEvents()
      .filter((e) => e.startsAt < to && e.endsAt > from)
      .map(
        (e): DayItem => ({
          key: `sjtu-${e.id}`,
          id: e.id,
          title: e.title,
          startsAt: e.startsAt,
          endsAt: e.endsAt,
          allDay: e.allDay,
          location: e.location,
          taskId: null,
          kind: "sjtu",
        }),
      );
    // All-day entries first, then by start time.
    return [...local, ...sjtu].sort((a, b) =>
      a.allDay === b.allDay ? a.startsAt - b.startsAt : a.allDay ? -1 : 1,
    );
  }

  onMount(() => {
    void restoreView().then(() => reload());
    void loadSjtu();
    const stopReminder = startSjtuReminder();
    let unlisten: (() => void) | undefined;
    let unlistenClosed: (() => void) | undefined;
    void watchSjtuSynced().then((un) => (unlisten = un));
    void watchSjtuWindowClosed().then((un) => (unlistenClosed = un));
    return () => {
      stopReminder();
      unlisten?.();
      unlistenClosed?.();
    };
  });

  async function restoreView() {
    try {
      const saved = await getSetting<string>("ui.calendarView");
      if (saved === "week" || saved === "month") view = saved;
    } catch {
      // Backend unavailable: keep the default week view.
    }
  }

  async function onExportIcs() {
    try {
      const r = await exportCalendarIcs();
      pushToast("ok", `已导出 ${r.count} 条日程到 ${r.path}`);
    } catch (err) {
      pushToast("error", `导出失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function reload() {
    try {
      const [evts, tsks] = await Promise.all([listEventsRange(visibleFrom(), visibleTo()), listTasks()]);
      events = evts;
      tasks = tsks;
    } catch (err) {
      pushToast("error", `读取日程失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  /** Event-fetch range covering whatever the current view shows. */
  function visibleFrom(): number {
    return view === "month" ? monthCells[0]!.getTime() : weekStartsAt.getTime();
  }

  function visibleTo(): number {
    return view === "month" ? monthCells[41]!.getTime() + DAY_MS : weekEnd;
  }

  async function setView(v: "week" | "month") {
    if (view === v) return;
    view = v;
    await reload();
    // Persistence is best-effort: the in-session view already switched.
    try {
      await setSetting("ui.calendarView", v);
    } catch {
      /* degraded mode / storage failure */
    }
  }

  function shiftBack() {
    if (view === "week") {
      anchor = new Date(anchor.getTime() - 7 * DAY_MS);
    } else {
      anchor = new Date(anchor.getFullYear(), anchor.getMonth() - 1, 1);
    }
    void reload();
  }

  function shiftForward() {
    if (view === "week") {
      anchor = new Date(anchor.getTime() + 7 * DAY_MS);
    } else {
      anchor = new Date(anchor.getFullYear(), anchor.getMonth() + 1, 1);
    }
    void reload();
  }

  function goToday() {
    anchor = startOfDay(new Date());
    selectedDay = anchor;
    void reload();
  }

  /** Open the creator from a grid click: `d` = day, `hour` = start hour. */
  function openCreator(d: Date, hour: number | null) {
    creating = true;
    createAllDay = hour === null;
    const base = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const h = hour === null ? 0 : hour;
    createStart = `${base}T${String(h).padStart(2, "0")}:00`;
    createTitle = "";
    createDur = 60;
    createTaskId = null;
  }

  async function submitCreate() {
    if (!creating) return;
    const title = createTitle.trim();
    if (!title) {
      creating = false;
      return;
    }
    const startMs = new Date(createStart).getTime();
    const endMs = createAllDay
      ? startMs + DAY_MS
      : startMs + createDur * 60_000;
    if (Number.isNaN(startMs)) {
      pushToast("error", "开始时间无效");
      return;
    }
    try {
      await createEvent(title, startMs, endMs, createAllDay, {
        taskId: createTaskId,
      });
      creating = false;
      await reload();
      pushToast("ok", "日程已创建");
    } catch (err) {
      pushToast("error", `创建失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function remove(item: DayItem) {
    try {
      await deleteEvent(item.id);
      await reload();
    } catch (err) {
      pushToast("error", `删除失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  function timeOf(ms: number): string {
    const d = new Date(ms);
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  }

  function taskTitle(id: number | null): string | null {
    if (id === null) return null;
    return tasks.find((t) => t.id === id)?.title ?? null;
  }

  /** Grid offset for the current-time marker. */
  const nowPct = $derived.by(() => {
    const n = new Date();
    return ((n.getHours() * 60 + n.getMinutes()) / 1440) * 100;
  });
</script>

<div class="calendar-layout page-enter">
  <div class="calendar">
    <header class="head">
    <div>
      <h1>日历</h1>
      <p class="muted">{headLabel} · 本地数据，不上传</p>
    </div>
    <div class="nav">
      <div class="view-toggle" role="radiogroup" aria-label="视图切换">
        <button
          type="button"
          role="radio"
          aria-checked={view === "week"}
          class:active={view === "week"}
          onclick={() => void setView("week")}
        >
          周
        </button>
        <button
          type="button"
          role="radio"
          aria-checked={view === "month"}
          class:active={view === "month"}
          onclick={() => void setView("month")}
        >
          月
        </button>
      </div>
      <button type="button" class="nav-btn" title={view === "week" ? "上一周" : "上一月"} onclick={shiftBack}>
        <ChevronLeft size={15} />
      </button>
      <button type="button" class="nav-btn today" onclick={() => goToday()}>今天</button>
      <button type="button" class="nav-btn" title={view === "week" ? "下一周" : "下一月"} onclick={shiftForward}>
        <ChevronRight size={15} />
      </button>
    </div>
  </header>

  {#if creating}
    <div class="creator">
      <input
        type="text"
        placeholder="日程标题，回车创建"
        bind:value={createTitle}
        maxlength="60"
        onkeydown={(e) => {
          if (e.key === "Enter") void submitCreate();
          if (e.key === "Escape") creating = false;
        }}
      />
      <label>
        开始
        <input type="datetime-local" bind:value={createStart} disabled={createAllDay} />
      </label>
      <label>
        时长
        <select bind:value={createDur} disabled={createAllDay}>
          <option value={30}>30 分钟</option>
          <option value={60}>1 小时</option>
          <option value={90}>1.5 小时</option>
          <option value={120}>2 小时</option>
          <option value={240}>4 小时</option>
        </select>
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={createAllDay} />
        全天
      </label>
      <label>
        关联任务
        <select bind:value={createTaskId}>
          <option value={null}>不关联</option>
          {#each tasks.filter((t) => t.status !== "done") as t (t.id)}
            <option value={t.id}>{t.title}</option>
          {/each}
        </select>
      </label>
      <button type="button" class="btn primary" onclick={() => void submitCreate()}>创建</button>
      <button type="button" class="btn" onclick={() => (creating = false)}>取消</button>
    </div>
  {:else}
    <div class="page-actions">
      <button type="button" class="btn new-event" onclick={() => openCreator(selectedDay, 9)}>
        ＋ 新建日程
      </button>
      <button type="button" class="btn" onclick={() => void onExportIcs()}>导出 ICS</button>
    </div>
  {/if}

  {#if view === "week"}
  <div class="week glass" role="grid" aria-label="周视图">
    <div class="corner" aria-hidden="true"></div>
    {#each weekDays as day (day.d.getTime())}
      <button
        type="button"
        class="col-head"
        class:last-col={day.i === 6}
        class:today={day.today}
        class:selected={selectedDay.getTime() === day.d.getTime()}
        onclick={() => (selectedDay = day.d)}
        ondblclick={() => openCreator(day.d, null)}
      >
        <span class="dow">周{WEEKDAYS[day.i]}</span>
        <span class="dom" class:mark={day.today}>{day.d.getDate()}</span>
      </button>
    {/each}

    <div class="gutter allday-label" aria-hidden="true">全天</div>
    {#each weekDays as day (day.d.getTime())}
      <div class="allday" class:last-col={day.i === 6} class:weekend={day.weekend}>
        {#each day.all as e (e.key)}
          <button
            type="button"
            class="ev all"
            class:sjtu={e.kind === "sjtu"}
            title={e.title}
            onclick={() => (selectedDay = day.d)}
          >
            {e.title}
          </button>
        {/each}
      </div>
    {/each}

    <div class="gutter hours" style={`height: ${24 * HOUR_H}px`} aria-hidden="true">
      {#each HOURS as h (h)}
        <span class="hour" style={`top: ${h * HOUR_H}px`}>{String(h).padStart(2, "0")}:00</span>
      {/each}
    </div>
    {#each weekDays as day (day.d.getTime())}
      <div
        class="grid"
        class:last-col={day.i === 6}
        class:weekend={day.weekend}
        class:today={day.today}
        style={`height: ${24 * HOUR_H}px`}
      >
        {#each HOURS as h (h)}
          <button
            type="button"
            class="slot"
            style={`top: ${h * HOUR_H}px; height: ${HOUR_H}px`}
            title="{day.d.getMonth() + 1}月{day.d.getDate()}日 {String(h).padStart(2, '0')}:00 — 点击新建"
            onclick={() => openCreator(day.d, h)}
          ></button>
        {/each}
        {#if day.today}
          <div class="now-line" style={`top: ${(nowPct / 100) * 24 * HOUR_H}px`}></div>
        {/if}
        {#each day.timed as e (e.key)}
          {@const s = Math.max(e.startsAt, day.d.getTime())}
          {@const en = Math.min(e.endsAt, day.d.getTime() + DAY_MS)}
          {@const top = ((new Date(s).getHours() * 60 + new Date(s).getMinutes()) / 1440) * 24 * HOUR_H}
          {@const height = Math.max(((en - s) / DAY_MS) * 24 * HOUR_H, 18)}
          <div
            class="ev timed"
            class:linked={e.taskId !== null}
            class:sjtu={e.kind === "sjtu"}
            style={`top: ${top}px; height: ${height}px`}
            title={`${timeOf(e.startsAt)}–${timeOf(e.endsAt)} ${e.title}`}
          >
            {e.title}
          </div>
        {/each}
      </div>
    {/each}
  </div>
  {/if}

  {#if view === "month"}
    <div class="month glass" role="grid" aria-label="月视图">
      <div class="month-head">
        {#each WEEKDAYS as w, i (i)}
          <span class="dow">周{w}</span>
        {/each}
      </div>
      <div class="month-body">
        {#each monthCells as c (c.getTime())}
          {@const evs = dayItems(c)}
          <button
            type="button"
            role="gridcell"
            class="cell"
            class:dim={c.getMonth() !== monthStart.getMonth()}
            class:today={isToday(c)}
            class:selected={selectedDay.getTime() === c.getTime()}
            onclick={() => (selectedDay = c)}
            ondblclick={() => openCreator(c, null)}
          >
            <span class="num">{c.getDate()}</span>
            <span class="chips">
              {#each evs.slice(0, 3) as e (e.key)}
                <span
                  class="chip"
                  class:linked={e.kind === "local" && e.taskId !== null}
                  class:sjtu={e.kind === "sjtu"}
                  title={`${e.allDay ? "全天" : timeOf(e.startsAt)} ${e.title}`}
                >
                  {#if !e.allDay}
                    <span class="ct">{timeOf(e.startsAt)}</span>
                  {/if}
                  <span class="ct-title">{e.title}</span>
                </span>
              {/each}
              {#if evs.length > 3}
                <span class="more">还有 {evs.length - 3} 项</span>
              {/if}
            </span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <section class="agenda glass" aria-label="当日日程">
    <h2>
      {selectedDay.getMonth() + 1}月{selectedDay.getDate()}日
      周{WEEKDAYS[(selectedDay.getDay() + 6) % 7]}
    </h2>
    {#if dayItems(selectedDay).length === 0}
      <p class="muted">
        {view === "week"
          ? "这一天还没有安排——点击周网格上的任意小时即可新建"
          : "这一天还没有安排——双击月历上的日期即可新建"}
      </p>
    {:else}
      <ul>
        {#each dayItems(selectedDay) as e (e.key)}
          <li>
            <span class="when">
              {e.allDay ? "全天" : `${timeOf(e.startsAt)}–${timeOf(e.endsAt)}`}
            </span>
            <span class="what">
              {#if e.kind === "sjtu"}
                <span class="sjtu-tag">交大</span>
              {/if}
              {e.title}
              {#if e.location}
                <span class="loc">· {e.location}</span>
              {/if}
              {#if e.kind === "local" && taskTitle(e.taskId)}
                <span class="task-ref">· {taskTitle(e.taskId)}</span>
              {/if}
            </span>
            {#if e.kind === "local"}
              <button type="button" class="del" title="删除日程" onclick={() => void remove(e)}>
                <X size={13} />
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
  </div>

  <SjtuSidebar />
</div>

<style>
  .calendar-layout {
    display: flex;
    align-items: flex-start;
    justify-content: center;
    gap: var(--space-5);
    max-width: 1280px;
    margin: 0 auto;
  }

  .calendar {
    flex: 1;
    min-width: 0;
    max-width: 960px;
    margin: 0 auto;
  }

  @media (max-width: 1120px) {
    .calendar-layout {
      flex-direction: column;
      align-items: stretch;
    }
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
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

  .nav {
    display: inline-flex;
    gap: var(--space-2);
  }

  .nav-btn {
    display: grid;
    place-items: center;
    padding: 5px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .nav-btn:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .nav-btn.today {
    color: var(--accent);
    font-weight: 600;
  }

  .btn {
    padding: 5px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out);
  }

  .btn:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn.primary {
    border-color: transparent;
    background: var(--grad-accent);
    color: var(--accent-contrast);
    font-weight: 600;
    box-shadow: var(--shadow-sm);
  }

  .btn.primary:hover {
    background: var(--grad-accent);
    color: var(--accent-contrast);
    transform: translateY(-1px);
    box-shadow: var(--shadow-md);
  }

  .page-actions {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }

  .new-event {
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
  }

  .creator {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    margin-bottom: var(--space-3);
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border-strong));
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    box-shadow: 0 0 0 3px var(--accent-ring), var(--shadow-sm);
    font-size: var(--font-size-s);
  }

  .creator input[type="text"] {
    flex: 1;
    min-width: 160px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-s);
    background: var(--surface);
    color: var(--text-primary);
    outline: none;
  }

  .creator label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
  }

  .creator input,
  .creator select {
    border: 1px solid var(--border);
    border-radius: var(--radius-s);
    background: var(--surface);
    color: var(--text-primary);
    padding: 4px 6px;
  }

  /* Week grid: a time gutter + 7 day columns across three shared rows
     (header / all-day / time grid) so cells stay aligned even when the
     all-day row grows. */
  .week {
    display: grid;
    grid-template-columns: 46px repeat(7, 1fr);
    grid-template-rows: auto auto auto;
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    overflow: hidden;
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
  }

  .corner {
    grid-row: 1;
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }

  .gutter {
    grid-column: 1;
    border-right: 1px solid var(--border);
    position: relative;
  }

  .allday-label {
    grid-row: 2;
    display: grid;
    place-items: center;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .hours {
    grid-row: 3;
  }

  .hour {
    position: absolute;
    right: 6px;
    transform: translateY(-50%);
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-tertiary);
    line-height: 1;
  }

  .col-head {
    grid-row: 1;
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 6px;
    width: 100%;
    padding: var(--space-2) 0;
    border: none;
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .col-head.last-col,
  .allday.last-col,
  .grid.last-col {
    border-right: none;
  }

  .col-head:hover {
    background: var(--surface-hover);
  }

  .col-head.selected {
    background: var(--accent-soft);
  }

  .weekend {
    background: color-mix(in srgb, var(--text-tertiary) 6%, transparent);
  }

  .grid.today {
    background: color-mix(in srgb, var(--accent) 4%, transparent);
  }

  .dow {
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .dom {
    font-weight: 600;
  }

  .dom.mark {
    display: inline-grid;
    place-items: center;
    min-width: 24px;
    height: 24px;
    border-radius: 999px;
    background: var(--grad-accent);
    color: var(--accent-contrast);
  }

  .allday {
    grid-row: 2;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 26px;
    padding: 3px 4px;
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }

  .ev {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    border: none;
    border-left: 3px solid var(--accent);
    border-radius: 6px;
    background: color-mix(in srgb, var(--accent) 14%, var(--surface));
    color: var(--text-primary);
    font-size: var(--font-size-s);
    padding: 2px 6px;
    cursor: default;
    box-shadow: var(--shadow-sm);
  }

  .ev.all {
    cursor: pointer;
  }

  .ev.linked {
    border-left-color: var(--ok);
  }

  /* SJTU-synced entries: read-only, warning-amber, tagged in the agenda. */
  .ev.sjtu {
    border-left-color: var(--warn);
    background: color-mix(in srgb, var(--warn) 16%, var(--surface));
  }

  .sjtu-tag {
    display: inline-block;
    margin-right: 4px;
    padding: 0 5px;
    border-radius: var(--radius-s);
    background: color-mix(in srgb, var(--warn) 18%, transparent);
    color: var(--warn);
    font-size: 11px;
    vertical-align: 1px;
  }

  .loc {
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }

  .grid {
    position: relative;
    grid-row: 3;
    border-right: 1px solid var(--border);
  }

  .slot {
    position: absolute;
    left: 0;
    right: 0;
    border: none;
    border-top: 1px solid var(--border);
    background: transparent;
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out);
  }

  .slot:hover {
    background: var(--accent-soft);
  }

  .now-line {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(
      90deg,
      var(--error),
      color-mix(in srgb, var(--error) 30%, transparent)
    );
    pointer-events: none;
    z-index: 2;
  }

  .now-line::before {
    content: "";
    position: absolute;
    left: -4px;
    top: -3px;
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: var(--error);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--error) 22%, transparent);
  }

  .ev.timed {
    position: absolute;
    left: 3px;
    right: 3px;
    z-index: 1;
    pointer-events: none;
    line-height: 1.4;
  }

  .agenda {
    margin-top: var(--space-4);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow-sm);
  }

  .agenda h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--font-size-l);
    font-weight: 600;
  }

  .agenda ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .agenda li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 5px 8px;
    border-radius: var(--radius-m);
    transition: background var(--duration-fast) var(--ease-out);
  }

  .agenda li:hover {
    background: var(--surface-hover);
  }

  .when {
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .what {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .task-ref {
    color: var(--ok);
    font-size: var(--font-size-s);
  }

  .del {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out),
      background var(--duration-fast) var(--ease-out);
  }

  .agenda li:hover .del,
  .del:focus-visible {
    opacity: 1;
  }

  .del:hover {
    color: var(--error);
    background: color-mix(in srgb, var(--error) 10%, transparent);
  }

  .view-toggle {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    overflow: hidden;
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
  }

  .view-toggle button {
    padding: 5px 12px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .view-toggle button + button {
    border-left: 1px solid var(--border);
  }

  .view-toggle button.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .month {
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    overflow: hidden;
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
  }

  .month-head {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    border-bottom: 1px solid var(--border);
  }

  .month-head .dow {
    padding: var(--space-2) 0;
    text-align: center;
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .month-body {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    grid-auto-rows: minmax(96px, auto);
  }

  .cell {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
    padding: 4px 6px;
    border: none;
    border-right: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    background: transparent;
    cursor: pointer;
    text-align: left;
  }

  .cell:nth-child(7n) {
    border-right: none;
  }

  .cell:nth-last-child(-n + 7) {
    border-bottom: none;
  }

  .cell:hover {
    background: var(--surface-hover);
  }

  .cell.selected {
    outline: 1px solid var(--accent);
    outline-offset: -1px;
    background: color-mix(in srgb, var(--accent) 5%, transparent);
  }

  .cell.today {
    background: color-mix(in srgb, var(--accent) 4%, transparent);
  }

  .cell.dim .num {
    color: var(--text-tertiary);
    opacity: 0.55;
  }

  .cell .num {
    display: inline-grid;
    place-items: center;
    min-width: 20px;
    height: 20px;
    padding: 0 4px;
    border-radius: 999px;
    font-size: var(--font-size-s);
    font-weight: 600;
    color: var(--text-secondary);
  }

  .cell.today .num {
    background: var(--grad-accent);
    color: var(--accent-contrast);
  }

  .chips {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    min-width: 0;
  }

  .chip {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
    padding: 1px 6px 1px 5px;
    border-left: 3px solid var(--accent);
    border-radius: 5px;
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
    color: var(--text-primary);
    font-size: 11px;
    line-height: 1.6;
    text-align: left;
  }

  .chip .ct {
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-secondary);
  }

  .chip .ct-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip.sjtu {
    border-left-color: var(--warn);
    background: color-mix(in srgb, var(--warn) 16%, var(--surface));
  }

  .chip.sjtu .ct {
    color: var(--warn);
  }

  .chip.linked {
    border-left-color: var(--ok);
  }

  .cell.dim .chip {
    opacity: 0.6;
  }

  .more {
    font-size: 11px;
    color: var(--text-tertiary);
    padding-left: 5px;
  }
</style>
