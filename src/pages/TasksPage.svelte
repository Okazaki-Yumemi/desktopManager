<script lang="ts">
  import { onMount } from "svelte";
  import { Circle, CircleCheck, CircleDashed, Flag, Pencil, Search, X } from "@lucide/svelte";
  import { createTask, deleteTask, listTasks, setTaskStatus, updateTask } from "../services/backend";
  import type { Task, TaskStatus } from "../types/domain";
  import { pushToast } from "../stores/toast.svelte";

  const PRIORITY_LABELS = ["无", "低", "中", "高"] as const;

  let tasks = $state<Task[]>([]);
  let loading = $state(true);
  let query = $state("");
  let filter = $state<"all" | TaskStatus>("all");

  // Quick capture: the one input that must always be one keystroke away.
  let capture = $state("");
  let captureInput = $state<HTMLInputElement | undefined>(undefined);
  let creating = $state(false);

  // Inline editing of one task.
  let editingId = $state<number | null>(null);
  let editTitle = $state("");
  let editPriority = $state(0);
  let editDue = $state("");
  let editNotes = $state("");

  const hasQuery = $derived(query.trim().length > 0);
  const counts = $derived.by(() => ({
    all: tasks.length,
    todo: tasks.filter((t) => t.status === "todo").length,
    doing: tasks.filter((t) => t.status === "doing").length,
    done: tasks.filter((t) => t.status === "done").length,
  }));
  const visible = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    return tasks.filter(
      (t) =>
        (filter === "all" || t.status === filter) &&
        (!needle ||
          t.title.toLowerCase().includes(needle) ||
          (t.notes?.toLowerCase().includes(needle) ?? false)),
    );
  });

  onMount(() => {
    void reload();
    // In-app fast capture: Ctrl+N focuses the capture box from anywhere.
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "n") {
        e.preventDefault();
        captureInput?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  async function reload() {
    try {
      tasks = await listTasks();
    } catch (err) {
      pushToast("error", `读取任务失败：${err instanceof Error ? err.message : String(err)}`);
    } finally {
      loading = false;
    }
  }

  async function submitCapture() {
    if (!creating) return;
    creating = false;
    const title = capture.trim();
    capture = "";
    if (!title) return;
    try {
      await createTask(title);
      await reload();
    } catch (err) {
      pushToast("error", `创建失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function cycleStatus(task: Task) {
    const next: TaskStatus =
      task.status === "todo" ? "doing" : task.status === "doing" ? "done" : "todo";
    try {
      await setTaskStatus(task.id, next);
      await reload();
    } catch (err) {
      pushToast("error", `更新状态失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  function startEdit(task: Task) {
    editingId = task.id;
    editTitle = task.title;
    editPriority = task.priority;
    editDue = task.dueAt === null ? "" : toDateInput(task.dueAt);
    editNotes = task.notes ?? "";
  }

  async function submitEdit() {
    if (editingId === null) return;
    const id = editingId;
    editingId = null;
    const title = editTitle.trim();
    if (!title) return;
    try {
      await updateTask(id, title, {
        notes: editNotes.trim() || null,
        priority: editPriority,
        dueAt: editDue ? fromDateInput(editDue) : null,
      });
      await reload();
      pushToast("ok", "已保存");
    } catch (err) {
      pushToast("error", `保存失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  async function remove(task: Task) {
    try {
      await deleteTask(task.id);
      if (editingId === task.id) editingId = null;
      await reload();
    } catch (err) {
      pushToast("error", `删除失败：${err instanceof Error ? err.message : String(err)}`);
    }
  }

  function toDateInput(ms: number): string {
    const d = new Date(ms);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  function fromDateInput(value: string): number {
    return new Date(`${value}T00:00:00`).getTime();
  }

  function dueLabel(task: Task): { text: string; overdue: boolean } {
    const due = task.dueAt;
    if (due === null) return { text: "", overdue: false };
    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const day = new Date(due);
    const dayStart = new Date(day.getFullYear(), day.getMonth(), day.getDate());
    const days = Math.round((dayStart.getTime() - today.getTime()) / 86_400_000);
    if (days < 0) return { text: `逾期 ${-days} 天`, overdue: true };
    if (days === 0) return { text: "今天", overdue: false };
    if (days === 1) return { text: "明天", overdue: false };
    return { text: `${day.getMonth() + 1}月${day.getDate()}日`, overdue: false };
  }

  function statusIcon(status: TaskStatus) {
    return status === "todo" ? Circle : status === "doing" ? CircleDashed : CircleCheck;
  }
</script>

<div class="tasks">
  <header class="head">
    <div>
      <h1>任务</h1>
      <p class="muted">
        {#if loading}
          正在读取…
        {:else}
          {counts.doing} 进行中 · {counts.todo} 待办 · {counts.done} 已完成
        {/if}
      </p>
    </div>
  </header>

  <div class="capture">
    <input
      type="text"
      placeholder="想到什么就记下来，回车创建（Ctrl+N 随时回到这里）"
      bind:value={capture}
      bind:this={captureInput}
      maxlength="80"
      onfocus={() => (creating = true)}
      onblur={() => void submitCapture()}
      onkeydown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          (e.currentTarget as HTMLInputElement).blur();
        }
      }}
    />
  </div>

  <div class="chips">
    <button type="button" class="chip" class:active={filter === "all"} onclick={() => (filter = "all")}>
      全部 <span class="count">{counts.all}</span>
    </button>
    <button
      type="button"
      class="chip"
      class:active={filter === "doing"}
      onclick={() => (filter = "doing")}
    >
      进行中 <span class="count">{counts.doing}</span>
    </button>
    <button
      type="button"
      class="chip"
      class:active={filter === "todo"}
      onclick={() => (filter = "todo")}
    >
      待办 <span class="count">{counts.todo}</span>
    </button>
    <button
      type="button"
      class="chip"
      class:active={filter === "done"}
      onclick={() => (filter = "done")}
    >
      已完成 <span class="count">{counts.done}</span>
    </button>
    <span class="search">
      <Search size={14} />
      <input type="search" placeholder="搜索任务…" bind:value={query} aria-label="搜索任务" />
    </span>
  </div>

  {#if loading}
    <p class="muted state">正在加载…</p>
  {:else if visible.length === 0}
    <p class="muted state">
      {#if hasQuery}
        没有匹配的任务
      {:else if filter === "all"}
        还没有任务——在上面输入，回车即可创建
      {:else}
        这个状态下没有任务
      {/if}
    </p>
  {:else}
    <ul class="list">
      {#each visible as task (task.id)}
        {@const StatusIcon = statusIcon(task.status)}
        <li>
          {#if editingId === task.id}
            <div class="edit">
              <input class="edit-title" bind:value={editTitle} maxlength="80" aria-label="任务标题" />
              <div class="edit-row">
                <label>
                  优先级
                  <select bind:value={editPriority}>
                    {#each PRIORITY_LABELS as label, p (p)}
                      <option value={p}>{label}</option>
                    {/each}
                  </select>
                </label>
                <label>
                  截止
                  <input type="date" bind:value={editDue} />
                </label>
              </div>
              <textarea bind:value={editNotes} rows="2" placeholder="备注（可选）"></textarea>
              <div class="edit-actions">
                <button type="button" class="btn primary" onclick={() => void submitEdit()}>
                  保存
                </button>
                <button type="button" class="btn" onclick={() => (editingId = null)}>取消</button>
              </div>
            </div>
          {:else}
            <div class="row">
              <button
                type="button"
                class="status {task.status}"
                title="点击推进状态：待办 → 进行中 → 完成"
                onclick={() => void cycleStatus(task)}
              >
                <StatusIcon size={14} />
              </button>
              <span class="title" class:done={task.status === "done"}>{task.title}</span>
              {#if task.priority > 0}
                <span
                  class="prio prio-{task.priority}"
                  title="优先级：{PRIORITY_LABELS[task.priority]}"
                >
                  <Flag size={12} />
                  {PRIORITY_LABELS[task.priority]}
                </span>
              {/if}
              {#if task.dueAt !== null}
                {@const due = dueLabel(task)}
                <span class="due" class:overdue={due.overdue}>{due.text}</span>
              {/if}
              {#if task.notes}
                <span class="note-dot" title={task.notes}>备注</span>
              {/if}
              <span class="actions">
                <button type="button" class="row-btn" title="编辑" onclick={() => startEdit(task)}>
                  <Pencil size={13} />
                </button>
                <button type="button" class="row-btn" title="删除" onclick={() => void remove(task)}>
                  <X size={13} />
                </button>
              </span>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .tasks {
    max-width: 720px;
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

  .capture input {
    width: 100%;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-primary);
    outline: none;
    font-size: var(--font-size-m);
  }

  .capture input:focus {
    border-color: var(--accent);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin: var(--space-4) 0;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 12px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-secondary);
    font-size: var(--font-size-s);
    cursor: pointer;
  }

  .chip.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--text-primary);
  }

  .count {
    color: var(--text-tertiary);
  }

  .search {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    color: var(--text-tertiary);
  }

  .search input {
    border: none;
    background: transparent;
    outline: none;
    color: var(--text-primary);
    padding: 4px 0;
    width: 160px;
  }

  .state {
    text-align: center;
    padding: var(--space-6) 0;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
  }

  .status {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    padding: 0;
    border: none;
    border-radius: 999px;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
  }

  .status.doing {
    color: var(--accent);
    animation: turn 1.6s linear infinite;
  }

  .status.done {
    color: var(--ok);
  }

  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title.done {
    color: var(--text-tertiary);
    text-decoration: line-through;
  }

  .prio {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .prio-1 {
    color: var(--text-secondary);
  }

  .prio-2 {
    color: var(--warn);
  }

  .prio-3 {
    color: var(--error);
  }

  .due {
    flex-shrink: 0;
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .due.overdue {
    color: var(--error);
  }

  .note-dot {
    flex-shrink: 0;
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .actions {
    display: inline-flex;
    gap: 2px;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--duration-fast) var(--ease-out);
  }

  .row:hover .actions,
  .row:focus-within .actions {
    opacity: 1;
    pointer-events: auto;
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
  }

  .row-btn:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .edit {
    padding: var(--space-3);
    border: 1px solid var(--accent);
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .edit-title {
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-primary);
    outline: none;
  }

  .edit-row {
    display: flex;
    gap: var(--space-4);
    font-size: var(--font-size-s);
    color: var(--text-secondary);
  }

  .edit-row label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .edit-row select,
  .edit-row input[type="date"] {
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-primary);
    padding: 3px 6px;
  }

  .edit textarea {
    resize: vertical;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    background: var(--surface);
    color: var(--text-primary);
    font: inherit;
    outline: none;
  }

  .edit-actions {
    display: flex;
    gap: var(--space-2);
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
</style>
