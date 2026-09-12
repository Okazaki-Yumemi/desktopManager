<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Toasts from "./components/Toasts.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import TodayPage from "./pages/TodayPage.svelte";
  import DesktopPage from "./pages/DesktopPage.svelte";
  import FocusPage from "./pages/FocusPage.svelte";
  import CalendarPage from "./pages/CalendarPage.svelte";
import TasksPage from "./pages/TasksPage.svelte";
import AssignmentsPage from "./pages/AssignmentsPage.svelte";
import SettingsPage from "./pages/SettingsPage.svelte";
  import { currentPage, navigate, PAGES, type PageId } from "./stores/router.svelte";
  import { palette, togglePalette } from "./stores/palette.svelte";
  import {
    densityPref,
    glassPref,
    iconSizePref,
    loadAccentPreference,
    loadCustomAccent,
    loadThemePreference,
    motionPref,
    surfacePref,
    watchSystemTheme,
  } from "./stores/theme.svelte";
import { loadSoundPreference } from "./lib/chime.svelte";
import { initWallpaper, startWallpaperRotation, wallpaper } from "./stores/wallpaper.svelte";
import { loadCanvasState } from "./stores/canvas.svelte";

  const page = $derived(currentPage());

  /** True when a keyboard shortcut should NOT fire (user is typing). */
  function isTypingTarget(e: KeyboardEvent): boolean {
    const t = e.target;
    return (
      t instanceof HTMLElement &&
      (t.tagName === "INPUT" ||
        t.tagName === "TEXTAREA" ||
        t.tagName === "SELECT" ||
        t.isContentEditable)
    );
  }

  function onGlobalKeydown(e: KeyboardEvent): void {
    if (e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "k") {
      e.preventDefault();
      togglePalette();
      return;
    }
    if (e.ctrlKey && !e.shiftKey && !e.altKey && !isTypingTarget(e)) {
      // Ctrl+1..6 jump to the Nth page.
      const n = Number(e.key);
      if (n >= 1 && n <= PAGES.length) {
        e.preventDefault();
        navigate(PAGES[n - 1]!.id as PageId);
      }
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onGlobalKeydown);
    void loadThemePreference();
    void loadAccentPreference();
    void surfacePref.load();
    void densityPref.load();
    void glassPref.load();
    void motionPref.load();
    void loadSoundPreference();
    void loadCustomAccent();
    void iconSizePref.load();
    void initWallpaper();
    void loadCanvasState();
    const stopWallpaperRotation = startWallpaperRotation();
    const unlistenTheme = watchSystemTheme();
    return () => {
      window.removeEventListener("keydown", onGlobalKeydown);
      unlistenTheme();
      stopWallpaperRotation();
    };
  });
</script>

<div class="shell">
  {#if wallpaper.active}
    <div
      class="bg-layer"
      aria-hidden="true"
      style={`background-image: url('${wallpaper.url}'); opacity: ${wallpaper.opacity};`}
    ></div>
  {/if}
  <Sidebar />
  <main class="content">
    {#if page === "today"}
      <TodayPage />
    {:else if page === "desktop"}
      <DesktopPage />
    {:else if page === "focus"}
      <FocusPage />
    {:else if page === "calendar"}
      <CalendarPage />
    {:else if page === "tasks"}
      <TasksPage />
    {:else if page === "assignments"}
      <AssignmentsPage />
    {:else if page === "settings"}
      <SettingsPage />
    {/if}
  </main>
  <Toasts />
  {#if palette.open}
    <CommandPalette />
  {/if}
</div>

<style>
  .shell {
    position: relative;
    display: flex;
    height: 100%;
  }

  .bg-layer {
    position: absolute;
    inset: 0;
    z-index: 0;
    background-size: cover;
    background-position: center;
    background-repeat: no-repeat;
    pointer-events: none;
  }

  /* Keep both shell columns above the wallpaper layer. */
  .shell :global(.sidebar) {
    position: relative;
    z-index: 1;
  }

  .content {
    position: relative;
    z-index: 1;
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: var(--space-6);
  }
</style>
