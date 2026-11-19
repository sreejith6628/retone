# Testing

## Automated
`cargo test -p retone-core` runs the unit and integration tests (needs Node.js on PATH for the fake CLI used in tests).
`npm run check` type-checks the Svelte UI. CI runs both on Windows and macOS.

## Trying a provider from the command line
`retone-dev` runs one polish through a provider without the desktop app:

```bash
echo "can u send me the report by eod thx" | cargo run -q -p retone-core --bin retone-dev -- <provider> [--model M] [--tone T] [--length L] [--context C]
```

`<provider>` is one of the provider keys in `crates/retone-core/src/providers/registry.rs` (for example `claude-cli`, `codex-cli`, `gemini-cli`).

## Desktop app
- Windows: run through quick polish (shortcut and pill), polish with options, clipboard restore, and the tray menu in Notepad, a browser text box and a chat app.
- macOS: follow [macos-testing.md](macos-testing.md).
