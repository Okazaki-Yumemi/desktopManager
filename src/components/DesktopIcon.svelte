<script lang="ts">
  import type { Snippet } from "svelte";
  import { getIconDataUrl } from "../lib/iconCache";

  let {
    path,
    size = 22,
    fallback,
  }: { path: string; size?: number; fallback: Snippet } = $props();

  let url = $state<string | null>(null);

  $effect(() => {
    const p = path;
    url = null;
    void getIconDataUrl(p).then((u) => {
      if (p === path) url = u;
    });
  });
</script>

{#if url}
  <img class="shell-icon" src={url} width={size} height={size} alt="" aria-hidden="true" />
{:else}
  {@render fallback()}
{/if}

<style>
  .shell-icon {
    display: block;
    image-rendering: auto;
  }
</style>
