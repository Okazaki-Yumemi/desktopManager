/// Custom wallpaper state: image is stored in the app data dir by the
/// backend and served over the `bg` custom protocol; only opacity lives in
/// settings. Call `initWallpaper()` once at startup.
import { invoke } from "@tauri-apps/api/core";
import { getSetting, setSetting } from "../services/backend";

export const wallpaper = $state({ active: false, opacity: 0.35, url: "" });

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

export async function uploadWallpaper(file: File): Promise<void> {
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
  await invoke("background_set", { dataB64, mime: file.type });
  await initWallpaper();
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
