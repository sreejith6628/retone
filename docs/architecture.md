# Architecture

Retone is a Cargo workspace plus a Svelte frontend: `crates/retone-core` (platform-free logic), `src-tauri` (the Tauri 2 desktop shell), and `src` (UI).

## retone-core (`crates/retone-core`)

All logic that can be tested without a desktop.

- `capture.rs` — copies the current selection and pastes results back. The OS clipboard and keystrokes come in through the `ClipboardIo` and `KeySender` traits. Copy is detected by putting a unique sentinel on the clipboard first; the user's clipboard is saved and restored around both copy and paste.
- `prompt.rs` — builds the system prompt and user message from the style, tone, length, context and draft. The draft is wrapped in `<draft>` tags so it is treated as text to rewrite.
- `job.rs` — one polish job: prompt, provider, paste. Reports `PillStatus` (working, done, hint, error, copied) through a `StatusSink`, pastes through a `Desktop` trait, and `Busy` allows only one job at a time.
- `providers/` — the `Provider` trait (`complete`, `status`). CLI providers (`claude_cli`, `codex_cli`, `gemini_cli`) run the user's installed tools via `procutil.rs`; API providers (`anthropic_api`, `openai_compat`, which also serves OpenAI, Gemini and OpenRouter) use HTTPS. `registry.rs` maps a `ProviderId` plus settings and secrets to a ready provider.
- `settings.rs` — persisted settings (including the default shortcuts per OS) and the `SecretStore` for API keys, backed by the OS credential store.
- `models.rs` and `resources/models.json` — curated model lists, embedded at build time.
- `procutil.rs` — safe child-process spawning: executable resolution, no console window, login-shell PATH on macOS, timeout with process-tree kill.
- `pillgeom.rs` — floating-button geometry.
- `bin/retone-dev.rs` — command-line harness for trying providers (see `docs/testing.md`).

## Desktop shell (`src-tauri`)

- `shortcuts.rs` — validates and registers the two global shortcuts (quick polish, options).
- `flows.rs` — orchestrates quick polish, the options panel and `run_job`, which builds the provider and calls `job::run`.
- `pill.rs` — the small always-on-top floating window: position, status text and click handling.
- `winmgr.rs` — on-demand settings and options windows.
- `menus.rs` — tray icon and the pill's right-click menu.
- `commands.rs`, `state.rs`, `applog.rs` — frontend commands, shared app state, and a small diagnostic log that never contains draft or result text.
- `platform/` — OS adapters: `windows.rs` (foreground-window target, low-level mouse hook for the non-activating pill, topmost upkeep) and `macos.rs` (remembers and reactivates the previously frontmost app, Accessibility permission check).

## UI (`src`)

Svelte pages and components for the pill, options panel and settings (`Pill.svelte`, `Options.svelte`, `Settings.svelte`, `ProviderCard.svelte`, `ModelPicker.svelte`, `ShortcutInput.svelte`), talking to the shell through `src/lib/api.ts`.

## Flow of a rewrite

1. A global shortcut (or pill click) fires in `shortcuts.rs`/`pill.rs` and calls into `flows.rs`.
2. The shell records the target app (`platform::current_target`) and copies the selection (`capture::copy_selection`), saving the user's clipboard and restoring it afterwards.
3. `run_job` builds the provider from the registry and calls `job::run`, which builds the prompt and asks the provider for the rewrite while the pill shows "working".
4. On success the shell reactivates the target app and pastes the result (`capture::paste_text`), then restores the clipboard. If the target is gone, the result is left on the clipboard and the pill says so.
5. On error the pill shows a short message; clicking it opens Settings.
