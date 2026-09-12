<script lang="ts">
  import { onMount } from "svelte";
  import {
    AlertTriangle,
    CheckCircle2,
    ClipboardList,
    Clock3,
    ExternalLink,
    KeyRound,
    RefreshCw,
    Trash2,
  } from "@lucide/svelte";
  import type { CanvasAssignment } from "../types/domain";
  import {
    CANVAS_BASE,
    CANVAS_HOST,
    canvasLastSyncAt,
    canvasSetupError,
    canvasSnapshot,
    canvasSyncing,
    canvasToken,
    loadCanvasState,
    openAssignmentLink,
    removeCanvasToken,
    saveCanvasToken,
    syncCanvas,
  } from "../stores/canvas.svelte";

  let tokenInput = $state("");
  let showSubmitted = $state(false);
  let courseFilter = $state<number | "all">("all");
  let now = $state(Date.now());

  const snap = $derived(canvasSnapshot());
  const hasToken = $derived(canvasToken() !== "");

  onMount(() => {
    void loadCanvasState();
    // Keep countdowns honest while the page is open.
    const timer = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(timer);
  });

  type GroupKey = "overdue" | "today" | "week" | "later";

  const GROUP_META: ReadonlyArray<{ key: GroupKey; label: string }> = [
    { key: "overdue", label: "已逾期" },
    { key: "today", label: "今天截止" },
    { key: "week", label: "未来 7 天" },
    { key: "later", label: "更远" },
  ];

  function dayStart(ms: number): number {
    const d = new Date(ms);
    return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  }

  function groupOf(dueAt: number, nowMs: number): GroupKey {
    const startToday = dayStart(nowMs);
    if (dueAt < startToday) return "overdue";
    if (dueAt < startToday + 86_400_000) return "today";
    if (dueAt < startToday + 7 * 86_400_000) return "week";
    return "later";
  }

  function fmtDue(dueAt: number): string {
    const d = new Date(dueAt);
    const hm = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
    const date = `${d.getMonth() + 1}月${d.getDate()}日`;
    return d.getFullYear() === new Date().getFullYear()
      ? `${date} ${hm}`
      : `${d.getFullYear()}年${date} ${hm}`;
  }

  function countdown(dueAt: number, nowMs: number): string {
    const diff = dueAt - nowMs;
    const abs = Math.abs(diff);
    const days = Math.floor(abs / 86_400_000);
    const hours = Math.floor((abs % 86_400_000) / 3_600_000);
    const mins = Math.floor((abs % 3_600_000) / 60_000);
    const span =
      days >= 1
        ? `${days} 天 ${hours} 小时`
        : hours >= 1
          ? `${hours} 小时 ${mins} 分`
          : `${mins} 分钟`;
    return diff >= 0 ? `还剩 ${span}` : `已逾期 ${span}`;
  }

  function fmtSyncedAt(ts: number | null): string {
    if (ts === null) return "尚未同步";
    const d = new Date(ts);
    const hm = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
    return `上次同步 ${d.getMonth() + 1}月${d.getDate()}日 ${hm}`;
  }

  /** Course names arrive as "（2025-2026-1）-XX101-1 高等数学" — drop the term prefix. */
  function shortCourse(name: string): string {
    const stripped = name.replace(/^\s*[（(][^（）()]*[）)]\s*[-—–]?\s*/, "");
    return stripped.length > 0 ? stripped : name;
  }

  function courseHue(name: string): number {
    let h = 0;
    for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) % 360;
    return h;
  }

  const visible = $derived.by(() => {
    const s = snap;
    if (!s) return [] as CanvasAssignment[];
    return s.assignments.filter(
      (a) =>
        (courseFilter === "all" || a.courseId === courseFilter) &&
        (showSubmitted || !a.submitted),
    );
  });

  const grouped = $derived.by(() => {
    const g: Record<GroupKey, CanvasAssignment[]> = {
      overdue: [],
      today: [],
      week: [],
      later: [],
    };
    for (const a of visible) g[groupOf(a.dueAt, now)].push(a);
    return g;
  });

  const coursesInUse = $derived.by(() => {
    const s = snap;
    if (!s) return [] as { id: number; name: string }[];
    return s.courses.filter((c) => s.assignments.some((a) => a.courseId === c.id));
  });

  const submittedCount = $derived(
    snap ? snap.assignments.filter((a) => a.submitted).length : 0,
  );

  function courseName(id: number): string {
    return snap?.courses.find((c) => c.id === id)?.name ?? `课程 ${id}`;
  }

  function onSubmitToken(e: SubmitEvent): void {
    e.preventDefault();
    void saveCanvasToken(tokenInput).then((ok) => {
      if (ok) tokenInput = "";
    });
  }
</script>

