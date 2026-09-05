<script lang="ts">
  import { currentToasts, dismissToast, type Toast } from "../stores/toast.svelte";

  const toasts = $derived(currentToasts());
</script>

{#if toasts.length > 0}
  <div class="toasts" role="status" aria-live="polite">
    {#each toasts as t (t.id)}
      <button
        type="button"
        class={`toast ${t.kind}`}
        onclick={() => dismissToast(t.id)}
        title="Click to dismiss"
      >
        {t.message}
      </button>
    {/each}
  </div>
{/if}

<style>
  .toasts {
    position: fixed;
    right: var(--space-4);
    bottom: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    z-index: 100;
    max-width: 420px;
  }

  .toast {
    text-align: left;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-m);
    background: var(--surface);
    box-shadow: var(--shadow);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    overflow-wrap: anywhere;
  }

  .toast.error {
    border-color: color-mix(in srgb, var(--error) 45%, transparent);
    color: var(--error);
  }

  .toast.ok {
    border-color: color-mix(in srgb, var(--ok) 45%, transparent);
    color: var(--ok);
  }
</style>
