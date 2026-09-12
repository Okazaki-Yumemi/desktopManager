import {
  canvasOpenLink,
  canvasSync,
  canvasTestConnection,
  getSetting,
  setSetting,
} from "../services/backend";
import type { CanvasSnapshot } from "../types/domain";
import { pushToast } from "./toast.svelte";

/**
 * Canvas LMS state (M15): a read-only assignment/DDL snapshot pulled by the
 * backend through the official Canvas REST API. The personal access token is
 * pasted by the user on the 作业 page and lives only in the local settings
 * table; nothing ever leaves the machine except the authenticated GETs to
 * oc.sjtu.edu.cn themselves. The snapshot is cached in settings so restarts
 * restore the last known deadlines.
 */

const TOKEN_KEY = "canvas.token";
const SNAPSHOT_KEY = "canvas.snapshot";
const LAST_SYNC_KEY = "canvas.lastSyncAt";

export const CANVAS_BASE = "https://oc.sjtu.edu.cn";
export const CANVAS_HOST = "oc.sjtu.edu.cn";

let token = $state("");
let snapshot = $state<CanvasSnapshot | null>(null);
let lastSyncAt = $state<number | null>(null);
let syncing = $state(false);
let setupError = $state<string | null>(null);

export function canvasToken(): string {
  return token;
}

export function canvasSnapshot(): CanvasSnapshot | null {
  return snapshot;
}

export function canvasLastSyncAt(): number | null {
  return lastSyncAt;
}

export function canvasSyncing(): boolean {
  return syncing;
}

export function canvasSetupError(): string | null {
  return setupError;
}

export async function loadCanvasState(): Promise<void> {
  try {
    const [savedToken, savedSnapshot, savedAt] = await Promise.all([
      getSetting<string>(TOKEN_KEY),
      getSetting<CanvasSnapshot>(SNAPSHOT_KEY),
      getSetting<number>(LAST_SYNC_KEY),
    ]);
    token = typeof savedToken === "string" ? savedToken : "";
    snapshot = savedSnapshot ?? null;
    lastSyncAt = typeof savedAt === "number" ? savedAt : null;
    // Startup refresh: deadlines drift daily, and a fresh snapshot is the
    // whole point of the page. Silent — failures keep the cached view.
    if (token) void syncCanvas({ silent: true });
  } catch {
    // Backend unavailable (e.g. plain browser dev): page shows setup state.
  }
}

/** Save + verify a token; only a working token lands in settings. */
export async function saveCanvasToken(raw: string): Promise<boolean> {
  const trimmed = raw.trim();
  if (!trimmed) {
    setupError = "令牌不能为空";
    return false;
  }
  syncing = true;
  setupError = null;
  try {
    const profile = await canvasTestConnection(CANVAS_BASE, trimmed);
    token = trimmed;
    await setSetting(TOKEN_KEY, trimmed);
    pushToast("ok", `Canvas 已连接：${profile.name}`);
    await syncCanvas({ silent: true });
    return true;
  } catch (err) {
    setupError = err instanceof Error ? err.message : String(err);
    return false;
  } finally {
    syncing = false;
  }
}

export async function removeCanvasToken(): Promise<void> {
  token = "";
  snapshot = null;
  lastSyncAt = null;
  setupError = null;
  try {
    await Promise.all([
      setSetting(TOKEN_KEY, null),
      setSetting(SNAPSHOT_KEY, null),
      setSetting(LAST_SYNC_KEY, null),
    ]);
    pushToast("info", "已清除 Canvas 令牌与本地缓存");
  } catch (err) {
    pushToast("error", `清除失败：${err instanceof Error ? err.message : String(err)}`);
  }
}

export async function syncCanvas(opts: { silent?: boolean } = {}): Promise<void> {
  if (!token || syncing) return;
  syncing = true;
  try {
    const snap = await canvasSync(CANVAS_BASE, token);
    snapshot = snap;
    lastSyncAt = snap.fetchedAt;
    await Promise.all([
      setSetting(SNAPSHOT_KEY, snap),
      setSetting(LAST_SYNC_KEY, snap.fetchedAt),
    ]);
    if (!opts.silent) {
      pushToast(
        snap.skippedCourses > 0 ? "info" : "ok",
        snap.skippedCourses > 0
          ? `同步完成：${snap.assignments.length} 条作业（${snap.skippedCourses} 门课程拉取失败）`
          : `同步完成：${snap.assignments.length} 条作业`,
      );
    }
  } catch (err) {
    if (!opts.silent) {
      pushToast("error", `Canvas 同步失败：${err instanceof Error ? err.message : String(err)}`);
    }
  } finally {
    syncing = false;
  }
}

export function openAssignmentLink(url: string): void {
  void canvasOpenLink(CANVAS_BASE, url).catch((err) =>
    pushToast("error", `无法打开链接：${err instanceof Error ? err.message : String(err)}`),
  );
}
