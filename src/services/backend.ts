import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  Collection,
  DesktopItem,
  FocusDay,
  FocusKind,
  FocusSession,
  LayoutApplyReport,
  LayoutSummary,
  Scene,
  SceneLayout,
  ShortcutInfo,
  SyncOutcome,
} from "../types/domain";

export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}

export async function getShortcutInfo(): Promise<ShortcutInfo> {
  return invoke<ShortcutInfo>("shortcuts_get");
}

export async function getSetting<T>(key: string): Promise<T | null> {
  return invoke<T | null>("settings_get", { key });
}

export async function setSetting(key: string, value: unknown): Promise<void> {
  await invoke("settings_set", { key, value });
}

export async function getDesktopItems(): Promise<DesktopItem[]> {
  return invoke<DesktopItem[]>("desktop_list");
}

export async function searchDesktopItems(query: string): Promise<DesktopItem[]> {
  return invoke<DesktopItem[]>("desktop_search", { query });
}

export async function rescanDesktop(): Promise<SyncOutcome> {
  return invoke<SyncOutcome>("desktop_rescan");
}

export async function openDesktopItem(path: string): Promise<void> {
  await invoke("desktop_open", { path });
}

export async function listCollections(): Promise<Collection[]> {
  return invoke<Collection[]>("collections_list");
}

export async function createCollection(name: string, color: string): Promise<Collection> {
  return invoke<Collection>("collection_create", { name, color });
}

export async function deleteCollection(id: number): Promise<void> {
  await invoke("collection_delete", { id });
}

export async function assignToCollection(id: number, path: string): Promise<boolean> {
  return invoke<boolean>("collection_assign", { id, path });
}

export async function unassignFromCollection(id: number, path: string): Promise<boolean> {
  return invoke<boolean>("collection_unassign", { id, path });
}

export async function getCollectionItems(id: number): Promise<DesktopItem[]> {
  return invoke<DesktopItem[]>("collection_items", { id });
}

/** Drag a path from outside the desktop (Explorer, Start Menu, …) in. */
export async function assignExternalToCollection(
  id: number,
  path: string,
): Promise<boolean> {
  return invoke<boolean>("collection_assign_external", { id, path });
}

/** Open a collection-held item (indexed or external snapshot). */
export async function openCollectionItem(path: string): Promise<void> {
  await invoke("collection_open", { path });
}

export async function setWallpaper(dataB64: string, mime: string): Promise<void> {
  await invoke("background_set", { dataB64, mime });
}

export async function clearWallpaper(): Promise<void> {
  await invoke("background_clear");
}

/** kind: "collections" | "all". Backs the DB up before deleting. */
export async function purgeAppData(kind: "collections" | "all"): Promise<void> {
  await invoke("appdata_purge", { kind });
}

/** Capture the live desktop icon layout through the shell (LVM route). */
export async function captureLayout(
  name: string,
): Promise<{ id: number; name: string; itemCount: number }> {
  return invoke("layout_capture", { name });
}

export async function listLayouts(): Promise<LayoutSummary[]> {
  return invoke<LayoutSummary[]>("layout_list");
}

/** Restore a saved layout; refused when auto-arrange is detected. */
export async function applyLayout(id: number): Promise<LayoutApplyReport> {
  return invoke<LayoutApplyReport>("layout_apply", { id });
}

export async function deleteLayout(id: number): Promise<boolean> {
  return invoke<boolean>("layout_delete", { id });
}

// --- Scenes (M4) -----------------------------------------------------------

export async function listScenes(): Promise<Scene[]> {
  return invoke<Scene[]>("scenes_list");
}

export async function createScene(name: string, color?: string): Promise<Scene> {
  return invoke<Scene>("scene_create", { name, color });
}

export async function renameScene(id: number, name: string): Promise<void> {
  await invoke("scene_rename", { id, name });
}

export async function deleteScene(id: number): Promise<void> {
  await invoke("scene_delete", { id });
}

export async function setSceneVisibility(
  id: number,
  collectionId: number,
  visible: boolean,
): Promise<void> {
  await invoke("scene_set_visibility", { id, collectionId, visible });
}

/** Visibility rows of a scene; collections without a row are visible. */
export async function getSceneVisibility(id: number): Promise<SceneLayout[]> {
  return invoke<SceneLayout[]>("scene_visibility", { id });
}

// --- Focus (M5) -----------------------------------------------------------

/** Start a focus session; refused while another one is still running. */
export async function startFocus(
  kind: FocusKind,
  plannedSeconds: number,
  taskId?: number | null,
  sceneId?: number | null,
): Promise<FocusSession> {
  return invoke<FocusSession>("focus_start", { kind, plannedSeconds, taskId, sceneId });
}

/** The running session, if any — the recovery path after a restart. */
export async function getRunningFocus(): Promise<FocusSession | null> {
  return invoke<FocusSession | null>("focus_running");
}

/** End a session as completed or abandoned. */
export async function finishFocus(
  id: number,
  status: "completed" | "abandoned",
): Promise<FocusSession> {
  return invoke<FocusSession>("focus_finish", { id, status });
}

/** Tally one mid-session interruption; the session keeps running. */
export async function interruptFocus(id: number): Promise<FocusSession> {
  return invoke<FocusSession>("focus_interrupt", { id });
}

/** Free-text note on a session (blank clears it). */
export async function setFocusNote(id: number, note: string): Promise<void> {
  await invoke("focus_note", { id, note });
}

/** Sessions that started on a local day (YYYY-MM-DD), running ones included. */
export async function listFocusSessions(day: string): Promise<FocusSession[]> {
  return invoke<FocusSession[]>("focus_sessions", { day });
}

/** Per-day focus totals over the last N local days. */
export async function getFocusSummary(days: number): Promise<FocusDay[]> {
  return invoke<FocusDay[]>("focus_summary", { days });
}
