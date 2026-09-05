export interface Toast {
  id: number;
  kind: "error" | "info" | "ok";
  message: string;
}

const items = $state<Toast[]>([]);
let nextId = 1;

export function currentToasts(): Toast[] {
  return items;
}

export function pushToast(kind: Toast["kind"], message: string, ttlMs = 6000): void {
  const toast: Toast = { id: nextId++, kind, message };
  items.push(toast);
  setTimeout(() => {
    const idx = items.findIndex((t) => t.id === toast.id);
    if (idx >= 0) items.splice(idx, 1);
  }, ttlMs);
}

export function dismissToast(id: number): void {
  const idx = items.findIndex((t) => t.id === id);
  if (idx >= 0) items.splice(idx, 1);
}
