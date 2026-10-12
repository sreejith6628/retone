<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Pill from "$lib/Pill.svelte";
  import Options from "$lib/Options.svelte";
  import Settings from "$lib/Settings.svelte";

  // One SPA, three windows: render by window label.
  const label = getCurrentWindow().label;
  // Before first paint, so the pill window never shows an opaque frame.
  if (label === "pill") document.documentElement.classList.add("pill");
</script>

{#if label === "pill"}
  <Pill />
{:else if label === "options"}
  <Options />
{:else}
  <Settings />
{/if}
