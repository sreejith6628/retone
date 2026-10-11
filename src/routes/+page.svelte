<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Pill from "$lib/Pill.svelte";
  import Options from "$lib/Options.svelte";
  import Settings from "$lib/Settings.svelte";

  // One SPA, three windows: render by window label.
  const label = getCurrentWindow().label;
</script>

{#if label === "pill"}
  <Pill />
{:else if label === "options"}
  <Options />
{:else}
  <Settings />
{/if}
