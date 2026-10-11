<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api } from "./api";

  let text = $state("✨ Polish");
  let kind = $state("idle");
  let el: HTMLDivElement;
  // Mouse handling here is used on macOS; on Windows the native hook handles clicks
  // (the webview is display-only so it never steals focus).
  let down: { x: number; y: number } | null = null;
  let dragging = false;
  let saveTimer: ReturnType<typeof setTimeout> | undefined;

  onMount(() => {
    document.body.classList.add("pill-body");
    const offs: Array<() => void> = [];
    listen<{ text: string; kind: string }>("pill-status", (e) => { text = e.payload.text; kind = e.payload.kind; })
      .then((u) => offs.push(u));
    getCurrentWindow().onMoved(({ payload }) => {
      clearTimeout(saveTimer);
      saveTimer = setTimeout(() => api.pillMoved(payload.x, payload.y), 400);
    }).then((u) => offs.push(u));
    return () => offs.forEach((u) => u());
  });

  function onDown(e: MouseEvent) {
    if (e.button !== 0) return;
    down = { x: e.screenX, y: e.screenY };
    dragging = false;
  }
  async function onMove(e: MouseEvent) {
    if (!down || dragging) return;
    if (Math.abs(e.screenX - down.x) > 4 || Math.abs(e.screenY - down.y) > 4) {
      dragging = true;
      down = null;
      await getCurrentWindow().startDragging();
    }
  }
  function onUp(e: MouseEvent) {
    if (!down) return;
    down = null;
    if (dragging) return;
    const r = el.getBoundingClientRect();
    api.pillClick(e.clientX - r.left >= r.width * 0.8 ? "menu" : "main");
  }
  function onContext(e: MouseEvent) {
    e.preventDefault();
    api.pillRightClick();
  }
</script>

<div class="pill {kind}" bind:this={el} role="button" tabindex="-1"
     onmousedown={onDown} onmousemove={onMove} onmouseup={onUp} oncontextmenu={onContext}>
  <span class="grip">⋮</span>
  <span class="label">{text}</span>
  <span class="sep"></span>
  <span class="arrow">▼</span>
</div>

<style>
  .pill {
    position: fixed; inset: 0; display: flex; align-items: center;
    border-radius: 15px; background: #d97757; color: #fff; overflow: hidden;
    font: 600 13px "Segoe UI", -apple-system, system-ui, sans-serif;
    user-select: none; cursor: default; white-space: nowrap;
  }
  .pill.working { background: #b9614a; }
  .pill.done { background: #2f7d4f; }
  .pill.hint { background: #3d3a36; }
  .pill.error { background: #b3261e; cursor: pointer; }
  .grip { width: 18px; text-align: center; opacity: .75; flex: none; }
  .label { flex: 1; text-align: center; overflow: hidden; text-overflow: ellipsis; padding: 0 4px; }
  .sep { width: 1px; height: 16px; background: rgba(255, 255, 255, .45); flex: none; }
  .arrow { width: 25px; text-align: center; font-size: 9px; flex: none; }
</style>
