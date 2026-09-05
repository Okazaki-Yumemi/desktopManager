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
