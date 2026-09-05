<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Toasts from "./components/Toasts.svelte";
  import TodayPage from "./pages/TodayPage.svelte";
  import DesktopPage from "./pages/DesktopPage.svelte";
  import FocusPage from "./pages/FocusPage.svelte";
  import CalendarPage from "./pages/CalendarPage.svelte";
  import TasksPage from "./pages/TasksPage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";
  import { currentPage } from "./stores/router.svelte";
  import {
    densityPref,
    glassPref,
    loadAccentPreference,
    loadThemePreference,
    motionPref,
    surfacePref,
    watchSystemTheme,
  } from "./stores/theme.svelte";
  import { initWallpaper, wallpaper } from "./stores/wallpaper.svelte";

  const page = $derived(currentPage());

  onMount(() => {
    void loadThemePreference();
    void loadAccentPreference();
    void surfacePref.load();
    void densityPref.load();
    void glassPref.load();
    void motionPref.load();
    void initWallpaper();
    return watchSystemTheme();
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
    {:else if page === "settings"}
      <SettingsPage />
    {/if}
  </main>
  <Toasts />
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
