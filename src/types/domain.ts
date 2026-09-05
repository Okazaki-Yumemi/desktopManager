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
/// `source: "external"` marks snapshot items dragged into a collection from
/// outside the desktop (no desktop index row).
export interface DesktopItem {
  id: number;
  path: string;
  source: "user_desktop" | "public_desktop" | "external";
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

/// A virtual collection (mirrors storage::collections_repo::Collection).
/// `parentId` nests it under another collection (sub-collections).
export interface Collection {
  id: number;
  name: string;
  color: string;
  parentId: number | null;
  itemCount: number;
}

/// One entry of a read-only folder listing (desktop::browse::PathEntry) used
/// to expand a folder reference in place. A synthetic marker entry beyond
/// the 500-item cap carries a non-path `path` starting with "\0".
export interface PathEntry {
  name: string;
  path: string;
  isDir: boolean;
  ext: string | null;
  sizeBytes: number | null;
}

/// A saved desktop icon-layout snapshot (storage::layout_repo::LayoutSummary).
export interface LayoutSummary {
  id: number;
  name: string;
  createdAt: number;
  itemCount: number;
}

/// Result of applying a layout (commands::layout ApplyReport).
export interface LayoutApplyReport {
  applied: number;
  missing: number;
  diverged: number;
}

/// A named arrangement of collections (storage::scenes_repo::Scene).
export interface Scene {
  id: number;
  name: string;
  color: string | null;
  sortOrder: number;
}

/// Visibility of one collection inside a scene (missing rows = visible).
export interface SceneLayout {
  collectionId: number;
  visible: boolean;
}

// --- Focus (M5) -----------------------------------------------------------

/// One focus block (mirrors storage::focus_repo::FocusSession). The started
/// timestamp is the clock: elapsed time is derived from it at read time, so
/// a restart or webview reload cannot lose a running session.
export type FocusKind = "pomodoro" | "custom" | "count_up";
export type FocusStatus = "running" | "completed" | "interrupted" | "abandoned";

export interface FocusSession {
  id: number;
  taskId: number | null;
  sceneId: number | null;
  kind: FocusKind;
  plannedDurationS: number;
  actualDurationS: number;
  status: FocusStatus;
  startedAt: number;
  endedAt: number | null;
  interruptions: number;
  note: string | null;
}

/// Aggregated focus time for one local calendar day (YYYY-MM-DD).
export interface FocusDay {
  day: string;
  totalS: number;
  sessions: number;
  interruptions: number;
}

// --- Tasks & Calendar (M6) -------------------------------------------------

export type TaskStatus = "todo" | "doing" | "done";

/// One task (mirrors storage::tasks_repo::Task). Priority 0–3 (0 = none).
export interface Task {
  id: number;
  title: string;
  notes: string | null;
  status: TaskStatus;
  priority: number;
  dueAt: number | null;
  estimatedMinutes: number | null;
  tags: string[];
  createdAt: number;
  completedAt: number | null;
  updatedAt: number;
}

/// One calendar event (mirrors storage::calendar_repo::CalendarEvent).
/// Times are epoch ms; all-day events span their local day.
export interface CalendarEvent {
  id: number;
  title: string;
  startsAt: number;
  endsAt: number;
  allDay: boolean;
  notes: string | null;
  color: string | null;
  taskId: number | null;
  createdAt: number;
  updatedAt: number;
}
