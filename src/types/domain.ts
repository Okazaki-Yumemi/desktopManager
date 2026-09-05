/// Types shared with the Rust backend (serde camelCase payloads).
export interface AppInfo {
  name: string;
  version: string;
  dataDir: string;
  dbPath: string;
  logDir: string;
  os: string;
  schemaVersion: number;
}

export interface ShortcutInfo {
  binding: string;
  registered: boolean;
  error: string | null;
}

/// One indexed desktop entry (mirrors storage::desktop_repo::DesktopItem).
export interface DesktopItem {
  id: number;
  path: string;
  source: "user_desktop" | "public_desktop";
  displayName: string;
  kind: "file" | "folder" | "shortcut";
  ext: string | null;
  sizeBytes: number | null;
  modifiedAt: number | null;
  missing: boolean;
}

export interface SyncOutcome {
  added: number;
  updated: number;
  removed: number;
}
