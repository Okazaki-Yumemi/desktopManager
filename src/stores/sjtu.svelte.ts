import { playChime } from "../lib/chime.svelte";
import {
  clearSjtuEvents,
  getSetting,
  listSjtuEvents,
  onSjtuWindowClosed,
  openSjtuSync,
  onSjtuSynced,
  type SjtuSyncReport,
} from "../services/backend";
import { SvelteSet } from "svelte/reactivity";
import type { SjtuEvent } from "../types/domain";
import { pushToast } from "./toast.svelte";

/**
 * SJTU calendar state (M12): a read-only projection pushed in by the sync
 * webview. The store also runs the "next class" reminder — a toast + chime
 * ten minutes before each class while the app is open. Notification state is
 * session-scoped on purpose: after a restart an imminent class may chime
 * once more, which is the safe direction to err on.
 */

const REMINDER_LEAD_MS = 10 * 60_000;
const TICK_MS = 20_000;

let events = $state<SjtuEvent[]>([]);
let lastSyncAt = $state<number | null>(null);
let syncing = $state(false);

const notified = new SvelteSet<string>();

export function sjtuEvents(): SjtuEvent[] {
  return events;
}

export function sjtuLastSyncAt(): number | null {
  return lastSyncAt;
}

export function sjtuSyncing(): boolean {
  return syncing;
}

export async function loadSjtu(): Promise<void> {
  try {
    const [list, saved] = await Promise.all([
      listSjtuEvents(),
      getSetting<number>("sjtu.lastSyncAt"),
    ]);
    events = list;
    lastSyncAt = typeof saved === "number" ? saved : null;
  } catch {
    // Backend unavailable (e.g. plain browser dev): sidebar shows empty state.
  }
}

/** Open the sync window; completion arrives later via onSjtuSynced. */
export async function startSjtuSync(): Promise<void> {
  syncing = true;
  try {
    await openSjtuSync();
    pushToast(
      "info",
      "已打开交大日历窗口；若要求登录请在窗口中登录 jAccount，同步完成后会自动关闭。",
      9000,
    );
  } catch (err) {
    syncing = false;
    pushToast("error", `无法打开同步窗口：${err instanceof Error ? err.message : String(err)}`);
  }
}

export async function clearSjtu(): Promise<void> {
  try {
    const removed = await clearSjtuEvents();
    pushToast("ok", removed > 0 ? `已清除 ${removed} 条交大日程` : "交大日程本来就是空的");
  } catch (err) {
    pushToast("error", `清除失败：${err instanceof Error ? err.message : String(err)}`);
  }
}

/** Backend event handler: reload the projection, close the loop on `syncing`. */
export function applySjtuReport(report: SjtuSyncReport): void {
  syncing = false;
  if (report.count > 0) {
    pushToast("ok", `交大日程已同步：${report.count} 条`);
  }
  void loadSjtu();
}

/** Register the backend listener; returns its unlisten function. */
export function watchSjtuSynced(): Promise<() => void> {
  return onSjtuSynced(applySjtuReport);
}

/**
 * Re-arm the sync button when the sync window goes away without a sync
 * (the usual case before the user has logged into jAccount).
 */
export function watchSjtuWindowClosed(): Promise<() => void> {
  return onSjtuWindowClosed(() => (syncing = false));
}

/** The class in progress right now and the next upcoming one. */
export function sjtuNext(nowMs: number): { running: SjtuEvent | null; next: SjtuEvent | null } {
  let running: SjtuEvent | null = null;
  let next: SjtuEvent | null = null;
  for (const e of events) {
    if (e.allDay) continue;
    if (e.startsAt <= nowMs && nowMs < e.endsAt) {
      running ??= e;
    } else if (e.startsAt > nowMs && (next === null || e.startsAt < next.startsAt)) {
      next = e;
    }
  }
  return { running, next };
}

/** 20s ticker: fire the 10-minute-before reminder once per event. */
export function startSjtuReminder(): () => void {
  const check = () => {
    const now = Date.now();
    for (const e of events) {
      if (e.allDay || e.source !== "personal") continue;
      const lead = e.startsAt - now;
      if (lead > 0 && lead <= REMINDER_LEAD_MS && !notified.has(e.externalId)) {
        notified.add(e.externalId);
        pushToast(
          "info",
          `10 分钟后有课：${e.title}${e.location ? ` @ ${e.location}` : ""}`,
          12_000,
        );
        playChime("focus");
      }
    }
  };
  check();
  const timer = setInterval(check, TICK_MS);
  return () => clearInterval(timer);
}
