export type PageId = "today" | "desktop" | "focus" | "calendar" | "tasks" | "settings";

export const PAGES: ReadonlyArray<{ id: PageId; label: string }> = [
  { id: "today", label: "今天" },
  { id: "desktop", label: "桌面" },
  { id: "focus", label: "专注" },
  { id: "calendar", label: "日历" },
  { id: "tasks", label: "任务" },
  { id: "settings", label: "设置" },
];

let current = $state<PageId>("today");

export function currentPage(): PageId {
  return current;
}

export function navigate(page: PageId): void {
  current = page;
}