<div class="assign page-enter">
  <header class="head">
    <div>
      <h1>作业 &amp; DDLs</h1>
      <p class="sub">来自 Canvas（{CANVAS_HOST}）· 官方 API 只读同步</p>
    </div>
    {#if hasToken}
      <div class="head-actions">
        <span class="last-sync">{fmtSyncedAt(canvasLastSyncAt())}</span>
        <button
          type="button"
          class="btn"
          onclick={() => void syncCanvas()}
          disabled={canvasSyncing()}
        >
          <RefreshCw size={14} class={canvasSyncing() ? "spin" : ""} aria-hidden="true" />
          {canvasSyncing() ? "同步中…" : "立即同步"}
        </button>
        <button
          type="button"
          class="btn icon-only"
          title="清除令牌与本地缓存"
          aria-label="清除令牌与本地缓存"
          onclick={() => void removeCanvasToken()}
        >
          <Trash2 size={14} aria-hidden="true" />
        </button>
      </div>
    {/if}
  </header>

  {#if !hasToken}
    <section class="glass card" aria-label="连接 Canvas">
      <h2><KeyRound size={16} aria-hidden="true" /> 连接 Canvas</h2>
      <ol class="steps">
        <li>
          在
          <button type="button" class="link" onclick={() => openAssignmentLink(CANVAS_BASE)}>
            {CANVAS_HOST}
          </button>
          打开「账户 → 设置」，拉到「已批准的集成」点「+ 新建访问令牌」，用途随意填写（如
          desktop-manager），有效期选「无日期」最省心。
        </li>
        <li>把生成的令牌粘贴到下面。令牌只保存在本机数据库，应用只做只读查询，不会上传任何数据。</li>
      </ol>
      <form class="token-form" onsubmit={onSubmitToken}>
        <input
          type="password"
          placeholder="Canvas 访问令牌"
          bind:value={tokenInput}
          autocomplete="off"
          spellcheck="false"
          aria-label="Canvas 访问令牌"
        />
        <button
          type="submit"
          class="btn primary"
          disabled={canvasSyncing() || tokenInput.trim() === ""}
        >
          {canvasSyncing() ? "验证中…" : "连接并同步"}
        </button>
      </form>
      {#if canvasSetupError()}
        <p class="err"><AlertTriangle size={13} aria-hidden="true" /> {canvasSetupError()}</p>
      {/if}
    </section>
  {:else if !snap}
    <section class="glass card empty">
      <ClipboardList size={26} aria-hidden="true" />
      <p>
        {canvasSyncing()
          ? "正在从 Canvas 拉取作业与截止时间…"
          : "还没有同步过，点右上角「立即同步」拉取作业。"}
      </p>
    </section>
  {:else}
    <div class="toolbar">
      {#if coursesInUse.length > 1}
        <div class="chips" role="group" aria-label="按课程筛选">
          <button
            type="button"
            class="chip-filter"
            class:active={courseFilter === "all"}
            onclick={() => (courseFilter = "all")}
          >
            全部
          </button>
          {#each coursesInUse as c (c.id)}
            <button
              type="button"
              class="chip-filter"
              class:active={courseFilter === c.id}
              onclick={() => (courseFilter = c.id)}
            >
              {shortCourse(c.name)}
            </button>
          {/each}
        </div>
      {/if}
      <button type="button" class="btn" onclick={() => (showSubmitted = !showSubmitted)}>
        {showSubmitted ? "隐藏已提交" : `显示已提交（${submittedCount}）`}
      </button>
    </div>

    {#if visible.length === 0}
      <section class="glass card empty">
        <CheckCircle2 size={26} aria-hidden="true" />
        <p>
          {showSubmitted
            ? "这个筛选下没有作业记录。"
            : "视野内没有待提交的作业 🎉（统计范围：近 30 天逾期 → 一年内截止）"}
        </p>
      </section>
    {:else}
      {#each GROUP_META as meta (meta.key)}
        {@const list = grouped[meta.key]}
        {#if list.length > 0}
          <section class="gblock" class:is-overdue={meta.key === "overdue"}>
            <h2 class="ghead">
              {meta.label}
              <span class="count">{list.length}</span>
              {#if meta.key === "overdue"}
                <Clock3 size={13} aria-hidden="true" class="warn" />
              {/if}
            </h2>
            <ul class="list glass">
              {#each list as a (a.id)}
                {@const cname = courseName(a.courseId)}
                <li class="item" class:done={a.submitted}>
                  <span class="chip" style={`--hue: ${courseHue(cname)};`} title={cname}>
                    {shortCourse(cname)}
                  </span>
                  <div class="mid">
                    <p class="title">{a.name}</p>
                    <p class="due" class:hot={a.dueAt > now && a.dueAt - now < 86_400_000}>
                      {fmtDue(a.dueAt)} · {countdown(a.dueAt, now)}
                    </p>
                  </div>
                  {#if a.submitted}
                    <span class="done-badge">
                      <CheckCircle2 size={13} aria-hidden="true" /> 已提交
                    </span>
                  {/if}
                  <button
                    type="button"
                    class="btn icon-only"
                    title="在浏览器中打开"
                    aria-label={`打开「${a.name}」`}
                    onclick={() => openAssignmentLink(a.htmlUrl)}
                  >
                    <ExternalLink size={14} aria-hidden="true" />
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
      {/each}
      <p class="note">
        只读同步自 Canvas 官方 API；每门课程取按截止时间排序的前 100 个作业，无截止日期的不显示，逾期超过
        30 天的不再列出。
      </p>
    {/if}
  {/if}
</div>

<style>
  .assign {
    max-width: 960px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }

  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: var(--space-4);
    flex-wrap: wrap;
  }

  h1 {
    margin: 0;
    font-size: var(--font-size-2xl);
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .sub {
    margin: var(--space-1) 0 0;
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }

  .head-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .last-sync {
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .card {
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .card h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--font-size-l);
    font-weight: 600;
  }

  .card :global(svg) {
    color: var(--accent);
  }

  .card.empty {
    align-items: center;
    text-align: center;
    color: var(--text-secondary);
    padding: var(--space-8, 48px) var(--space-6);
  }

  .card.empty p {
    margin: 0;
  }

  .steps {
    margin: 0;
    padding-left: 1.4em;
    color: var(--text-secondary);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    line-height: 1.65;
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
    font: inherit;
  }

  .token-form {
    display: flex;
    gap: var(--space-2);
  }

  .token-form input {
    flex: 1;
    padding: 7px var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--font-size-s);
  }

  .err {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--error);
    font-size: var(--font-size-s);
  }

  .err :global(svg) {
    color: var(--error);
    flex-shrink: 0;
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
    font-size: var(--font-size-m);
    cursor: pointer;
    white-space: nowrap;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .btn:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .btn:disabled {
    opacity: 0.55;
    cursor: default;
  }

  .btn.primary {
    border-color: transparent;
    background: var(--grad-accent);
    color: var(--accent-contrast);
    font-weight: 600;
  }

  .btn.icon-only {
    padding: 5px 8px;
  }

  .btn :global(.spin) {
    animation: rotate 1s linear infinite;
  }

  @keyframes rotate {
    to {
      transform: rotate(360deg);
    }
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .chips {
    display: flex;
    gap: var(--space-1);
    flex-wrap: wrap;
  }

  .chip-filter {
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: transparent;
    color: var(--text-secondary);
    font-size: var(--font-size-s);
    cursor: pointer;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .chip-filter:hover {
    background: var(--surface-hover);
  }

  .chip-filter.active {
    border-color: transparent;
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .gblock {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .ghead {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--font-size-m);
    font-weight: 600;
    color: var(--text-secondary);
  }

  .ghead .count {
    padding: 0 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: var(--font-size-s);
    font-variant-numeric: tabular-nums;
  }

  .gblock.is-overdue .ghead {
    color: var(--error);
  }

  .gblock.is-overdue .ghead .count {
    background: color-mix(in srgb, var(--error) 15%, transparent);
    color: var(--error);
  }

  .ghead :global(.warn) {
    color: var(--warn);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .item {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 9px var(--space-3);
    border-radius: var(--radius-m);
    transition: background var(--duration-fast) var(--ease-out);
  }

  .item:hover {
    background: var(--surface-hover);
  }

  .chip {
    flex-shrink: 0;
    max-width: 168px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 2px 9px;
    border-radius: 999px;
    font-size: var(--font-size-s);
    background: hsl(var(--hue) 55% 50% / 0.14);
    color: hsl(var(--hue) 65% 42%);
  }

  :global([data-theme="dark"]) .chip,
  :global([data-theme="oled"]) .chip {
    color: hsl(var(--hue) 75% 72%);
  }

  .mid {
    flex: 1;
    min-width: 0;
  }

  .title {
    margin: 0;
    font-size: var(--font-size-m);
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .due {
    margin: 1px 0 0;
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .due.hot {
    color: var(--warn);
    font-weight: 600;
  }

  .gblock.is-overdue .due {
    color: var(--error);
  }

  .item.done .title {
    color: var(--text-tertiary);
    text-decoration: line-through;
    text-decoration-color: color-mix(in srgb, var(--text-tertiary) 55%, transparent);
  }

  .done-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
    color: var(--ok);
    font-size: var(--font-size-s);
  }

  .done-badge :global(svg) {
    color: var(--ok);
  }

  .note {
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
    line-height: 1.6;
  }
</style>
