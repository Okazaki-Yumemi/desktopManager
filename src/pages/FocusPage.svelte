<script lang="ts">
  import { onMount } from "svelte";
  import { Check, Coffee, Flag, Layers, Play, Timer, X } from "@lucide/svelte";
  import {
    finishFocus,
    getFocusSummary,
    getRunningFocus,
    getSetting,
    interruptFocus,
    listFocusSessions,
    listScenes,
    setFocusNote,
    setSetting,
    startFocus,
  } from "../services/backend";
  import type { FocusDay, FocusKind, FocusSession, Scene } from "../types/domain";
  import { pushToast } from "../stores/toast.svelte";

  interface Preset {
    id: string;
    label: string;
    kind: FocusKind;
    focusS: number;
    breakS: number;
  }

  const PRESETS: Preset[] = [
    { id: "p25", label: "番茄 25/5", kind: "pomodoro", focusS: 25 * 60, breakS: 5 * 60 },
    { id: "p50", label: "深度 50/10", kind: "pomodoro", focusS: 50 * 60, breakS: 10 * 60 },
    { id: "countup", label: "正计时", kind: "count_up", focusS: 0, breakS: 0 },
    { id: "custom", label: "自定义", kind: "custom", focusS: 0, breakS: 0 },
  ];

  // idle → focus (DB-backed session) → break (UI-only, not persisted).
  let phase = $state<"idle" | "focus" | "break">("idle");
  let presetId = $state("p25");
  let session = $state<FocusSession | null>(null);
  let now = $state(Date.now());
  let breakEndsAt = $state(0);
  let breakTotalS = $state(0);
  let autoFinishing = false;

  let customFocusMin = $state(30);
  let customBreakMin = $state(5);
  let scenes = $state<Scene[]>([]);
  let boundSceneId = $state<number | null>(null);
  let noteDraft = $state("");

  let dayList = $state<FocusSession[]>([]);
  let summary = $state<FocusDay[]>([]);
  let busy = $state(false);

  const FALLBACK_PRESET: Preset = PRESETS[0]!;
  const preset = $derived(PRESETS.find((p) => p.id === presetId) ?? FALLBACK_PRESET);
  // Idle preview of the selected preset; while running the DB session is
  // authoritative (a recovered session may come from another preset).
  const previewPlannedS = $derived(
    preset.kind === "count_up"
      ? 0
      : preset.kind === "custom"
        ? Math.max(1, customFocusMin) * 60
        : preset.focusS,
  );
  const previewBreakS = $derived(
    preset.kind === "count_up"
      ? 0
      : preset.kind === "custom"
        ? Math.max(0, customBreakMin) * 60
        : preset.breakS,
  );
  const plannedS = $derived(phase === "focus" && session ? session.plannedDurationS : previewPlannedS);
  const breakS = $derived(previewBreakS);
  const elapsedS = $derived(
    phase === "focus" && session
      ? Math.floor((now - session.startedAt) / 1000)
      : phase === "break"
        ? Math.max(0, Math.ceil((breakEndsAt - now) / 1000))
        : 0,
  );
  const overS = $derived(
    phase === "focus" && plannedS > 0 ? Math.max(0, elapsedS - plannedS) : 0,
  );
  const displayS = $derived(
    phase === "focus" && plannedS > 0 && elapsedS <= plannedS ? plannedS - elapsedS : elapsedS,
  );
  const progress = $derived(
    phase === "break"
      ? breakTotalS > 0
        ? 1 - elapsedS / breakTotalS
        : 0
      : plannedS > 0
        ? Math.min(1, elapsedS / plannedS)
        : 0,
  );
  const maxTotalS = $derived(Math.max(60, ...summary.map((d) => d.totalS)));

  function fmt(total: number): string {
    const s = Math.max(0, Math.floor(total));
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const ss = `${s % 60}`.padStart(2, "0");
    const mm = `${m}`.padStart(2, "0");
    return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
  }

  function localDay(d: Date): string {
    const mm = `${d.getMonth() + 1}`.padStart(2, "0");
    const dd = `${d.getDate()}`.padStart(2, "0");
    return `${d.getFullYear()}-${mm}-${dd}`;
  }

  function timeOf(ms: number): string {
    const d = new Date(ms);
    return `${`${d.getHours()}`.padStart(2, "0")}:${`${d.getMinutes()}`.padStart(2, "0")}`;
  }

  const KIND_LABEL: Record<string, string> = {
    pomodoro: "番茄钟",
    custom: "自定义",
    count_up: "正计时",
  };
  const STATUS_LABEL: Record<string, string> = {
    running: "进行中",
    completed: "已完成",
    interrupted: "已打断",
    abandoned: "已放弃",
  };

  function errText(err: unknown): string {
    return typeof err === "string" ? err : err instanceof Error ? err.message : "操作失败";
  }

  function maybeAutoAdvance(): void {
    if (phase === "focus" && session && session.plannedDurationS > 0 && !autoFinishing) {
      if (elapsedS >= session.plannedDurationS) {
        autoFinishing = true;
        void endFocus("completed", true);
      }
    } else if (phase === "break" && breakEndsAt > 0 && now >= breakEndsAt) {
      phase = "idle";
      breakEndsAt = 0;
      pushToast("info", "休息结束，随时可以开始下一段专注");
    }
  }

  onMount(() => {
    const timer = setInterval(() => {
      now = Date.now();
      maybeAutoAdvance();
    }, 500);
    void init();
    return () => clearInterval(timer);
  });

  async function init(): Promise<void> {
    try {
      const saved = await getSetting<string>("focus.preset");
      if (saved && PRESETS.some((p) => p.id === saved)) presetId = saved;
    } catch {
      /* first run: keep the default preset */
    }
    try {
      scenes = await listScenes();
    } catch {
      scenes = [];
    }
    // Recovery: a session row with status=running keeps counting from
    // started_at even if the app was closed or the page was reloaded.
    try {
      const running = await getRunningFocus();
      if (running) {
        session = running;
        noteDraft = running.note ?? "";
        boundSceneId = running.sceneId;
        autoFinishing = false;
        phase = "focus";
      }
    } catch {
      /* backend unavailable: stay idle */
    }
    await refreshHistory();
  }

  async function refreshHistory(): Promise<void> {
    const day = localDay(new Date());
    try {
      dayList = await listFocusSessions(day);
    } catch {
      dayList = [];
    }
    try {
      summary = await getFocusSummary(7);
    } catch {
      summary = [];
    }
  }

  async function start(): Promise<void> {
    if (busy || phase !== "idle") return;
    busy = true;
    try {
      await setSetting("focus.preset", presetId);
      const s = await startFocus(preset.kind, previewPlannedS, null, boundSceneId);
      session = s;
      noteDraft = "";
      autoFinishing = false;
      phase = "focus";
      pushToast("info", `开始专注（${preset.label}）`);
    } catch (err) {
      pushToast("error", errText(err));
    } finally {
      busy = false;
    }
  }

  async function endFocus(status: "completed" | "abandoned", auto = false): Promise<void> {
    if (!session) return;
    const sessionId = session.id;
    const kind = session.kind;
    try {
      const done = await finishFocus(sessionId, status);
      if (status === "completed") {
        pushToast("ok", `专注完成，实际 ${fmt(done.actualDurationS)}`);
        if (kind !== "count_up" && breakS > 0) {
          breakTotalS = breakS;
          breakEndsAt = Date.now() + breakS * 1000;
          phase = "break";
        } else {
          phase = "idle";
        }
      } else {
        pushToast("info", `已放弃本次专注（记录 ${fmt(done.actualDurationS)}）`);
        phase = "idle";
      }
      session = null;
      await refreshHistory();
    } catch (err) {
      pushToast("error", errText(err));
      // The session may have been finished elsewhere: resync the UI.
      phase = "idle";
      session = null;
      void refreshHistory();
    }
    if (auto) autoFinishing = false;
  }

  async function onInterrupt(): Promise<void> {
    if (!session || busy) return;
    busy = true;
    try {
      const s = await interruptFocus(session.id);
      session = s;
      pushToast("info", `已记录一次打断（共 ${s.interruptions} 次）`);
    } catch (err) {
      pushToast("error", errText(err));
    } finally {
      busy = false;
    }
  }

  async function saveNote(): Promise<void> {
    if (!session) return;
    try {
      await setFocusNote(session.id, noteDraft);
    } catch (err) {
      pushToast("error", errText(err));
    }
  }

  /** Soft scene integration: switching is one click and never forced. */
  async function applyBoundScene(): Promise<void> {
    if (!session?.sceneId) return;
    const sceneId = session.sceneId;
    try {
      await setSetting("ui.activeScene", sceneId);
      const name = scenes.find((s) => s.id === sceneId)?.name ?? "";
      pushToast("ok", `已切换到场景「${name}」，回到桌面页即可看到`);
    } catch (err) {
      pushToast("error", errText(err));
    }
  }

  function skipBreak(): void {
    phase = "idle";
    breakEndsAt = 0;
  }
