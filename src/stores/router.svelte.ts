export type PageId = "today" | "desktop" | "focus" | "calendar" | "tasks" | "settings";

export const PAGES: ReadonlyArray<{ id: PageId; label: string }> = [
  { id: "today", label: "Today" },
  { id: "desktop", label: "Desktop" },
  { id: "focus", label: "Focus" },
  { id: "calendar", label: "Calendar" },
  { id: "tasks", label: "Tasks" },
  { id: "settings", label: "Settings" },
];

let current = $state<PageId>("today");

export function currentPage(): PageId {
  return current;
}

export function navigate(page: PageId): void {
  current = page;
}
