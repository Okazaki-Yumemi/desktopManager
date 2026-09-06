<script lang="ts">
  import { PAGES, currentPage, navigate } from "../stores/router.svelte";

  const active = $derived(currentPage());
</script>

<nav class="sidebar" aria-label="主导航">
  <div class="brand">
    <span class="brand-mark" aria-hidden="true"></span>
    <span class="brand-name">DesktopManager</span>
  </div>
  <ul>
    {#each PAGES as p (p.id)}
      <li>
        <button
          type="button"
          class="nav-item"
          class:active={active === p.id}
          aria-current={active === p.id ? "page" : undefined}
          onclick={() => navigate(p.id)}
        >
          {p.label}
        </button>
      </li>
    {/each}
  </ul>
  <div class="sidebar-footer">v1.0.1 · M12</div>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    width: 216px;
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
  }

  .brand-mark {
    width: 14px;
    height: 14px;
    border-radius: var(--radius-s);
    background: var(--accent);
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
    display: block;
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

  .nav-item:hover {
    background: var(--surface-hover);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .sidebar-footer {
    margin-top: auto;
    padding: var(--space-2);
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }
</style>
