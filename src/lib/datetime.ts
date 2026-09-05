/// Pure date/time helpers (unit tested; no DOM dependency).

export function formatDateLong(d: Date, locale = "zh-CN"): string {
  return d.toLocaleDateString(locale, {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

/// Compact date for list rows (epoch milliseconds → "2026/9/5").
export function formatDateShort(epochMs: number): string {
  return new Date(epochMs).toLocaleDateString("zh-CN");
}

export function greetingForHour(hour: number): string {
  if (hour < 5) return "夜深了";
  if (hour < 12) return "早上好";
  if (hour < 18) return "下午好";
  return "晚上好";
}

export function formatDuration(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}小时${m}分`;
  if (m > 0) return `${m}分${sec}秒`;
  return `${sec}秒`;
}
