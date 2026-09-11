/// 今天页个性化偏好：自定义座右铭 + 时钟风格。所有值存本地 settings 表；
/// 读写失败时保留会话内状态（外观类偏好，失败可接受）。
import { getSetting, setSetting } from "../services/backend";

export type ClockStyle = "classic" | "minimal" | "flip" | "blocks";

export const CLOCK_STYLES: ReadonlyArray<{ value: ClockStyle; label: string }> = [
  { value: "classic", label: "经典" },
  { value: "minimal", label: "极简" },
  { value: "flip", label: "翻页" },
  { value: "blocks", label: "卡片" },
];

function isClockStyle(v: unknown): v is ClockStyle {
  return v === "classic" || v === "minimal" || v === "flip" || v === "blocks";
}

export const todayPrefs = $state({
  mottos: [] as string[],
  clockStyle: "classic" as ClockStyle,
});

/** Called from TodayPage and SettingsPage mounts (idempotent). */
export async function loadTodayPrefs(): Promise<void> {
  try {
    const m = await getSetting<string[] | null>("ui.mottos");
    if (Array.isArray(m)) {
      todayPrefs.mottos = m.filter((x): x is string => typeof x === "string").slice(0, 50);
    }
  } catch {
    /* backend off: keep defaults */
  }
  try {
    const c = await getSetting<unknown>("ui.clockStyle");
    if (isClockStyle(c)) todayPrefs.clockStyle = c;
  } catch {
    /* backend off: keep defaults */
  }
}

/** Replace the custom motto list; empty list = use built-in sentences. */
export async function setMottos(list: string[]): Promise<void> {
  const clean = list.map((m) => m.trim()).filter(Boolean).slice(0, 50);
  todayPrefs.mottos = clean;
  try {
    await setSetting("ui.mottos", clean);
  } catch {
    /* in-session only */
  }
}

export async function setClockStyle(style: ClockStyle): Promise<void> {
  todayPrefs.clockStyle = style;
  try {
    await setSetting("ui.clockStyle", style);
  } catch {
    /* in-session only */
  }
}
