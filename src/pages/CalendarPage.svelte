<script lang="ts">
  import { onMount } from "svelte";
  import { ChevronLeft, ChevronRight, Pencil, Plus, X } from "@lucide/svelte";
  import SjtuSidebar from "../components/SjtuSidebar.svelte";
  import {
    createEvent,
    deleteEvent,
    exportCalendarIcs,
    getSetting,
    listEventsRange,
    listTasks,
    setSetting,
    updateEvent,
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
  const HOUR_H = 44; // px per hour in the week grid

  const WEEKDAYS = ["一", "二", "三", "四", "五", "六", "日"] as const;
  const HOURS = Array.from({ length: 24 }, (_, i) => i);
  const DURATIONS = [15, 30, 45, 60, 90, 120, 180, 240] as const;

  let events = $state<CalendarEvent[]>([]);
  let tasks = $state<Task[]>([]);
  // Anchor day inside the displayed week/month (any day works; the rest is
  // derived). `view` is restored from settings in onMount.
  let anchor = $state(startOfDay(new Date()));
  let selectedDay = $state(startOfDay(new Date()));
  let view = $state<"week" | "month">("week");
  // Id of the local event shown in the editor / highlighted in the grid.
  let selectedEventId = $state<number | null>(null);

  // Create/edit form. `start` is a datetime-local string; when allDay only
  // its date part matters and `dur` is ignored (the event spans one day).
  interface FormState {
    mode: "create" | "edit";
    id: number | null;
    title: string;
    start: string;
    dur: number;
    allDay: boolean;
    taskId: number | null;
    notes: string;
  }
  let form = $state<FormState | null>(null);

  // Week-view drag-create: pressing on the time grid and moving spans hours;
  // releasing opens the creator with the pressed range. Hours are inclusive
  // start / exclusive end, clamped to [0, 24].
  interface DragSel {
    day: Date;
    from: number;
    to: number;
  }
  let dragSel = $state<DragSel | null>(null);
  // Hover preview while the pointer glides over the time grid.
  let hoverSel = $state<DragSel | null>(null);

  // Auto-scroll target inside today's column (current hour − 2).
  let scrollAnchor = $state<HTMLDivElement | undefined>(undefined);

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

  // Month view: 42 cells starting from the Monday on/before the 1st.
  const monthStart = $derived(new Date(anchor.getFullYear(), anchor.getMonth(), 1));
  const monthCells = $derived.by(() => {
    const first = weekStart(monthStart);
    return Array.from({ length: 42 }, (_, i) => new Date(first.getTime() + i * DAY_MS));
  });
  const monthLabel = $derived(`${monthStart.getFullYear()}年${monthStart.getMonth() + 1}月`);
  const headLabel = $derived(view === "week" ? rangeLabel : monthLabel);

  function isToday(d: Date): boolean {
    return startOfDay(new Date()).getTime() === d.getTime();
  }

  /**
   * One renderable entry for a calendar day: local events and the read-only
   * SJTU projection merged. `kind` drives the color and whether the day
   * panel offers editing.
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

  /** Per-day render model for the week grid (items computed once per day). */
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

  onMount(() => {
    void restoreView().then(() => {
      reload();
      // Bring the work hours into view: scroll the today column marker
      // (current hour − 2) toward the vertical center, once.
      requestAnimationFrame(() => {
        scrollAnchor?.scrollIntoView({ block: "center" });
      });
    });
    void loadSjtu();
    const stopReminder = startSjtuReminder();
    // Keyboard: ←/→ moves a week or a month, T jumps back to today —
    // never while the user is typing in a field.
    const onKey = (e: KeyboardEvent) => {
      const t = e.target;
      if (
        t instanceof HTMLElement &&
        (t.tagName === "INPUT" ||
          t.tagName === "TEXTAREA" ||
          t.tagName === "SELECT" ||
          t.isContentEditable)
      ) {
        return;
      }
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        shiftBack();
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        shiftForward();
      } else if (e.key === "t" || e.key === "T") {
        e.preventDefault();
        goToday();
      }
    };
    window.addEventListener("keydown", onKey);
    let unlisten: (() => void) | undefined;
    let unlistenClosed: (() => void) | undefined;
    void watchSjtuSynced().then((un) => (unlisten = un));
    void watchSjtuWindowClosed().then((un) => (unlistenClosed = un));
    return () => {
      window.removeEventListener("keydown", onKey);
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
    requestAnimationFrame(() => scrollAnchor?.scrollIntoView({ block: "center" }));
  }

  function toLocalInput(ms: number): string {
    const d = new Date(ms);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}T${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  }

  /** Open the create form; `hour`/`dur` come from a grid click or drag. */
  function openCreator(d: Date, hour: number | null, dur = 60) {
    selectedDay = d;
    const base = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const h = hour === null ? 9 : hour;
    form = {
      mode: "create",
      id: null,
      title: "",
      start: `${base}T${String(h).padStart(2, "0")}:00`,
      dur,
      allDay: false,
      taskId: null,
      notes: "",
    };
  }

  function openAllDayCreator(d: Date) {
    selectedDay = d;
    const base = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    form = {
      mode: "create",
      id: null,
      title: "",
      start: `${base}T00:00`,
      dur: 60,
      allDay: true,
      taskId: null,
      notes: "",
    };
  }

  /** Open the edit form for a local event (from a grid chip or the panel). */
  function openEditor(item: DayItem) {
    if (item.kind !== "local") return;
    const ev = events.find((e) => e.id === item.id);
    if (!ev) return;
    selectedDay = startOfDay(new Date(ev.startsAt));
    selectedEventId = ev.id;
    form = {
      mode: "edit",
      id: ev.id,
      title: ev.title,
      start: toLocalInput(ev.startsAt),
      dur: Math.max(15, Math.round((ev.endsAt - ev.startsAt) / 60_000)),
      allDay: ev.allDay,
      taskId: ev.taskId,
      notes: ev.notes ?? "",
    };
  }

  function closeForm() {
    form = null;
  }

  async function submitForm() {
    if (!form) return;
    const draft = form;
    const title = draft.title.trim();
    if (!title) {
      form = null;
      return;
    }
    let startMs = new Date(draft.start).getTime();
    if (draft.allDay) {
      const d = new Date(draft.start);
      startMs = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
    }
    if (Number.isNaN(startMs)) {
      pushToast("error", "开始时间无效");
      return;
    }
    const endMs = draft.allDay ? startMs + DAY_MS : startMs + draft.dur * 60_000;
    try {
      if (draft.mode === "create") {
        await createEvent(title, startMs, endMs, draft.allDay, {
          taskId: draft.taskId,
          notes: draft.notes.trim() || null,
        });
        pushToast("ok", "日程已创建");
      } else if (draft.id !== null) {
        await updateEvent(draft.id, title, startMs, endMs, draft.allDay, {
          taskId: draft.taskId,
          notes: draft.notes.trim() || null,
        });
        pushToast("ok", "日程已更新");
      }
      form = null;
      await reload();
    } catch (err) {
      pushToast("error", `保存失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function removeFormEvent() {
    if (!form || form.mode !== "edit" || form.id === null) return;
    const id = form.id;
    form = null;
    selectedEventId = null;
    try {
      await deleteEvent(id);
      await reload();
      pushToast("ok", "日程已删除");
    } catch (err) {
      pushToast("error", `删除失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function remove(item: DayItem) {
    if (item.kind !== "local") return;
    try {
      await deleteEvent(item.id);
      if (selectedEventId === item.id) selectedEventId = null;
      if (form?.mode === "edit" && form.id === item.id) form = null;
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

  // --- Week-grid drag-create (pointer based; slot divs are paint-only) ----

  function hourFromEvent(e: PointerEvent, el: HTMLElement): number {
    const rect = el.getBoundingClientRect();
    const raw = ((e.clientY - rect.top) / HOUR_H) | 0;
    return Math.min(23, Math.max(0, raw));
  }

  function gridPointerDown(e: PointerEvent, d: Date, el: HTMLElement) {
    if (e.button !== 0) return;
    e.preventDefault();
    const from = hourFromEvent(e, el);
    dragSel = { day: d, from, to: from + 1 };
    hoverSel = null;
    const onMove = (ev: PointerEvent) => {
      const hour = hourFromEvent(ev, el);
      if (dragSel) {
        dragSel = {
          day: d,
          from: Math.min(from, hour),
          to: Math.max(from + 1, hour + 1),
        };
      }
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      const sel = dragSel;
      dragSel = null;
      if (!sel) return;
      openCreator(sel.day, sel.from, (sel.to - sel.from) * 60);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  function gridHover(e: PointerEvent, d: Date, el: HTMLElement) {
    if (dragSel) return;
    const hour = hourFromEvent(e, el);
    if (!hoverSel || hoverSel.day.getTime() !== d.getTime() || hoverSel.from !== hour) {
      hoverSel = { day: d, from: hour, to: hour + 1 };
    }
  }

  function gridLeave(d: Date) {
    if (hoverSel && hoverSel.day.getTime() === d.getTime()) hoverSel = null;
  }
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

    <div class="page-actions">
      <button type="button" class="btn primary" onclick={() => openCreator(selectedDay, 9)}>
        <Plus size={14} /> 新建日程
      </button>
      <button type="button" class="btn" onclick={() => void onExportIcs()}>导出 ICS</button>
    </div>

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
                class="ev"
                class:sjtu={e.kind === "sjtu"}
                class:active={selectedEventId === e.id}
                title={e.kind === "local" ? "点击编辑" : e.title}
                onclick={() => (e.kind === "local" ? openEditor(e) : (selectedDay = day.d))}
              >
                {e.title}
              </button>
            {/each}
            <button
              type="button"
              class="allday-add"
              title="新建全天日程"
              onclick={() => openAllDayCreator(day.d)}
            >
              <Plus size={12} />
            </button>
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
            role="presentation"
            onpointerdown={(e) => gridPointerDown(e, day.d, e.currentTarget as HTMLElement)}
            onpointermove={(e) => gridHover(e, day.d, e.currentTarget as HTMLElement)}
            onpointerleave={() => gridLeave(day.d)}
          >
            {#each HOURS as h (h)}
              <div class="slot" style={`top: ${h * HOUR_H}px; height: ${HOUR_H}px`}></div>
            {/each}
            {#if dragSel && dragSel.day.getTime() === day.d.getTime()}
              <div
                class="drag-range"
                style={`top: ${dragSel.from * HOUR_H}px; height: ${(dragSel.to - dragSel.from) * HOUR_H}px`}
              >
                {timeOf(day.d.getTime() + dragSel.from * 3_600_000)} –
                {timeOf(day.d.getTime() + dragSel.to * 3_600_000)}
              </div>
            {:else if hoverSel && hoverSel.day.getTime() === day.d.getTime()}
              <div
                class="hover-range"
                style={`top: ${hoverSel.from * HOUR_H}px; height: ${HOUR_H}px`}
              ></div>
            {/if}
            {#if day.today}
              <div class="now-line" style={`top: ${(nowPct / 100) * 24 * HOUR_H}px`}></div>
              <div
                class="scroll-anchor"
                bind:this={scrollAnchor}
                style={`top: ${Math.max(0, new Date().getHours() - 2) * HOUR_H}px`}
                aria-hidden="true"
              ></div>
            {/if}
            {#each day.timed as e (e.key)}
              {@const s = Math.max(e.startsAt, day.d.getTime())}
              {@const en = Math.min(e.endsAt, day.d.getTime() + DAY_MS)}
              {@const top = ((new Date(s).getHours() * 60 + new Date(s).getMinutes()) / 1440) * 24 * HOUR_H}
              {@const height = Math.max(((en - s) / DAY_MS) * 24 * HOUR_H, 22)}
              <div
                class="ev timed"
                class:linked={e.taskId !== null}
                class:sjtu={e.kind === "sjtu"}
                class:active={selectedEventId === e.id}
                style={`top: ${top}px; height: ${height}px`}
                title={e.kind === "local" ? `${timeOf(e.startsAt)}–${timeOf(e.endsAt)} ${e.title}（点击编辑）` : `${timeOf(e.startsAt)}–${timeOf(e.endsAt)} ${e.title}`}
                role="button"
                tabindex="0"
                onpointerdown={(pe) => pe.stopPropagation()}
                onclick={(ce) => {
                  ce.stopPropagation();
                  if (ce.detail > 1) return;
                  if (e.kind === "local") openEditor(e);
                }}
                onkeydown={(ke) => {
                  if (ke.key === "Enter" && e.kind === "local") {
                    ke.stopPropagation();
                    openEditor(e);
                  }
                }}
              >
                {#if height >= 34}
                  <span class="ev-time">{timeOf(e.startsAt)}</span>
                {/if}
                <span class="ev-title">{e.title}</span>
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
              ondblclick={() => openAllDayCreator(c)}
            >
              <span class="num">{c.getDate()}</span>
              <span class="chips">
                {#each evs.slice(0, 3) as e (e.key)}
                  <span
                    class="chip"
                    class:linked={e.kind === "local" && e.taskId !== null}
                    class:sjtu={e.kind === "sjtu"}
                    title={e.kind === "local" ? "点击日期后在右侧编辑" : `${e.allDay ? "全天" : timeOf(e.startsAt)} ${e.title}`}
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
  </div>

  <div class="side">
    <section class="day-panel glass" aria-label="当日日程">
      <header class="panel-head">
        <h2>
          {selectedDay.getMonth() + 1}月{selectedDay.getDate()}日
          周{WEEKDAYS[(selectedDay.getDay() + 6) % 7]}
        </h2>
        <button
          type="button"
          class="icon-btn"
          title="新建日程"
          aria-label="新建日程"
          onclick={() => openCreator(selectedDay, 9)}
        >
          <Plus size={14} />
        </button>
      </header>

      {#if form}
        <div class="form">
          <input
            class="form-title"
            type="text"
            placeholder="日程标题"
            bind:value={form.title}
            maxlength="60"
            aria-label="日程标题"
          />
          <label class="check">
            <input type="checkbox" bind:checked={form.allDay} />
            全天
          </label>
          <div class="form-row">
            <label>
              开始
              <input type="datetime-local" bind:value={form.start} disabled={form.allDay} />
            </label>
            <label>
              时长
              <select bind:value={form.dur} disabled={form.allDay}>
                {#each DURATIONS as d (d)}
                  <option value={d}>{d < 60 ? `${d} 分钟` : d % 60 === 0 ? `${d / 60} 小时` : `${(d / 60).toFixed(1)} 小时`}</option>
                {/each}
              </select>
            </label>
          </div>
          <label class="form-row">
            关联任务
            <select bind:value={form.taskId}>
              <option value={null}>不关联</option>
              {#each tasks.filter((t) => t.status !== "done") as t (t.id)}
                <option value={t.id}>{t.title}</option>
              {/each}
            </select>
          </label>
          <textarea
            rows="2"
            placeholder="备注（可选）"
            bind:value={form.notes}
            aria-label="备注"
          ></textarea>
          <div class="form-actions">
            {#if form.mode === "edit"}
              <button type="button" class="danger" onclick={() => void removeFormEvent()}>
                删除
              </button>
            {/if}
            <span class="form-spacer"></span>
            <button type="button" class="btn" onclick={closeForm}>取消</button>
            <button type="button" class="btn primary" onclick={() => void submitForm()}>
              {form.mode === "create" ? "创建" : "保存"}
            </button>
          </div>
        </div>
      {:else if dayItems(selectedDay).length === 0}
        <p class="empty">
          {#if view === "week"}
            这一天还没有安排——在左侧时间格上按下并拖动即可新建
          {:else}
            这一天还没有安排——双击日期即可新建
          {/if}
        </p>
      {:else}
        <ul class="day-list">
          {#each dayItems(selectedDay) as e (e.key)}
            <li class:active={selectedEventId === e.id} class:sjtu={e.kind === "sjtu"}>
              <span class="when">
                {e.allDay ? "全天" : `${timeOf(e.startsAt)}–${timeOf(e.endsAt)}`}
              </span>
              <span class="what" title={e.title}>
                {e.title}
                {#if e.location}
                  <span class="loc">· {e.location}</span>
                {/if}
                {#if e.kind === "local" && taskTitle(e.taskId)}
                  <span class="task-ref">· {taskTitle(e.taskId)}</span>
                {/if}
              </span>
              {#if e.kind === "local"}
                <button
                  type="button"
                  class="row-btn"
                  title="编辑日程"
                  aria-label="编辑日程：{e.title}"
                  onclick={() => openEditor(e)}
                >
                  <Pencil size={12} />
                </button>
                <button
                  type="button"
                  class="row-btn"
                  title="删除日程"
                  aria-label="删除日程：{e.title}"
                  onclick={() => void remove(e)}
                >
                  <X size={12} />
                </button>
              {:else}
                <span class="sjtu-tag" title="来自交大日历，只读">交大</span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <SjtuSidebar />
  </div>
</div>

<style>
  .calendar-layout {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    max-width: 1280px;
    margin: 0 auto;
  }

  .calendar {
    flex: 1;
    min-width: 0;
  }

  .side {
    width: 300px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  @media (max-width: 1120px) {
    .calendar-layout {
      flex-direction: column;
      align-items: stretch;
    }
    .side {
      width: auto;
    }
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: var(--space-3);
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
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out),
      transform var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
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

  /* --- Week grid: time gutter + 7 day columns × header/all-day/time rows -- */

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

  .allday-add {
    display: grid;
    place-items: center;
    width: 100%;
    min-height: 14px;
    padding: 0;
    border: 1px dashed transparent;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    opacity: 0;
    transition: opacity var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out),
      border-color var(--duration-fast) var(--ease-out);
  }

  .allday:hover .allday-add,
  .allday-add:focus-visible {
    opacity: 1;
  }

  .allday-add:hover {
    color: var(--accent);
    border-color: var(--border-strong);
  }

  .ev {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
    border: none;
    border-left: 3px solid var(--accent);
    border-radius: 6px;
    background: color-mix(in srgb, var(--accent) 16%, var(--surface));
    color: var(--text-primary);
    font-size: var(--font-size-s);
    padding: 2px 6px;
    cursor: pointer;
    box-shadow: var(--shadow-sm);
    transition: box-shadow var(--duration-fast) var(--ease-out),
      filter var(--duration-fast) var(--ease-out);
  }

  .ev:hover {
    filter: brightness(1.08);
  }

  .ev.active {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .ev.linked {
    border-left-color: var(--ok);
  }

  /* SJTU-synced entries: read-only, warning-amber, tagged in the panel. */
  .ev.sjtu {
    border-left-color: var(--warn);
    background: color-mix(in srgb, var(--warn) 18%, var(--surface));
  }

  .grid {
    position: relative;
    grid-row: 3;
    border-right: 1px solid var(--border);
    cursor: crosshair;
    touch-action: none;
    user-select: none;
  }

  .slot {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px solid var(--border);
    pointer-events: none;
  }

  .hover-range {
    position: absolute;
    left: 2px;
    right: 2px;
    border-radius: var(--radius-s);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    pointer-events: none;
  }

  .drag-range {
    position: absolute;
    left: 2px;
    right: 2px;
    z-index: 3;
    display: grid;
    place-items: center start;
    padding-left: 8px;
    border-radius: var(--radius-s);
    border: 1px dashed var(--accent);
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent);
    font-size: 11px;
    font-family: var(--font-mono);
    pointer-events: none;
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

  .scroll-anchor {
    position: absolute;
    width: 1px;
    height: 1px;
    pointer-events: none;
  }

  .ev.timed {
    position: absolute;
    left: 3px;
    right: 3px;
    z-index: 1;
    display: flex;
    align-items: baseline;
    gap: 4px;
    line-height: 1.4;
  }

  .ev-time {
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: 10px;
    opacity: 0.75;
  }

  .ev-title {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* --- Month view ---------------------------------------------------------- */

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

  /* --- Day panel (right column) -------------------------------------------- */

  .day-panel {
    padding: var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow-sm);
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-3);
  }

  .panel-head h2 {
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

  .icon-btn:hover {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }

  .empty {
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
    line-height: 1.6;
  }

  .day-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .day-list li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px 8px;
    border-radius: var(--radius-m);
    transition: background var(--duration-fast) var(--ease-out);
  }

  .day-list li:hover {
    background: var(--surface-hover);
  }

  .day-list li.active {
    background: var(--accent-soft);
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

  .loc {
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }

  .task-ref {
    color: var(--ok);
    font-size: var(--font-size-s);
  }

  .row-btn {
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
    flex-shrink: 0;
    transition: color var(--duration-fast) var(--ease-out),
      background var(--duration-fast) var(--ease-out);
  }

  .row-btn:hover {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .row-btn:last-of-type:hover {
    color: var(--error);
    background: color-mix(in srgb, var(--error) 10%, transparent);
  }

  .sjtu-tag {
    display: inline-block;
    padding: 0 5px;
    border-radius: var(--radius-s);
    background: color-mix(in srgb, var(--warn) 18%, transparent);
    color: var(--warn);
    font-size: 11px;
    flex-shrink: 0;
  }

  /* --- Editor form ---------------------------------------------------------- */

  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    font-size: var(--font-size-s);
  }

  .form-title {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-primary);
    outline: none;
    font-size: var(--font-size-m);
  }

  .form-title::placeholder {
    color: var(--text-tertiary);
  }

  .check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
  }

  .form-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
    flex-wrap: wrap;
  }

  .form input[type="datetime-local"],
  .form select,
  .form textarea {
    border: 1px solid var(--border);
    border-radius: var(--radius-s);
    background: var(--surface);
    color: var(--text-primary);
    padding: 5px 7px;
    font: inherit;
    outline: none;
  }

  .form textarea {
    resize: vertical;
    width: 100%;
  }

  .form-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .form-spacer {
    flex: 1;
  }

  .danger {
    padding: 5px 12px;
    border: 1px solid color-mix(in srgb, var(--error) 35%, transparent);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--error);
    cursor: pointer;
    font-size: var(--font-size-s);
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .danger:hover {
    background: color-mix(in srgb, var(--error) 12%, transparent);
  }

  /* --- View toggle ----------------------------------------------------------- */

  .view-toggle {
    display: inline-flex;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: color-mix(in srgb, var(--surface-active) 65%, transparent);
  }

  .view-toggle button {
    padding: 4px 12px;
    border: none;
    border-radius: calc(var(--radius-m) - 2px);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out), box-shadow var(--duration-fast) var(--ease-out);
  }

  .view-toggle button.active {
    background: var(--surface);
    color: var(--accent);
    font-weight: 600;
    box-shadow: var(--shadow-sm);
  }
</style>
