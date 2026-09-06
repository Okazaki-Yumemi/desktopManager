<script lang="ts">
  import { AlertCircle, CheckCircle2, Info } from "@lucide/svelte";
  import { currentToasts, dismissToast, type ToastKind } from "../stores/toast.svelte";

  const toasts = $derived(currentToasts());

  const ICONS: Record<ToastKind, typeof Info> = {
    ok: CheckCircle2,
    error: AlertCircle,
    info: Info,
  };
</script>

{#if toasts.length > 0}
  <div class="toasts" role="status" aria-live="polite">
    {#each toasts as t (t.id)}
      {@const Icon = ICONS[t.kind]}
      <button
        type="button"
        class={`toast ${t.kind} toast-enter`}
        onclick={() => dismissToast(t.id)}
        title="点击关闭"
      >
        <Icon size={15} aria-hidden="true" />
        <span>{t.message}</span>
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
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    text-align: left;
    border: 1px solid var(--border-strong);
    border-left-width: 3px;
    border-radius: var(--radius-m);
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    box-shadow: var(--shadow-md);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    overflow-wrap: anywhere;
    transition: transform var(--duration-fast) var(--ease-out),
      box-shadow var(--duration-fast) var(--ease-out);
  }

  .toast:hover {
    transform: translateX(-2px);
    box-shadow: var(--shadow-lg);
  }

  .toast.info {
    border-left-color: var(--accent);
    color: var(--text-primary);
  }

  .toast.info :global(svg) {
    color: var(--accent);
    flex-shrink: 0;
    margin-top: 2px;
  }

  .toast.error {
    border-left-color: var(--error);
    color: var(--text-primary);
  }

  .toast.error :global(svg) {
    color: var(--error);
    flex-shrink: 0;
    margin-top: 2px;
  }

  .toast.ok {
    border-left-color: var(--ok);
    color: var(--text-primary);
  }

  .toast.ok :global(svg) {
    color: var(--ok);
    flex-shrink: 0;
    margin-top: 2px;
  }
</style>
