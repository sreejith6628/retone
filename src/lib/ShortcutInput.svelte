<script lang="ts">
  let { value = $bindable(""), mac = false }: { value: string; mac: boolean } = $props();
  let recording = $state(false);

  const pretty = (s: string) => mac
    ? s.replace(/Super\+?/g, "⌘").replace(/Alt\+?/g, "⌥").replace(/Shift\+?/g, "⇧").replace(/Control\+?|Ctrl\+?/g, "⌃")
    : s;

  function key(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    if (e.key === "Escape") { recording = false; return; }
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
    const mods = [];
    if (e.ctrlKey) mods.push("Ctrl");
    if (e.altKey) mods.push("Alt");
    if (e.shiftKey) mods.push("Shift");
    if (e.metaKey) mods.push("Super");
    if (mods.length === 0) return; // a global shortcut needs at least one modifier
    const k = e.code.startsWith("Key") ? e.code.slice(3)
      : e.code.startsWith("Digit") ? e.code.slice(5)
      : e.code;
    value = [...mods, k].join("+");
    recording = false;
  }
</script>

<button class="sc" class:rec={recording} onclick={() => (recording = true)} onkeydown={key} onblur={() => (recording = false)}>
  {recording ? "Press the new shortcut… (Esc to cancel)" : pretty(value)}
</button>

<style>
  .sc { min-width: 220px; text-align: left; font-family: Consolas, ui-monospace, monospace; }
  .rec { border-color: var(--accent); background: var(--accent-soft); }
</style>
