<script lang="ts">
  import {
    CalendarDays,
    LayoutGrid,
    ListTodo,
    Settings,
    Sun,
    Timer,
  } from "@lucide/svelte";
  import { PAGES, currentPage, navigate, type PageId } from "../stores/router.svelte";

  const active = $derived(currentPage());

  const ICONS: Record<PageId, typeof Sun> = {
    today: Sun,
    desktop: LayoutGrid,
    focus: Timer,
    calendar: CalendarDays,
    tasks: ListTodo,
    settings: Settings,
  };
</script>

<nav class="sidebar" aria-label="主导航">
  <div class="brand">
    <span class="brand-mark" aria-hidden="true">
      <svg viewBox="0 0 12 12" width="12" height="12">
        <rect x="1" y="1" width="4.2" height="4.2" rx="1.2" fill="#fff" opacity="0.95" />
        <rect x="6.8" y="1" width="4.2" height="4.2" rx="1.2" fill="#fff" opacity="0.65" />
        <rect x="1" y="6.8" width="4.2" height="4.2" rx="1.2" fill="#fff" opacity="0.65" />
        <rect x="6.8" y="6.8" width="4.2" height="4.2" rx="1.2" fill="#fff" opacity="0.95" />
      </svg>
    </span>
    <span class="brand-name">DesktopManager</span>
  </div>
  <ul>
    {#each PAGES as p (p.id)}
      {@const Icon = ICONS[p.id]}
      <li>
        <button
          type="button"
          class="nav-item"
          class:active={active === p.id}
          aria-current={active === p.id ? "page" : undefined}
          onclick={() => navigate(p.id)}
        >
          <Icon size={16} class="ico" aria-hidden="true" />
          <span>{p.label}</span>
        </button>
      </li>
    {/each}
  </ul>
  <div class="sidebar-footer">v1.1.0 · M13</div>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    width: 228px;
    flex-shrink: 0;
    background: var(--glass);
    backdrop-filter: var(--glass-filter);
    border-right: 1px solid var(--border);
    padding: var(--space-4);
    gap: var(--space-5);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    font-weight: 600;
    font-size: var(--font-size-l);
    letter-spacing: -0.01em;
  }

  .brand-mark {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-m);
    background: var(--grad-accent);
    box-shadow: var(--shadow-sm);
    flex-shrink: 0;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    text-align: left;
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius-m);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background var(--duration-fast) var(--ease-out),
      color var(--duration-fast) var(--ease-out);
  }

  .nav-item :global(svg) {
    color: var(--text-tertiary);
    transition: color var(--duration-fast) var(--ease-out);
    flex-shrink: 0;
  }

  .nav-item:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .nav-item:hover :global(svg) {
    color: var(--text-secondary);
  }

  .nav-item.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .nav-item.active :global(svg) {
    color: var(--accent);
  }

  .sidebar-footer {
    margin-top: auto;
    padding: var(--space-2);
    border-top: 1px solid var(--border);
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }
</style>
