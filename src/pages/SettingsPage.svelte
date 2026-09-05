<script lang="ts">
  import {
    getThemePreference,
    setThemePreference,
    type ThemePreference,
  } from "../stores/theme.svelte";
  import { pushToast } from "../stores/toast.svelte";

  const options: ReadonlyArray<{ value: ThemePreference; label: string }> = [
    { value: "system", label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ];

  const current = $derived(getThemePreference());

  async function choose(value: ThemePreference) {
    try {
      await setThemePreference(value);
    } catch (err) {
      pushToast("error", `Could not save theme: ${String(err)}`);
    }
  }
</script>

<div class="settings">
  <h1>Settings</h1>

  <section class="group" aria-label="Appearance">
    <h2>Appearance</h2>
    <div class="row">
      <div class="row-text">
        <span class="row-title">Theme</span>
        <span class="row-desc">Persisted locally; “System” follows Windows automatically.</span>
      </div>
      <div class="segmented" role="radiogroup" aria-label="Theme">
        {#each options as o (o.value)}
          <button
            type="button"
            role="radio"
            aria-checked={current === o.value}
            class:active={current === o.value}
            onclick={() => choose(o.value)}
          >
            {o.label}
          </button>
        {/each}
      </div>
    </div>
  </section>

  <p class="note">More settings (accent color, density, performance mode) arrive with M7.</p>
</div>

<style>
  .settings {
    max-width: 720px;
    margin: 0 auto;
  }

  h1 {
    font-size: var(--font-size-xl);
    font-weight: 600;
    margin: 0 0 var(--space-5);
  }

  .group {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow);
    padding: var(--space-4) var(--space-5);
  }

  .group h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--font-size-l);
    font-weight: 600;
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }

  .row-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .row-title {
    font-weight: 500;
  }

  .row-desc {
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }

  .segmented {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    overflow: hidden;
  }

  .segmented button {
    border: none;
    background: var(--surface);
    padding: 6px 14px;
    cursor: pointer;
    color: var(--text-secondary);
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .segmented button + button {
    border-left: 1px solid var(--border);
  }

  .segmented button:hover {
    background: var(--surface-hover);
  }

  .segmented button.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .note {
    margin-top: var(--space-4);
    color: var(--text-tertiary);
    font-size: var(--font-size-s);
  }
</style>
