# Retone

**Select text anywhere, press a shortcut, get it back polished — using the AI you already pay for.**

Retone is a small Windows and macOS app. It rewrites the text you've selected in any app — Teams, Outlook, Slack, Gmail, WhatsApp Web — and pastes the result back in place. No Retone account, no subscription, no tracking.

## How it works

1. You select text in any app and press a global shortcut (or click the floating button).
2. Retone copies the selection, sends it to the AI provider you chose, and waits for the rewrite.
3. It pastes the result over your selection and restores your clipboard.

See [docs/architecture.md](docs/architecture.md) for the details.

## Providers

| Provider | What you need | Extra cost |
|---|---|---|
| **Claude** | Claude Pro/Max + [Claude Code](https://claude.com/claude-code) signed in | none |
| **ChatGPT** | ChatGPT Plus/Pro + [Codex CLI](https://github.com/openai/codex) signed in | none |
| **Gemini (API key)** | A free key from [Google AI Studio](https://aistudio.google.com/apikey) | free tier |
| Anthropic / OpenAI / OpenRouter | An API key | pay per use |
| Gemini CLI | An API key, Workspace or Vertex account (personal Google sign-in is no longer supported by Google) | depends |

## Install

Download the installer for your OS from the [Releases](../../releases) page. These builds are not code-signed yet, so your OS will warn you the first time.

**Windows**

Run the installer. If SmartScreen says "Windows protected your PC", click **More info → Run anyway**.

**macOS**

1. Open the `.dmg` and drag **Retone** to **Applications**.
2. The first time you open it, macOS blocks it because the build isn't notarized. Click **Done**, then:
   - **macOS 15 (Sequoia) and later:** open **System Settings → Privacy & Security**, scroll down and click **Open Anyway** next to Retone.
   - **macOS 14 and earlier:** right-click (or Ctrl-click) Retone in Applications → **Open** → **Open**.
   - Or remove the quarantine flag in Terminal:
     ```bash
     xattr -dr com.apple.quarantine /Applications/Retone.app
     ```
3. Grant Accessibility permission in **System Settings → Privacy & Security → Accessibility** so Retone can press ⌘C / ⌘V to copy your selection and paste the result. The first-run setup links you there.
4. **After updating** to a new Retone build, macOS may treat it as a new app: if Retone says it needs Accessibility although the switch looks on, remove Retone from the list (**–**) and turn the new one on.

Retone sends the physical C and V keys with ⌘. On layouts where those keys type other letters (for example Dvorak), copy/paste may not work yet.

## Using it

Default shortcuts (you can change them in Settings):

| Action | Windows | macOS |
|---|---|---|
| Quick polish | `Ctrl+Shift+P` | `⌥⌘P` |
| Polish with options | `Ctrl+Shift+O` | `⌥⌘O` |

- **Quick polish:** select text → press the quick shortcut, or click **✨ Polish** on the floating button.
- **Polish with options:** press the options shortcut, or click **▼** on the button — pick a tone (Professional, Friendly, Formal, Casual, Concise, Persuasive, Apologetic, Assertive, Grateful), length, add context ("this goes to my CEO"), or choose a different AI for this one message.
- **Write from scratch:** open options with nothing selected, describe the message, press Enter — it's typed where your cursor is.
- **Undo:** `Ctrl+Z` / `⌘Z` in your app brings your draft back.
- The floating button can be dragged anywhere; right-click it for the menu. Retone also lives in the system tray / menu bar.

## Setting up a subscription provider

1. Install the provider's official app (Retone shows the exact command), e.g. `npm install -g @anthropic-ai/claude-code`.
2. In Retone → Settings → AI providers, click **Sign in…** — a terminal opens with the provider's own login. Retone never sees your password or tokens.
3. Click **Re-check**, then **Test**.

Retone runs these tools inside an empty temporary folder and asks them only to rewrite text. Claude Code runs with all tools disabled (`--tools ""`, no MCP servers, no user settings). Gemini CLI runs in read-only plan mode with a policy that denies every tool, including MCP tools. Codex runs in its read-only sandbox with connected apps, web search, the shell tool, plugins and sub-agents switched off. These are the tools' own controls, verified by hand against the versions listed in [docs/cli-flags.md](docs/cli-flags.md). Retone does not pin CLI versions: a newer version of a tool could add capabilities these flags don't cover.

## Privacy & security

- Your text goes only to the provider you choose. Nothing is sent anywhere else; there is no telemetry.
- API keys are stored in Windows Credential Manager / macOS Keychain, never in a file.
- Selected text is clearly marked to the AI as text to rewrite, not instructions to follow. With the tools switched off as above (on the CLI versions we verified), text you select should at worst change what the rewrite says, not make the AI run commands, read your files, browse, or use your connected accounts. This relies on each CLI's own controls, so check the result before you send it, especially when you polish text someone else wrote.
- Retone asks each tool not to keep a session history: Claude (`--no-session-persistence`), Codex (`--ephemeral`). Gemini CLI always records sessions, so Retone tries to delete its record after each job (best effort: if Gemini's project list can't be read, the record stays under `~/.gemini/tmp`). The CLIs may still send their own usage telemetry to their vendors; see each tool's settings.
- Retone's own clipboard writes are excluded from Windows clipboard history and cloud clipboard. On macOS they are marked as concealed, which clipboard managers that follow that convention skip; this does not stop Universal Clipboard. The selection itself is copied by your app (Retone's simulated Ctrl+C / ⌘C), so it follows your normal clipboard history settings.
- Clipboard: Retone restores text and image clipboard contents after copying/pasting. Rich formats (e.g. formatted Outlook text, files) on the clipboard may be lost.

To report a vulnerability, see [SECURITY.md](SECURITY.md).

## Build from source

Requirements: Node 22, Rust (stable), and on Windows the MSVC build tools + Windows SDK; on macOS Xcode Command Line Tools.

```bash
npm ci
npm run tauri dev      # run
cargo test -p retone-core
npm run tauri build    # installers
```

Code layout: `crates/retone-core` (all testable logic: prompts, providers, CLI runner, capture, jobs, settings), `src-tauri` (desktop shell and OS adapters), `src` (Svelte UI).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [docs/architecture.md](docs/architecture.md).

## License

[MIT](LICENSE)
