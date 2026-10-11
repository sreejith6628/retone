# Retone

**Select text anywhere, press a shortcut, get it back polished — using the AI you already pay for.**

Retone is a small Windows and macOS app. It rewrites the text you've selected in any app — Teams, Outlook, Slack, Gmail, WhatsApp Web — and pastes the result back in place. It works with:

| Provider | What you need | Extra cost |
|---|---|---|
| **Claude** | Claude Pro/Max + [Claude Code](https://claude.com/claude-code) signed in | none |
| **ChatGPT** | ChatGPT Plus/Pro + [Codex CLI](https://github.com/openai/codex) signed in | none |
| **Gemini (API key)** | A free key from [Google AI Studio](https://aistudio.google.com/apikey) | free tier |
| Anthropic / OpenAI / OpenRouter | An API key | pay per use |
| Gemini CLI | An API key, Workspace or Vertex account (personal Google sign-in is no longer supported by Google) | depends |

No Retone account, no subscription, no tracking.

## Using it

- **Quick polish:** select text → `Ctrl+Shift+P` (Windows) / `⌥⌘P` (Mac), or click **✨ Polish** on the floating button.
- **Polish with options:** `Ctrl+Shift+O` / `⌥⌘O`, or click **▼** on the button — pick a tone (Professional, Friendly, Formal, Casual, Concise, Persuasive, Apologetic, Assertive, Grateful), length, add context ("this goes to my CEO"), or choose a different AI for this one message.
- **Write from scratch:** open options with nothing selected, describe the message, press Enter — it's typed where your cursor is.
- **Undo:** `Ctrl+Z` / `⌘Z` in your app brings your draft back.
- The floating button can be dragged anywhere; right-click it for the menu. Retone also lives in the system tray / menu bar.

## Install

Download the installer from the [Releases](../../releases) page.

These builds are not code-signed yet:
- **Windows:** if SmartScreen says "Windows protected your PC", click **More info → Run anyway**.
- **macOS:** right-click Retone → **Open** the first time. Then allow it in **System Settings → Privacy & Security → Accessibility** (needed to press ⌘C/⌘V for you). The first-run setup links you there.

## Setting up a subscription provider

1. Install the provider's official app (Retone shows the exact command), e.g. `npm install -g @anthropic-ai/claude-code`.
2. In Retone → Settings → AI providers, click **Sign in…** — a terminal opens with the provider's own login. Retone never sees your password or tokens.
3. Click **Re-check**, then **Test**.

Retone runs these tools with their file/command tools switched off (Claude, Gemini) or in read-only mode (Codex), inside an empty temporary folder, and asks them only to rewrite text.

## Privacy & security

- Your text goes only to the provider you choose. Nothing is sent anywhere else; there is no telemetry.
- API keys are stored in Windows Credential Manager / macOS Keychain, never in a file.
- Selected text is clearly marked to the AI as text to rewrite, not instructions to follow. Codex's read-only sandbox can still read files if a model were manipulated; the result is always shown to you in place.
- Codex and Gemini may keep their own local session logs; Retone asks Codex not to (`--ephemeral`).
- Clipboard: Retone restores text and image clipboard contents after copying/pasting. Rich formats (e.g. formatted Outlook text, files) on the clipboard may be lost.

## Build from source

Requirements: Node 22, Rust (stable), and on Windows the MSVC build tools + Windows SDK; on macOS Xcode Command Line Tools.

```bash
npm install
npm run tauri dev      # run
cargo test -p retone-core
npm run tauri build    # installers
```

Code layout: `crates/retone-core` (all testable logic: prompts, providers, CLI runner, capture, jobs, settings), `src-tauri` (desktop shell and OS adapters), `src` (Svelte UI). Design: `docs/superpowers/specs/`.

## License

MIT
