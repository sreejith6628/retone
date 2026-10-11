<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, statusView, TONES, type ProviderInfo, type Settings } from "./api";
  import ProviderCard from "./ProviderCard.svelte";
  import ShortcutInput from "./ShortcutInput.svelte";
  import ModelPicker from "./ModelPicker.svelte";

  let settings = $state<Settings | null>(null);
  let providers = $state<ProviderInfo[]>([]);
  let statuses = $state<Record<string, string>>({});
  let tab = $state("general");
  let mac = $state(false);
  let version = $state("");
  let saveMsg = $state("");
  let saveWarn = $state<string[]>([]);
  let setup = $state(false);
  let step = $state(1);
  let axOk = $state(true);
  let trial = $state("hi team, pls share the deck by tmrw eod, client meet is on fri");
  let trialOut = $state("");
  let trialBusy = $state(false);

  // Gemini CLI's status check runs a real (paid) prompt, so it is only checked on demand.
  const onDemand = new Set(["gemini-cli"]);

  async function refresh(key?: string) {
    const keys = key ? [key] : providers.map((p) => p.key).filter((k) => !onDemand.has(k));
    for (const k of keys) {
      statuses[k] = "";
      api.providerStatus(k).then((s) => (statuses[k] = s)).catch((e) => (statuses[k] = "error:" + e));
    }
    for (const k of onDemand) if (!key && !(k in statuses)) statuses[k] = "unchecked";
  }

  onMount(async () => {
    [settings, providers, mac, version] = await Promise.all([
      api.getSettings(), api.providerList(), api.platform().then((p) => p === "macos"), getVersion()]);
    setup = !settings.first_run_done;
    refresh();
    if (mac) axOk = await api.accessibilityStatus();
  });

  let ready = $derived(providers.filter((p) => statuses[p.key] === "ready"));
  let defaultProvider = $derived(providers.find((p) => p.key === settings?.default_provider));

  // First run: if the default (Claude) turns out not to be ready, switch to the first ready one —
  // but only once its own check has finished, so a faster check elsewhere doesn't win the race.
  $effect(() => {
    if (!setup || !settings || !ready.length) return;
    const own = statuses[settings.default_provider];
    if (own && own !== "ready" && own !== "unchecked") settings.default_provider = ready[0].key;
  });

  async function save(): Promise<boolean> {
    if (!settings) return false;
    saveMsg = ""; saveWarn = [];
    try {
      const r = await api.saveSettings($state.snapshot(settings) as Settings);
      saveWarn = r.warnings;
      saveMsg = r.warnings.length ? "" : "Saved ✓";
      setTimeout(() => (saveMsg = ""), 2500);
      return true;
    } catch (e) { saveWarn = [String(e)]; return false; }
  }

  async function tryIt() {
    if (!settings) return;
    trialBusy = true; trialOut = "";
    try { trialOut = await api.testProvider(settings.default_provider, settings.models[settings.default_provider] || null, trial); }
    catch (e) { trialOut = "⚠ " + e; }
    trialBusy = false;
  }

  async function finish() {
    if (!(await save())) return;
    await api.finishFirstRun();
    if (settings) settings.first_run_done = true;
    setup = false;
    await getCurrentWindow().close();
  }

  async function resetStyle() { if (settings) settings.style = await api.defaultStyle(); }
</script>

