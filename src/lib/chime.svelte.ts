import { getSetting, setSetting } from "../services/backend";

/**
 * Focus phase chime built on WebAudio — no audio assets to ship. Kept as a
 * `.svelte.ts` module so the enabled flag is reactive for the Settings UI.
 */

export const SOUND_SETTING_KEY = "ui.sound";

let enabled = $state(true);
let ctx: AudioContext | null = null;

export function isSoundEnabled(): boolean {
  return enabled;
}

export async function loadSoundPreference(): Promise<void> {
  try {
    const saved = await getSetting<boolean>(SOUND_SETTING_KEY);
    if (typeof saved === "boolean") enabled = saved;
  } catch {
    // Backend unavailable (e.g. plain browser dev): keep the default (on).
  }
}

export async function setSoundEnabled(v: boolean): Promise<void> {
  enabled = v;
  await setSetting(SOUND_SETTING_KEY, v);
}

/**
 * Two-tone chime: focus completion rises E5→A5, break end falls A5→E5.
 * Autoplay policy may suspend a context not created in a user gesture;
 * resume opportunistically and give up silently — a missed chime must never
 * break the focus flow.
 */
export function playChime(kind: "focus" | "break" = "focus"): void {
  if (!enabled) return;
  try {
    const Ctor =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!Ctor) return;
    ctx ??= new Ctor();
    const audio = ctx;
    void audio.resume().catch(() => {});
    if (audio.state === "suspended") return;
    const t0 = audio.currentTime + 0.01;
    const notes = kind === "focus" ? [659.25, 880] : [880, 659.25];
    for (const [i, freq] of notes.entries()) {
      const osc = audio.createOscillator();
      const gain = audio.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      const start = t0 + i * 0.18;
      const stop = start + 0.5;
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(0.16, start + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, stop);
      osc.connect(gain).connect(audio.destination);
      osc.start(start);
      osc.stop(stop + 0.05);
      osc.onended = () => {
        osc.disconnect();
        gain.disconnect();
      };
    }
  } catch {
    // Audio is best-effort decoration.
  }
}
