<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, statusView, type ProviderInfo, type Settings } from "./api";
  import ModelPicker from "./ModelPicker.svelte";

  let { p, settings = $bindable(), status = $bindable(), onrefresh }:
    { p: ProviderInfo; settings: Settings; status: string | undefined; onrefresh: () => void } = $props();

  let keyInput = $state("");
  let hasKey = $state(false);
  let testing = $state(false);
  let testResult = $state("");
  let testOk = $state(true);
  let busyMsg = $state("");
  let view = $derived(statusView(status));

  $effect(() => { if (!p.is_cli) api.hasApiKey(p.key).then((h) => (hasKey = h)); });

  let model = $state(settings.models[p.key] ?? "");
  $effect(() => { settings.models[p.key] = model; });
  let path = $state(settings.cli_paths[p.key] ?? "");
  $effect(() => { if (path.trim()) settings.cli_paths[p.key] = path.trim(); else delete settings.cli_paths[p.key]; });

  async function saveKey() {
    busyMsg = "";
    try { await api.setApiKey(p.key, keyInput); keyInput = ""; hasKey = true; busyMsg = "Key saved securely."; onrefresh(); }
    catch (e) { busyMsg = String(e); }
  }
  async function removeKey() { await api.deleteApiKey(p.key); hasKey = false; onrefresh(); }
  async function test() {
    testing = true; testResult = "";
    const t0 = performance.now();
    try {
      const out = await api.testProvider(p.key, model || null);
      testOk = true; testResult = `${out}  (${((performance.now() - t0) / 1000).toFixed(1)} s)`;
    } catch (e) { testOk = false; testResult = String(e); }
    testing = false;
  }
</script>

<div class="card">
  <div class="top">
    <div>
      <div class="name">{p.label}</div>
      {#if p.note}<div class="small muted">{p.note}</div>{/if}
    </div>
    <span class="badge {view.cls}">{view.text}</span>
  </div>

  {#if p.is_cli}
    {#if status === "not_installed" && p.install_cmd}
      <div class="small">Install it, then press Re-check: <code>{p.install_cmd}</code></div>
    {/if}
    <div class="row">
      <button onclick={() => api.signIn(p.key)}>Sign in…</button>
      <button onclick={onrefresh}>Re-check</button>
    </div>
    <details>
      <summary class="small muted">Advanced: custom program path</summary>
      <input type="text" placeholder="Leave empty to find it automatically" bind:value={path} />
    </details>
  {:else}
    <div class="row">
      <input type="password" placeholder={hasKey ? "•••••••• (saved — paste a new key to replace)" : "Paste your API key"} bind:value={keyInput} />
      <button class="primary" disabled={!keyInput.trim()} onclick={saveKey}>Save key</button>
      {#if hasKey}<button onclick={removeKey}>Remove</button>{/if}
    </div>
    <div class="small">
      {#if p.key_url}<button class="link" onclick={() => openUrl(p.key_url!)}>Get a key ↗</button>{/if}
      <span class="muted">Stored in your system's secure password store, never in a file.</span>
    </div>
    {#if busyMsg}<div class="small muted">{busyMsg}</div>{/if}
  {/if}

  <div class="row">
    <div class="grow"><ModelPicker models={p.models} bind:value={model} /></div>
    <button onclick={test} disabled={testing}>{testing ? "Testing…" : "Test"}</button>
  </div>
  {#if testResult}<div class="test" class:bad={!testOk}>{testResult}</div>{/if}
</div>

<style>
  .card { border: 1px solid var(--line); background: var(--panel); border-radius: var(--radius); padding: 14px; display: flex; flex-direction: column; gap: 10px; }
  .top { display: flex; justify-content: space-between; gap: 12px; align-items: flex-start; }
  .name { font-weight: 600; }
  .row { display: flex; gap: 8px; align-items: center; }
  .grow { flex: 1; }
  details input { margin-top: 6px; }
  .test { font-size: 13px; background: var(--ok-soft); color: var(--ink); padding: 8px 10px; border-radius: 8px; white-space: pre-wrap; }
  .test.bad { background: var(--bad-soft); color: var(--bad); }
</style>
