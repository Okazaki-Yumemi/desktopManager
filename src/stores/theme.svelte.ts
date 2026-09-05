import { getSetting, setSetting } from "../services/backend";

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

export const THEME_SETTING_KEY = "ui.theme";

let preference = $state<ThemePreference>("system");

export const DARK_QUERY = "(prefers-color-scheme: dark)";

export function systemPrefersDark(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia(DARK_QUERY).matches
  );
}

/// Pure resolution logic — unit tested without a DOM.
export function resolveTheme(p: ThemePreference, systemDark: boolean): ResolvedTheme {
  if (p === "system") return systemDark ? "dark" : "light";
  return p;
}

export function getThemePreference(): ThemePreference {
  return preference;
}

export function applyTheme(p: ThemePreference): ResolvedTheme {
  const resolved = resolveTheme(p, systemPrefersDark());
  document.documentElement.dataset.theme = resolved;
  return resolved;
}

export async function loadThemePreference(): Promise<void> {
  try {
    const saved = await getSetting<string>(THEME_SETTING_KEY);
    if (saved === "light" || saved === "dark" || saved === "system") {
      preference = saved;
      applyTheme(saved);
    } else {
      applyTheme("system");
    }
  } catch {
    // Backend unavailable (e.g. plain browser dev): fall back to system look.
    applyTheme("system");
  }
}

export async function setThemePreference(p: ThemePreference): Promise<void> {
  preference = p;
  applyTheme(p);
  // Propagate: callers surface persistence failures to the user.
  await setSetting(THEME_SETTING_KEY, p);
}

// ---------------------------------------------------------------------------
// Accent color (theme-independent; see tokens.css `[data-accent=…]` blocks)
// ---------------------------------------------------------------------------

export type AccentPreference = "ocean" | "violet" | "grass" | "amber" | "rose";

export const ACCENT_SETTING_KEY = "ui.accent";

export const ACCENT_PRESETS: ReadonlyArray<{ value: AccentPreference; label: string }> = [
  { value: "ocean", label: "海蓝" },
  { value: "violet", label: "紫罗兰" },
  { value: "grass", label: "草绿" },
  { value: "amber", label: "琥珀" },
  { value: "rose", label: "玫红" },
];

function isAccent(v: unknown): v is AccentPreference {
  return typeof v === "string" && ACCENT_PRESETS.some((p) => p.value === v);
}

let accent = $state<AccentPreference>("ocean");

export function getAccentPreference(): AccentPreference {
  return accent;
}

export function applyAccent(a: AccentPreference): void {
  document.documentElement.dataset.accent = a;
}

export async function loadAccentPreference(): Promise<void> {
  try {
    const saved = await getSetting<string>(ACCENT_SETTING_KEY);
    if (isAccent(saved)) accent = saved;
  } catch {
    // Backend unavailable (e.g. plain browser dev): keep the default accent.
  }
  applyAccent(accent);
}

export async function setAccentPreference(a: AccentPreference): Promise<void> {
  accent = a;
  applyAccent(a);
  await setSetting(ACCENT_SETTING_KEY, a);
}

/// Re-resolve when the OS theme flips while following "system".
/// Returns a cleanup function.
export function watchSystemTheme(): () => void {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
    return () => {};
  }
  const mql = window.matchMedia(DARK_QUERY);
  const onChange = () => {
    if (preference === "system") applyTheme("system");
  };
  mql.addEventListener("change", onChange);
  return () => mql.removeEventListener("change", onChange);
}
