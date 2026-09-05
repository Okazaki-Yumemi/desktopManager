<script lang="ts">
  import { onMount } from "svelte";
  import { ChevronLeft, ChevronRight, X } from "@lucide/svelte";
  import { createEvent, deleteEvent, listEventsRange, listTasks } from "../services/backend";
  import type { CalendarEvent, Task } from "../types/domain";
  import { pushToast } from "../stores/toast.svelte";

  const DAY_MS = 86_400_000;
  const HOUR_H = 40; // px per hour in the grid

  const WEEKDAYS = ["一", "二", "三", "四", "五", "六", "日"] as const;
  const HOURS = Array.from({ length: 24 }, (_, i) => i);

  let events = $state<CalendarEvent[]>([]);
  let tasks = $state<Task[]>([]);
  // Anchor day inside the displayed week (any day works; Monday is derived).
  let anchor = $state(startOfDay(new Date()));
  let selectedDay = $state(startOfDay(new Date()));

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

  function isToday(d: Date): boolean {
    return startOfDay(new Date()).getTime() === d.getTime();
  }

  function dayEvents(d: Date): CalendarEvent[] {
    const from = d.getTime();
    const to = from + DAY_MS;
    return events.filter((e) => e.startsAt < to && e.endsAt > from);
  }

  function timedEvents(d: Date): CalendarEvent[] {
    return dayEvents(d).filter((e) => !e.allDay);
  }

  onMount(() => {
    void reload();
  });

  async function reload() {
    try {
      const [evts, tsks] = await Promise.all([listEventsRange(weekStartsAt.getTime(), weekEnd), listTasks()]);
      events = evts;
      tasks = tsks;
    } catch (err) {
      pushToast("error", `读取日程失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  function shiftWeek(delta: number) {
    anchor = new Date(anchor.getTime() + delta * 7 * DAY_MS);
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

  async function remove(event: CalendarEvent) {
    try {
      await deleteEvent(event.id);
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

<div class="calendar">
  <header class="head">
    <div>
      <h1>日历</h1>
      <p class="muted">{rangeLabel} · 本地数据，不上传</p>
    </div>
    <div class="nav">
      <button type="button" class="nav-btn" title="上一周" onclick={() => shiftWeek(-1)}>
        <ChevronLeft size={15} />
      </button>
      <button type="button" class="nav-btn today" onclick={() => goToday()}>今天</button>
      <button type="button" class="nav-btn" title="下一周" onclick={() => shiftWeek(1)}>
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
    <button type="button" class="btn new-event" onclick={() => openCreator(selectedDay, 9)}>
      ＋ 新建日程
    </button>
  {/if}

  <div class="week glass" role="grid" aria-label="周视图">
    {#each days as d, i (d.getTime())}
      {@const dayTimed = timedEvents(d)}
      {@const dayAll = dayEvents(d).filter((e) => e.allDay)}
      <div class="col" class:today={isToday(d)} class:selected={selectedDay.getTime() === d.getTime()}>
        <button
          type="button"
          class="col-head"
          onclick={() => (selectedDay = d)}
          ondblclick={() => openCreator(d, null)}
        >
          <span class="dow">周{WEEKDAYS[i]}</span>
          <span class="dom" class:mark={isToday(d)}>{d.getDate()}</span>
        </button>
        <div class="allday">
          {#each dayAll as e (e.id)}
            <button type="button" class="ev all" title={e.title} onclick={() => (selectedDay = d)}>
              {e.title}
            </button>
          {/each}
        </div>
        <div class="grid" style={`height: ${24 * HOUR_H}px`}>
          {#each HOURS as h (h)}
            <button
              type="button"
              class="slot"
              style={`top: ${h * HOUR_H}px; height: ${HOUR_H}px`}
              title="{d.getMonth() + 1}月{d.getDate()}日 {String(h).padStart(2, '0')}:00 — 点击新建"
              onclick={() => openCreator(d, h)}
            ></button>
          {/each}
          {#if isToday(d)}
            <div class="now-line" style={`top: ${(nowPct / 100) * 24 * HOUR_H}px`}></div>
          {/if}
          {#each dayTimed as e (e.id)}
            {@const s = Math.max(e.startsAt, d.getTime())}
            {@const en = Math.min(e.endsAt, d.getTime() + DAY_MS)}
            {@const top = ((new Date(s).getHours() * 60 + new Date(s).getMinutes()) / 1440) * 24 * HOUR_H}
            {@const height = Math.max(((en - s) / DAY_MS) * 24 * HOUR_H, 18)}
            <div
              class="ev timed"
              class:linked={e.taskId !== null}
              style={`top: ${top}px; height: ${height}px`}
              title={`${timeOf(e.startsAt)}–${timeOf(e.endsAt)} ${e.title}`}
            >
              {e.title}
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>

  <section class="agenda glass" aria-label="当日日程">
    <h2>
      {selectedDay.getMonth() + 1}月{selectedDay.getDate()}日
      周{WEEKDAYS[(selectedDay.getDay() + 6) % 7]}
    </h2>
    {#if dayEvents(selectedDay).length === 0}
      <p class="muted">这一天还没有安排——点击周网格上的任意小时即可新建</p>
    {:else}
      <ul>
        {#each dayEvents(selectedDay) as e (e.id)}
          <li>
            <span class="when">
              {e.allDay ? "全天" : `${timeOf(e.startsAt)}–${timeOf(e.endsAt)}`}
            </span>
            <span class="what">
              {e.title}
              {#if taskTitle(e.taskId)}
                <span class="task-ref">· {taskTitle(e.taskId)}</span>
              {/if}
            </span>
            <button type="button" class="del" title="删除日程" onclick={() => void remove(e)}>
              <X size={13} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</div>

<style>
  .calendar {
    max-width: 960px;
    margin: 0 auto;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: var(--space-4);
  }

  h1 {
    margin: 0 0 var(--space-1);
    font-size: var(--font-size-xl);
    font-weight: 600;
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
  }

  .nav-btn:hover {
    background: var(--surface-hover);
  }

  .btn {
    padding: 5px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-secondary);
    cursor: pointer;
  }

  .btn.primary {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .new-event {
    margin-bottom: var(--space-3);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
  }

  .creator {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3);
    margin-bottom: var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    font-size: var(--font-size-s);
  }

  .creator input[type="text"] {
    flex: 1;
    min-width: 160px;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
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
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-primary);
    padding: 4px 6px;
  }

  .week {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    overflow: hidden;
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
  }

  .col {
    border-right: 1px solid var(--border);
    min-width: 0;
  }

  .col:last-child {
    border-right: none;
  }

  .col-head {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 6px;
    width: 100%;
    padding: var(--space-2) 0;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .col-head:hover {
    background: var(--surface-hover);
  }

  .col.selected .col-head {
    background: var(--accent-soft);
  }

  .dow {
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .dom {
    font-weight: 600;
  }

  .dom.mark {
    color: var(--accent);
  }

  .allday {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 8px;
    padding: 2px 4px;
    border-bottom: 1px solid var(--border);
  }

  .ev {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    border: none;
    border-left: 3px solid var(--accent);
    border-radius: var(--radius-s);
    background: var(--accent-soft);
    color: var(--text-primary);
    font-size: var(--font-size-s);
    padding: 1px 5px;
    cursor: default;
  }

  .ev.all {
    cursor: pointer;
  }

  .ev.linked {
    border-left-color: var(--ok);
  }

  .grid {
    position: relative;
  }

  .slot {
    position: absolute;
    left: 0;
    right: 0;
    border: none;
    border-top: 1px solid var(--border);
    background: transparent;
    cursor: pointer;
  }

  .slot:hover {
    background: var(--accent-soft);
  }

  .now-line {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--error);
    pointer-events: none;
    z-index: 2;
  }

  .ev.timed {
    position: absolute;
    left: 3px;
    right: 3px;
    z-index: 1;
    pointer-events: none;
  }

  .agenda {
    margin-top: var(--space-4);
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
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
    gap: var(--space-2);
  }

  .agenda li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
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
  }

  .del:hover {
    color: var(--error);
    background: var(--surface-hover);
  }
</style>
