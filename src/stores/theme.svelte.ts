import { getSetting, setSetting } from "../services/backend";
import { resolveEnum } from "../lib/prefs";

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
  // A preset choice supersedes any custom color; the inline overrides must go
  // or they would keep winning the cascade over the preset blocks.
  customAccent = null;
  clearCustomAccent();
  applyAccent(a);
  await setSetting(ACCENT_SETTING_KEY, a);
}

// ---------------------------------------------------------------------------
// Custom accent color (M7 leftover): a hex the user picks, stored verbatim
// and applied as inline CSS custom properties so it wins the cascade over
// every [data-accent] preset block in tokens.css.
// ---------------------------------------------------------------------------

export const ACCENT_CUSTOM_SETTING_KEY = "ui.accentCustom";

export const HEX_COLOR_RE = /^#[0-9a-fA-F]{6}$/;

let customAccent = $state<string | null>(null);

export function getCustomAccent(): string | null {
  return customAccent;
}

function applyCustomAccent(hex: string): void {
  document.documentElement.dataset.accent = "custom";
  document.documentElement.style.setProperty("--accent", hex);
  document.documentElement.style.setProperty(
    "--accent-soft",
    `color-mix(in srgb, ${hex} 13%, transparent)`,
  );
}

function clearCustomAccent(): void {
  document.documentElement.style.removeProperty("--accent");
  document.documentElement.style.removeProperty("--accent-soft");
}

export async function loadCustomAccent(): Promise<void> {
  try {
    const saved = await getSetting<string>(ACCENT_CUSTOM_SETTING_KEY);
    if (typeof saved === "string" && HEX_COLOR_RE.test(saved)) {
      customAccent = saved.toLowerCase();
      applyCustomAccent(customAccent);
    }
  } catch {
    // Backend unavailable: presets remain in charge.
  }
}

export async function setCustomAccent(hex: string): Promise<void> {
  const norm = hex.toLowerCase();
  if (!HEX_COLOR_RE.test(norm)) throw new Error("无效的颜色值");
  customAccent = norm;
  applyCustomAccent(norm);
  await setSetting(ACCENT_CUSTOM_SETTING_KEY, norm);
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

// ---------------------------------------------------------------------------
// Enum-valued appearance preferences (M7): surface style, density, glass
// strength, motion. Each one persists a settings key and mirrors its value
// onto a `data-*` attribute that tokens.css responds to.
// ---------------------------------------------------------------------------

type EnumPref<T extends string> = {
  get(): T;
  load(): Promise<void>;
  set(v: T): Promise<void>;
};

function enumPref<T extends string>(
  key: string,
  attr: string,
  allowed: readonly T[],
  fallback: T,
): EnumPref<T> {
  let value = $state<T>(fallback);
  return {
    get: () => value,
    load: async () => {
      try {
        const saved = await getSetting<string>(key);
        value = resolveEnum(allowed, saved, fallback);
      } catch {
        // Backend unavailable (e.g. plain browser dev): keep the default.
      }
      document.documentElement.dataset[attr] = value;
    },
    set: async (v: T) => {
      value = v;
      document.documentElement.dataset[attr] = v;
      await setSetting(key, v);
    },
  };
}

export type SurfacePreference = "standard" | "soft" | "sharp" | "oled";
export type DensityPreference = "comfortable" | "compact";
export type GlassPreference = "off" | "soft" | "normal" | "strong";
export type MotionPreference = "standard" | "reduced" | "off";

export const SURFACE_PRESETS: ReadonlyArray<{ value: SurfacePreference; label: string }> = [
  { value: "standard", label: "标准" },
  { value: "soft", label: "柔和" },
  { value: "sharp", label: "硬朗" },
  { value: "oled", label: "纯黑" },
];

export const DENSITY_OPTIONS: ReadonlyArray<{ value: DensityPreference; label: string }> = [
  { value: "comfortable", label: "舒适" },
  { value: "compact", label: "紧凑" },
];

export const GLASS_OPTIONS: ReadonlyArray<{ value: GlassPreference; label: string }> = [
  { value: "off", label: "关闭" },
  { value: "soft", label: "轻" },
  { value: "normal", label: "标准" },
  { value: "strong", label: "强" },
];

export const MOTION_OPTIONS: ReadonlyArray<{ value: MotionPreference; label: string }> = [
  { value: "standard", label: "标准" },
  { value: "reduced", label: "减弱" },
  { value: "off", label: "关闭" },
];

export const surfacePref = enumPref<SurfacePreference>(
  "ui.surface",
  "surface",
  SURFACE_PRESETS.map((p) => p.value),
  "standard",
);

export const densityPref = enumPref<DensityPreference>(
  "ui.density",
  "density",
  DENSITY_OPTIONS.map((p) => p.value),
  "comfortable",
);

export const glassPref = enumPref<GlassPreference>(
  "ui.glass",
  "glass",
  GLASS_OPTIONS.map((p) => p.value),
  "normal",
);

export const motionPref = enumPref<MotionPreference>(
  "ui.motion",
  "motion",
  MOTION_OPTIONS.map((p) => p.value),
  "standard",
);
