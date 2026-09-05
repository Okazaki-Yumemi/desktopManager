import { invoke } from "@tauri-apps/api/core";
import type { AppInfo, ShortcutInfo } from "../types/domain";

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
