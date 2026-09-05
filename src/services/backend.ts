import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, Collection, DesktopItem, ShortcutInfo, SyncOutcome } from "../types/domain";

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