{#if settings}
<div class="shell">
  {#if setup}
    <header class="welcome">
      <h1>Welcome to Retone</h1>
      <p class="muted">Select text anywhere, press a shortcut, and get it back polished — using the AI you already pay for.</p>
      <ol class="steps">
        <li class:on={step === 1}>1 · Choose your AI</li>
        <li class:on={step === 2}>2 · Try it</li>
        <li class:on={step === 3}>3 · Shortcuts{mac ? " & permission" : ""}</li>
      </ol>
    </header>
    <section class="body">
      {#if step === 1}
        <p>Retone works with a subscription you already have (via its official app), or with an API key — including Google's <b>free</b> Gemini key.</p>
        <div class="cards">
          {#each providers as p (p.key)}
            <label class="choose">
              <input type="radio" name="prov" value={p.key} bind:group={settings.default_provider} />
              <ProviderCard {p} bind:settings bind:status={statuses[p.key]} onrefresh={() => refresh(p.key)} />
            </label>
          {/each}
        </div>
      {:else if step === 2}
        <p>Your AI: <b>{defaultProvider?.label}</b> <span class="badge {statusView(statuses[settings.default_provider]).cls}">{statusView(statuses[settings.default_provider]).text}</span></p>
        <textarea rows="3" bind:value={trial}></textarea>
        <div><button class="primary" onclick={tryIt} disabled={trialBusy}>{trialBusy ? "Polishing…" : "✨ Polish this"}</button></div>
        {#if trialOut}<div class="result">{trialOut}</div>{/if}
      {:else}
        {#if mac}
          <div class="card">
            <b>Allow Retone to copy &amp; paste for you</b>
            <p class="small muted">macOS needs Accessibility permission so Retone can press ⌘C / ⌘V in the app you're typing in.</p>
            <div class="rowc">
              <span class="badge {axOk ? 'ready' : 'warn'}">{axOk ? "✅ Allowed" : "⚠️ Not allowed yet"}</span>
              <button onclick={() => api.openAccessibilitySettings()}>Open System Settings</button>
              <button onclick={async () => (axOk = await api.accessibilityStatus())}>Re-check</button>
            </div>
          </div>
        {/if}
        <div class="grid">
          <span>Quick polish</span><ShortcutInput {mac} bind:value={settings.shortcut_quick} />
          <span>Polish with options</span><ShortcutInput {mac} bind:value={settings.shortcut_options} />
        </div>
        <label class="check"><input type="checkbox" bind:checked={settings.launch_at_login} /> Start Retone when I log in</label>
        <label class="check"><input type="checkbox" bind:checked={settings.show_pill} /> Show the floating ✨ Polish button</label>
      {/if}
      {#each saveWarn as w}<p class="warn">{w}</p>{/each}
    </section>
    <footer>
      {#if step > 1}<button onclick={() => step--}>Back</button>{/if}
      <span class="grow"></span>
      {#if step < 3}
        <button class="primary" onclick={() => step++} disabled={step === 1 && statuses[settings.default_provider] !== "ready"}>Next</button>
      {:else}
        <button class="primary" onclick={finish}>Finish</button>
      {/if}
    </footer>
  {:else}
    <nav>
      {#each [["general", "General"], ["providers", "AI providers"], ["style", "Writing style"], ["shortcuts", "Shortcuts"], ["about", "About"]] as [k, l]}
        <button class:on={tab === k} onclick={() => (tab = k)}>{l}</button>
      {/each}
    </nav>
    <section class="body">
      {#if tab === "general"}
        <div class="grid">
          <span>Default AI</span>
          <div class="rowc">
            <select bind:value={settings.default_provider}>
              {#each providers as p}<option value={p.key}>{p.label}</option>{/each}
            </select>
            <span class="badge {statusView(statuses[settings.default_provider]).cls}">{statusView(statuses[settings.default_provider]).text}</span>
          </div>
          <span>Model</span>
          {#if defaultProvider}{#key defaultProvider.key}
            <ModelPicker models={defaultProvider.models} bind:value={settings.models[defaultProvider.key]} />
          {/key}{/if}
          <span>Default tone</span>
          <select bind:value={settings.default_tone}>{#each TONES as t}<option>{t}</option>{/each}</select>
        </div>
        <label class="check"><input type="checkbox" bind:checked={settings.show_pill} /> Show the floating ✨ Polish button</label>
        <label class="check"><input type="checkbox" bind:checked={settings.launch_at_login} /> Start Retone when I log in</label>
      {:else if tab === "providers"}
        <div class="cards">
          {#each providers as p (p.key)}
            <ProviderCard {p} bind:settings bind:status={statuses[p.key]} onrefresh={() => refresh(p.key)} />
          {/each}
        </div>
      {:else if tab === "style"}
        <p class="muted small">Instructions Claude, ChatGPT or Gemini follow every time. Tone, length and context from the options panel are added on top.</p>
        <textarea rows="14" class="mono" bind:value={settings.style}></textarea>
        <div><button onclick={resetStyle}>Reset to default</button></div>
      {:else if tab === "shortcuts"}
        <div class="grid">
          <span>Quick polish</span><ShortcutInput {mac} bind:value={settings.shortcut_quick} />
          <span>Polish with options</span><ShortcutInput {mac} bind:value={settings.shortcut_options} />
        </div>
        <p class="small muted">On the floating button: click to polish · ▼ for options · drag to move · right-click for the menu.</p>
        {#if mac}
          <div class="rowc">
            <span class="badge {axOk ? 'ready' : 'warn'}">Accessibility: {axOk ? "allowed" : "not allowed"}</span>
            <button onclick={() => api.openAccessibilitySettings()}>Open System Settings</button>
          </div>
        {/if}
      {:else}
        <h2>Retone {version}</h2>
        <p>Rewrite selected text anywhere with the AI you already have.</p>
        <p class="small muted">Privacy: Retone has no account, no tracking and no analytics. Your text goes only to the AI provider you choose. API keys live in your system's secure password store.</p>
        <button onclick={() => { setup = true; step = 1; }}>Run first-time setup again</button>
      {/if}
      {#each saveWarn as w}<p class="warn">{w}</p>{/each}
    </section>
    <footer>
      <span class="grow small muted">{saveMsg}</span>
      <button class="primary" onclick={save}>Save</button>
    </footer>
  {/if}
</div>
{/if}

<style>
  .shell { display: flex; flex-direction: column; height: 100vh; }
  nav { display: flex; gap: 4px; padding: 12px 16px 0; border-bottom: 1px solid var(--line); }
  nav button { border: none; border-bottom: 2px solid transparent; border-radius: 0; background: none; padding: 8px 12px; }
  nav button.on { border-bottom-color: var(--accent); color: var(--accent); font-weight: 600; }
  .body { flex: 1; overflow: auto; padding: 18px 20px; display: flex; flex-direction: column; gap: 14px; }
  footer { display: flex; gap: 8px; align-items: center; padding: 12px 20px; border-top: 1px solid var(--line); background: var(--panel); }
  .grow { flex: 1; }
  .grid { display: grid; grid-template-columns: 180px 1fr; gap: 10px 14px; align-items: center; }
  .rowc { display: flex; gap: 8px; align-items: center; }
  .cards { display: flex; flex-direction: column; gap: 10px; }
  .choose { display: flex; gap: 10px; align-items: flex-start; }
  .choose > input { margin-top: 18px; }
  .choose > :global(.card) { flex: 1; }
  .card { border: 1px solid var(--line); background: var(--panel); border-radius: var(--radius); padding: 14px; display: flex; flex-direction: column; gap: 6px; }
  .welcome { padding: 20px 20px 0; }
  .welcome h1 { margin: 0 0 4px; font-size: 22px; }
  .steps { display: flex; gap: 18px; list-style: none; padding: 0; margin: 14px 0 0; border-bottom: 1px solid var(--line); }
  .steps li { padding-bottom: 8px; color: var(--muted); border-bottom: 2px solid transparent; }
  .steps li.on { color: var(--accent); border-bottom-color: var(--accent); font-weight: 600; }
  .result { background: var(--ok-soft); padding: 10px 12px; border-radius: 8px; white-space: pre-wrap; }
  .warn { margin: 0; color: var(--bad); }
  .mono { font-family: Consolas, ui-monospace, monospace; font-size: 13px; }
  h2 { margin: 0; }
</style>
