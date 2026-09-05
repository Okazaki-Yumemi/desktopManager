<script lang="ts">
  let {
    title,
    milestone,
    description,
  }: { title: string; milestone: string; description: string } = $props();
</script>

<div class="placeholder">
  <h2>{title}</h2>
  <p class="description">{description}</p>
  <span class="badge">{milestone}</span>
</div>

<style>
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
    max-width: 480px;
    margin: 12vh auto 0;
    text-align: left;
  }

  h2 {
    margin: 0;
    font-size: var(--font-size-xl);
    font-weight: 600;
  }

  .description {
    margin: 0;
    color: var(--text-secondary);
  }

  .badge {
    padding: 2px var(--space-2);
    border: 1px solid var(--border);
    border-radius: 999px;
    font-size: var(--font-size-s);
    color: var(--text-tertiary);
  }
</style>
