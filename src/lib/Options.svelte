<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, type OptionsInit } from "./api";
  import ModelPicker from "./ModelPicker.svelte";

  let init = $state<OptionsInit | null>(null);
  let tone = $state("Standard");
  let length = $state("same");
  let context = $state("");
  let provider = $state("");
  let model = $state("");
  let error = $state("");
  let sending = false;
  let ctxEl = $state<HTMLTextAreaElement>();

  let current = $derived(init?.providers.find((p) => p.key === provider));

  onMount(() => {
    api.optionsInit().then((i) => {
      init = i;
      tone = i.default_tone;
      provider = i.provider;
      model = i.models[i.provider] ?? "";
      setTimeout(() => ctxEl?.focus(), 50);
    });
    const un = getCurrentWindow().onCloseRequested(() => { if (!sending) api.optionsCancel(); });
    return () => { un.then((u) => u()); };
  });

  function changeProvider(e: Event) {
    provider = (e.target as HTMLSelectElement).value;
    model = init?.models[provider] ?? "";
  }

  async function go() {
    error = "";
    try {
      sending = true;
      await api.optionsGo({ tone, length, context, provider, model: model || null });
    } catch (e) {
      sending = false;
      error = String(e);
    }
  }

  function keys(e: KeyboardEvent) {
    if (e.key === "Escape") { e.preventDefault(); api.optionsCancel(); }
    else if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); go(); }
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === "Escape") api.optionsCancel(); }} />

{#if init}
  <main>
    <p class="head">
      {#if init.has_draft}Polishing your selected text ({init.chars} characters){:else}Nothing selected — Retone will write a new message from your context{/if}
    </p>

    <section>
      <div class="caps">Tone</div>
      <div class="chips">
        {#each init.tones as t}
          <button class:on={tone === t} onclick={() => (tone = t)}>{t}</button>
        {/each}
      </div>
    </section>

    <section>
      <div class="caps">Length</div>
      <div class="seg">
        {#each [["shorter", "Shorter"], ["same", "Same"], ["longer", "Longer"]] as [v, l]}
          <button class:on={length === v} onclick={() => (length = v)}>{l}</button>
        {/each}
      </div>
    </section>

    <section>
      <div class="caps">Extra context <span class="muted">(optional)</span></div>
      <textarea bind:this={ctxEl} bind:value={context} rows="3" onkeydown={keys}
        placeholder="e.g. replying to an upset client · this goes to my CEO · mention a call on Monday"></textarea>
      <div class="small muted">Enter = polish · Shift+Enter = new line · Esc = cancel</div>
    </section>

    <section class="row">
      <div class="grow">
        <div class="caps">AI for this message</div>
        <select value={provider} onchange={changeProvider}>
          {#each init.providers as p}<option value={p.key}>{p.label}</option>{/each}
        </select>
      </div>
      <div class="grow">
        <div class="caps">Model</div>
        {#if current}{#key provider}<ModelPicker models={current.models} bind:value={model} />{/key}{/if}
      </div>
    </section>

    {#if error}<p class="error">{error}</p>{/if}

    <footer>
      <button class="primary" onclick={go}>✨ {init.has_draft ? "Polish" : "Write it"}</button>
      <button onclick={() => api.optionsCancel()}>Cancel</button>
    </footer>
  </main>
{/if}

<style>
  main { padding: 16px 18px; display: flex; flex-direction: column; gap: 14px; }
  .head { margin: 0; color: var(--accent); font-weight: 600; }
  section { display: flex; flex-direction: column; gap: 6px; }
  .chips { display: grid; grid-template-columns: repeat(5, 1fr); gap: 6px; }
  .chips button, .seg button { padding: 6px 4px; font-size: 13px; }
  .seg { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; max-width: 300px; }
  button.on { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); font-weight: 600; }
  .row { flex-direction: row; gap: 10px; }
  .grow { flex: 1; display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  footer { display: flex; gap: 8px; }
  .error { margin: 0; color: var(--bad); }
</style>
