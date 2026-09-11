/// Custom wallpaper state: the current image is stored in the app data dir
/// by the backend (background.img) and served over the `bg` custom
/// protocol; a named library (wallpapers/<millis>.img) powers thumbnails
/// and auto-switching. Only opacity lives in settings.
/// Call `initWallpaper()` once at startup.
import { SvelteDate } from "svelte/reactivity";
import { invoke } from "@tauri-apps/api/core";
import {
  backgroundApply,
  backgroundLibraryAdd,
  backgroundLibraryList,
  backgroundLibraryRemove,
  getSetting,
  setSetting,
} from "../services/backend";

export const wallpaper = $state({ active: false, opacity: 0.35, url: "" });

export type WallpaperAutoMode = "off" | "startup" | "30m" | "1h" | "6h" | "day";
export type WallpaperAutoOrder = "seq" | "rand";

export const AUTO_MODES: ReadonlyArray<{ value: WallpaperAutoMode; label: string }> = [
  { value: "off", label: "关闭" },
  { value: "startup", label: "启动时" },
  { value: "30m", label: "30 分钟" },
  { value: "1h", label: "1 小时" },
  { value: "6h", label: "6 小时" },
  { value: "day", label: "每天" },
];

function isMode(v: unknown): v is WallpaperAutoMode {
  return v === "off" || v === "startup" || v === "30m" || v === "1h" || v === "6h" || v === "day";
}

function isOrder(v: unknown): v is WallpaperAutoOrder {
  return v === "seq" || v === "rand";
}

/** Library + auto-switch preferences. `current` is the applied library name
    (empty when the background was set outside the library). */
export const wallpaperLib = $state({
  names: [] as string[],
  current: "",
  autoMode: "off" as WallpaperAutoMode,
  autoOrder: "seq" as WallpaperAutoOrder,
});

export async function initWallpaper(): Promise<void> {
  const conf = await getSetting<{ opacity?: number } | null>("ui.background");
  if (conf) {
    wallpaper.active = true;
    wallpaper.opacity = conf.opacity ?? 0.35;
    // no-store on the protocol side; the version busts webview caches.
    wallpaper.url = `http://bg.localhost/background.img?v=${Date.now()}`;
  } else {
    wallpaper.active = false;
    wallpaper.url = "";
  }
}

/** Load library + auto-switch prefs (settings failures keep defaults). */
export async function loadWallpaperLibrary(): Promise<void> {
  try {
    wallpaperLib.names = await backgroundLibraryList();
  } catch {
    wallpaperLib.names = [];
  }
  try {
    const cur = await getSetting<string | null>("ui.wallpaperCurrent");
    wallpaperLib.current = typeof cur === "string" ? cur : "";
  } catch {
    wallpaperLib.current = "";
  }
  try {
    const auto = await getSetting<{ mode?: unknown; order?: unknown } | null>("ui.wallpaperAuto");
    if (auto) {
      if (isMode(auto.mode)) wallpaperLib.autoMode = auto.mode;
      if (isOrder(auto.order)) wallpaperLib.autoOrder = auto.order;
    }
  } catch {
    /* keep defaults */
  }
}

async function setAutoPrefs(): Promise<void> {
  try {
    await setSetting("ui.wallpaperAuto", {
      mode: wallpaperLib.autoMode,
      order: wallpaperLib.autoOrder,
    });
  } catch {
    /* in-session only */
  }
}

export async function setWallpaperAutoMode(mode: WallpaperAutoMode): Promise<void> {
  wallpaperLib.autoMode = mode;
  await setAutoPrefs();
}

export async function setWallpaperAutoOrder(order: WallpaperAutoOrder): Promise<void> {
  wallpaperLib.autoOrder = order;
  await setAutoPrefs();
}