</script>

<div class="page">
  <header class="head">
    <h1>专注</h1>
    <p class="sub">番茄钟 / 自定义 / 正计时。会话写入本地数据库，重启后自动恢复计时。</p>
  </header>

  <section
    class="timer-card"
    class:is-focus={phase === "focus"}
    class:is-break={phase === "break"}
  >
    <div class="phase-label">
      {#if phase === "idle"}
        待开始
      {:else if phase === "focus"}
        <Timer size={15} /> 专注中
      {:else}
        <Coffee size={15} /> 休息中
      {/if}
    </div>
    <div class="clock" class:over={overS > 0}>{fmt(displayS)}</div>

    {#if phase === "focus" && plannedS > 0}
      <div class="progress"><div class="bar" style={`width: ${(progress * 100).toFixed(1)}%`}></div></div>
      <div class="plan-note">计划 {fmt(plannedS)}{session?.kind === "count_up" ? "" : ` · 剩余 ${fmt(Math.max(0, plannedS - elapsedS))}`}</div>
      {#if overS > 0}
        <div class="over-note">已超过计划时长 —— 请选择“完成”或“放弃”</div>
      {/if}
    {:else if phase === "break"}
      <div class="progress"><div class="bar rest" style={`width: ${(progress * 100).toFixed(1)}%`}></div></div>
      <button type="button" class="ghost" onclick={skipBreak}>跳过休息</button>
    {:else}
      <div class="plan-note">本次专注 {previewPlannedS > 0 ? fmt(previewPlannedS) : "不限时"}{previewBreakS > 0 ? ` · 休息 ${fmt(previewBreakS)}` : ""}</div>
    {/if}

    {#if phase === "idle"}
      <div class="chips">
        {#each PRESETS as p (p.id)}
          <button
            type="button"
            class="chip"
            class:active={p.id === presetId}
            onclick={() => (presetId = p.id)}
          >
            {p.label}
          </button>
        {/each}
      </div>
      {#if preset.kind === "custom"}
        <div class="custom-row">
          专注 <input type="number" min="1" max="240" bind:value={customFocusMin} /> 分钟
          <span class="gap"></span>
          休息 <input type="number" min="0" max="60" bind:value={customBreakMin} /> 分钟
        </div>
      {/if}
      {#if scenes.length > 0}
        <label class="scene-row">
          绑定场景
          <select bind:value={boundSceneId}>
            <option value={null}>不绑定</option>
            {#each scenes as s (s.id)}
              <option value={s.id}>{s.name}</option>
            {/each}
          </select>
        </label>
      {/if}
      <button type="button" class="primary start-btn" disabled={busy} onclick={start}>
        <Play size={16} /> 开始专注
      </button>
    {:else if phase === "focus" && session}
      <div class="controls">
        <button type="button" class="ghost" disabled={busy} onclick={onInterrupt}>
          <Flag size={14} /> 打断{session.interruptions > 0 ? `（${session.interruptions}）` : ""}
        </button>
        <button type="button" class="primary" disabled={busy} onclick={() => void endFocus("completed")}>
          <Check size={14} /> 完成
        </button>
        <button type="button" class="danger" disabled={busy} onclick={() => void endFocus("abandoned")}>
          <X size={14} /> 放弃
        </button>
      </div>
      {#if session.sceneId}
        <div class="scene-bound">
          场景：{scenes.find((s) => s.id === session?.sceneId)?.name ?? session.sceneId}
          <button type="button" class="ghost" onclick={applyBoundScene}>
            <Layers size={13} /> 应用场景
          </button>
        </div>
      {/if}
      <textarea
        class="note"
        rows="2"
        placeholder="这半小时做了什么？（离开输入框即保存）"
        bind:value={noteDraft}
        onblur={saveNote}
      ></textarea>
    {/if}
  </section>

  <section class="card">
    <h2>今日记录</h2>
    {#if dayList.length === 0}
      <p class="empty">今天还没有专注记录。</p>
    {:else}
      {#each dayList as s (s.id)}
        <div class="row">
          <span class="when">
            {timeOf(s.startedAt)}–{s.endedAt ? timeOf(s.endedAt) : "…"}
          </span>
          <span class="kind">{KIND_LABEL[s.kind] ?? s.kind}</span>
          <span class="dur">{s.status === "running" ? "进行中" : fmt(s.actualDurationS)}</span>
          <span class={`badge ${s.status}`}>{STATUS_LABEL[s.status] ?? s.status}</span>
          {#if s.interruptions > 0}
            <span class="int">打断 {s.interruptions}</span>
          {/if}
          {#if s.note}
            <span class="note-text" title={s.note}>{s.note}</span>
          {/if}
        </div>
      {/each}
    {/if}
  </section>

  <section class="card">
    <h2>近 7 天</h2>
    {#if summary.length === 0}
      <p class="empty">还没有已完成的专注。</p>
    {:else}
      {#each summary as d (d.day)}
        <div class="sum-row">
          <span class="day">{d.day.slice(5)}</span>
          <span class="track">
            <span class="fill" style={`width: ${((d.totalS / maxTotalS) * 100).toFixed(1)}%`}></span>
          </span>
          <span class="sum-text">
            {Math.round(d.totalS / 60)} 分 · {d.sessions} 段{d.interruptions > 0 ? ` · 打断 ${d.interruptions}` : ""}
          </span>
        </div>
      {/each}
    {/if}
  </section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    max-width: 760px;
    margin: 0 auto;
  }

  .head h1 {
    margin: 0 0 var(--space-1);
    font-size: var(--font-size-xl);
  }
  .sub {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-size-s);
  }

  .timer-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-5) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    background: var(--surface);
    transition: border-color var(--duration-normal) var(--ease-out);
  }
  .timer-card.is-focus {
    border-color: var(--accent);
  }
  .timer-card.is-break {
    border-color: var(--ok);
  }

  .phase-label {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--text-secondary);
    font-size: var(--font-size-s);
  }

  .clock {
    font-family: var(--font-mono);
    font-size: 64px;
    line-height: 1.1;
    font-variant-numeric: tabular-nums;
  }
  .clock.over {
    color: var(--warn);
  }

  .progress {
    width: min(100%, 420px);
    height: 6px;
    border-radius: 999px;
    background: var(--bg);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .bar {
    height: 100%;
    background: var(--accent);
    transition: width var(--duration-normal) var(--ease-out);
  }
  .bar.rest {
    background: var(--ok);
  }

  .plan-note,
  .over-note {
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }
  .over-note {
    color: var(--warn);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    justify-content: center;
  }
  .chip {
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-primary);
    padding: 6px 14px;
    font-size: var(--font-size-s);
    cursor: pointer;
  }
  .chip:hover {
    background: var(--surface-hover);
  }
  .chip.active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .custom-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-s);
    color: var(--text-secondary);
  }
  .custom-row .gap {
    width: var(--space-3);
  }
  .custom-row input {
    width: 72px;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-s);
    background: var(--bg);
    color: var(--text-primary);
    font-size: var(--font-size-s);
  }

  .scene-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-s);
    color: var(--text-secondary);
  }
  .scene-row select {
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-s);
    background: var(--bg);
    color: var(--text-primary);
    font-size: var(--font-size-s);
  }

  .start-btn {
    min-width: 180px;
    justify-content: center;
  }

  button.primary,
  button.ghost,
  button.danger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border-radius: var(--radius-m);
    padding: 7px 14px;
    font-size: var(--font-size-s);
    cursor: pointer;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-primary);
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
  }
  button.ghost:hover,
  button.danger:hover {
    background: var(--surface-hover);
  }
  button.danger {
    color: var(--error);
  }
  button:disabled {
    opacity: 0.55;
    cursor: default;
  }

  .controls {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
    justify-content: center;
  }

  .scene-bound {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--font-size-s);
    color: var(--text-secondary);
  }
  .scene-bound button {
    padding: 4px 10px;
  }

  .note {
    width: min(100%, 520px);
    resize: vertical;
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--bg);
    color: var(--text-primary);
    font-size: var(--font-size-s);
    font-family: var(--font-ui);
  }

  .card {
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    padding: var(--space-4);
  }
  .card h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--font-size-m);
  }
  .empty {
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }

  .row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
    padding: var(--space-2) 0;
    border-top: 1px solid var(--border);
    font-size: var(--font-size-s);
  }
  .row:first-of-type {
    border-top: none;
  }
  .when {
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }
  .dur {
    font-family: var(--font-mono);
  }
  .badge {
    border-radius: 999px;
    padding: 1px 8px;
    font-size: var(--font-size-s);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }
  .badge.completed {
    color: var(--ok);
    border-color: var(--ok);
  }
  .badge.abandoned {
    color: var(--error);
    border-color: var(--error);
  }
  .badge.interrupted {
    color: var(--warn);
    border-color: var(--warn);
  }
  .badge.running {
    color: var(--accent);
    border-color: var(--accent);
  }
  .int {
    color: var(--warn);
  }
  .note-text {
    color: var(--text-tertiary);
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sum-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-1) 0;
    font-size: var(--font-size-s);
  }
  .day {
    width: 44px;
    color: var(--text-secondary);
    font-family: var(--font-mono);
  }
  .track {
    flex: 1;
    height: 10px;
    border-radius: 999px;
    background: var(--bg);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .sum-text {
    min-width: 150px;
    color: var(--text-secondary);
  }
</style>
