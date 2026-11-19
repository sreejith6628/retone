# Contributing to Retone

Thanks for helping out. Retone is a Tauri 2 + Svelte + Rust desktop app. Read [docs/architecture.md](docs/architecture.md) first for the lay of the land.

## Dev setup

Requirements: Node 22, Rust (stable); on Windows the MSVC build tools + Windows SDK; on macOS the Xcode Command Line Tools.

```bash
npm ci
npm run tauri dev          # run the app with hot reload
cargo test -p retone-core  # core unit + integration tests (needs Node.js on PATH for the fake CLI)
npm run check              # svelte-check / TypeScript
npm run build              # build the frontend
cargo build -p retone      # compile the desktop shell for your OS
```

CI runs `npm run check`, `npm run build`, `cargo test -p retone-core` and `cargo build -p retone` on Windows and macOS. Please make sure they pass locally. See [docs/testing.md](docs/testing.md) for manual smoke tests against real providers.

## Branches and pull requests

- Fork, then branch from `main` with a short descriptive name (`fix-pill-transparency`, `feat-new-provider`).
- Keep PRs focused: one change per PR.
- Use conventional-style commit messages (`fix(ui): ...`, `feat(core): ...`, `docs: ...`).
- Add or update tests for behaviour changes in `retone-core`.
- Fill in the pull request template.
- Do not include real API keys, personal paths or private text in code, tests or issues.

## Adding a provider

Providers live in `crates/retone-core/src/providers/`. Each one implements the `Provider` trait (`complete` and `status`) from `providers/mod.rs`.

1. Add the implementation: a CLI provider like `claude_cli.rs`, or an HTTP provider (OpenAI-compatible ones can reuse `openai_compat.rs` with a different base URL).
2. Register it in `providers/registry.rs`: add a `ProviderId` variant, include it in `ProviderId::ALL`, and fill in `key()`, `label()`, `is_cli()`, `key_url()` and `build_provider()`.
3. Add curated models to `crates/retone-core/resources/models.json` (every provider needs at least one; the registry test checks this).
4. For CLI providers, document the verified flags in `docs/cli-flags.md`. Never use flags that auto-approve tools or edits.
5. Add tests (see `tests/http_providers.rs` and `tests/fixtures/fake_cli.cjs` for the patterns) and surface the provider in the Svelte settings UI (`src/lib/`) if needed.

## Reporting security issues

Please do not open public issues for vulnerabilities. See [SECURITY.md](SECURITY.md).