/** Add an image to the library and make it the current background. */
export async function addWallpaper(file: File): Promise<void> {
  if (!file.type.startsWith("image/")) {
    throw new Error("请选择图片文件（PNG/JPEG/WebP）");
  }
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  // Chunked btoa to avoid call-stack limits on multi-MB images.
  let binary = "";
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) {
    binary += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  const dataB64 = btoa(binary);
  const name = await backgroundLibraryAdd(dataB64, file.type);
  await loadWallpaperLibrary();
  await applyWallpaper(name);
}

/** Make a library image current (copies it over background.img). */
export async function applyWallpaper(name: string): Promise<void> {
  await backgroundApply(name);
  wallpaperLib.current = name;
  try {
    await setSetting("ui.wallpaperCurrent", name);
  } catch {
    /* rotation bookkeeping is best-effort */
  }
  await initWallpaper();
}

export async function removeWallpaper(name: string): Promise<void> {
  await backgroundLibraryRemove(name);
  await loadWallpaperLibrary();
}

export async function clearWallpaper(): Promise<void> {
  await invoke("background_clear");
  wallpaper.active = false;
  wallpaper.url = "";
}

export async function setWallpaperOpacity(value: number): Promise<void> {
  wallpaper.opacity = value;
  await setSetting("ui.background", { opacity: value });
}

/** Pick the next library entry after `current` (seq) or a different random
    one (rand). Returns null when rotation is impossible. */
function nextName(): string | null {
  const names = wallpaperLib.names;
  if (names.length < 2) return null;
  const idx = names.indexOf(wallpaperLib.current);
  if (wallpaperLib.autoOrder === "rand") {
    let pick = idx;
    while (pick === idx) {
      pick = Math.floor(Math.random() * names.length);
    }
    return names[pick] ?? null;
  }
  return names[(idx + 1) % names.length] ?? null;
}

/** Rotate once, silently (used by the timer / startup / daily switch). */
export async function rotateWallpaper(): Promise<boolean> {
  const name = nextName();
  if (name === null) return false;
  try {
    await applyWallpaper(name);
    return true;
  } catch {
    return false;
  }
}

/** Auto-switch runner: startup + daily fire on app start; interval modes
    are checked on a 30 s tick against the *current* mode, so changing the
    mode in settings takes effect without an app restart. Interval progress
    is in-memory (a restart resets the interval — the cosmetic, honest
    direction); the daily switch is keyed by date in settings. */
export function startWallpaperRotation(): () => void {
  let lastRotate = Date.now();
  let lastDay = new SvelteDate().toDateString();
  void (async () => {
    await loadWallpaperLibrary();
    const mode = wallpaperLib.autoMode;
    if (mode === "startup" || mode === "day") {
      // Switch at most once per calendar day (keyed in settings so a daily
      // rotation survives restarts).
      const today = new SvelteDate().toDateString();
      try {
        const last = await getSetting<string | null>("ui.wallpaperLastSwitch");
        if (last !== today && wallpaperLib.names.length >= 2) {
          if (await rotateWallpaper()) {
            await setSetting("ui.wallpaperLastSwitch", today);
          }
        }
      } catch {
        /* skip this boot */
      }
    }
  })();
  const INTERVAL_MS: Partial<Record<WallpaperAutoMode, number>> = {
    "30m": 30 * 60_000,
    "1h": 60 * 60_000,
    "6h": 6 * 60 * 60_000,
  };
  const timer = setInterval(() => {
    const dueMs = INTERVAL_MS[wallpaperLib.autoMode];
    if (dueMs !== undefined) {
      if (Date.now() - lastRotate >= dueMs) {
        lastRotate = Date.now();
        if (wallpaperLib.names.length >= 2) void rotateWallpaper();
      }
      return;
    }
    // Daily mode: rotate when the calendar day flips while running.
    if (wallpaperLib.autoMode === "day") {
      const today = new SvelteDate().toDateString();
      if (today !== lastDay) {
        lastDay = today;
        if (wallpaperLib.names.length >= 2) void rotateWallpaper();
      }
    }
  }, 30_000);
  return () => clearInterval(timer);
}
