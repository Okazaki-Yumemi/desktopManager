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
  import { loadThemePreference, watchSystemTheme } from "./stores/theme.svelte";

  const page = $derived(currentPage());

  onMount(() => {
    void loadThemePreference();
    return watchSystemTheme();
  });
</script>

<div class="shell">
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
    display: flex;
    height: 100%;
  }

  .content {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: var(--space-6);
  }
</style>
