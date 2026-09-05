import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, DesktopItem, ShortcutInfo, SyncOutcome } from "../types/domain";

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
