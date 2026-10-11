<script lang="ts">
  import type { ModelInfo } from "./api";
  let { models, value = $bindable() }: { models: ModelInfo[]; value: string | undefined } = $props();

  const CUSTOM = "__custom__";
  let custom = $state(false);
  $effect(() => { if (value && !models.some((m) => m.id === value)) custom = true; });
  let selected = $derived(custom ? CUSTOM : value || (models[0]?.id ?? ""));

  function pick(e: Event) {
    const v = (e.target as HTMLSelectElement).value;
    if (v === CUSTOM) { custom = true; value = ""; } else { custom = false; value = v; }
  }
</script>

<div class="mp">
  <select value={selected} onchange={pick}>
    {#each models as m}<option value={m.id}>{m.label}</option>{/each}
    <option value={CUSTOM}>Custom model ID…</option>
  </select>
  {#if custom}
    <input type="text" placeholder="e.g. claude-sonnet-5-5" bind:value />
  {/if}
</div>

<style>
  .mp { display: flex; gap: 8px; }
  .mp select { flex: 1; }
  .mp input { flex: 1; }
</style>
