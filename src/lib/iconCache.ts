import { invoke } from "@tauri-apps/api/core";

/// Frontend side of the lazy bounded icon cache: backend extracts shell
/// icons on demand (its own LRU), we cache the encoded data-URLs per session
/// and fall back to a generic glyph on any failure.
const MAX_ENTRIES = 512;
const cache = new Map<string, string>();

export async function getIconDataUrl(path: string): Promise<string | null> {
  const hit = cache.get(path);
  if (hit !== undefined) {
    // Refresh recency (Map iterates in insertion order).
    cache.delete(path);
    cache.set(path, hit);
    return hit;
  }
  try {
    const payload = await invoke<{ width: number; height: number; rgba: string } | null>(
      "desktop_icon",
      { path },
    );
    if (!payload) return null;
    const url = rgbaToPngDataUrl(payload);
    if (!url) return null;
    if (cache.size >= MAX_ENTRIES) {
      const oldest = cache.keys().next().value;
      if (oldest !== undefined) cache.delete(oldest);
    }
    cache.set(path, url);
    return url;
  } catch {
    return null;
  }
}

function rgbaToPngDataUrl(payload: {
  width: number;
  height: number;
  rgba: string;
}): string | null {
  const canvas = document.createElement("canvas");
  canvas.width = payload.width;
  canvas.height = payload.height;
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;
  const raw = Uint8Array.from(atob(payload.rgba), (c) => c.charCodeAt(0));
  const img = new ImageData(new Uint8ClampedArray(raw.buffer), payload.width, payload.height);
  ctx.putImageData(img, 0, 0);
  return canvas.toDataURL("image/png");
}
