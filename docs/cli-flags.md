# CLI flags (verified 2026-10-11, Windows)

## Claude Code — 2.1.294
`claude -p --model <m> --tools "" --setting-sources "" --strict-mcp-config --no-session-persistence --system-prompt-file <f>`; draft on stdin.
Status: `claude auth status` → JSON with `loggedIn`.

## Codex CLI — codex-cli 0.162.1
- `codex exec [PROMPT]`: prompt read from stdin when omitted or `-`.
- Used: `--skip-git-repo-check`, `--ephemeral` (no session files), `--ignore-user-config` (no user MCP/config; auth still from CODEX_HOME), `--sandbox read-only`, `--color never`, `-m <model>`, `-o/--output-last-message <file>`.
- No system-prompt flag → system prompt prepended to stdin.
- No flag disables the shell tool entirely; read-only sandbox is the limit (residual risk documented, spec §9).
- Status: `codex login status` → exit 0 + "Logged in using ChatGPT" when signed in.
- Models: no reliable list for ChatGPT-plan auth → "CLI default" + custom ID.

## Gemini CLI — 0.63.0
- `-p/--prompt <text>`: headless; appended to stdin input.
- `-m <model>`, `-o/--output-format json` (`{"response": ...}`).
- `--approval-mode plan` = read-only mode. **Never** `-y/--yolo` or `--approval-mode yolo`.
- Tool exclusion: settings v2 key `tools.exclude` (array). Project settings (`<cwd>/.gemini/settings.json`) need a trusted workspace → pass `--skip-trust`.
- Auth: OAuth creds under `~/.gemini/`; none present on owner's machine yet (sign-in in Task 10).

## Curated model IDs (resources/models.json) — checked 2026-10-11
- Claude CLI aliases: sonnet/haiku/opus (verified working in AHK tool). Anthropic API: claude-*-5-5 (current family).
- Gemini CLI: IDs taken from the CLI 0.63.0 bundle (gemini-3-flash-preview, gemini-2.5-pro; default = CLI's own).
- OpenAI: gpt-5.4-mini / gpt-5.4 / gpt-5-mini per developers.openai.com/api/docs/models/all. Not live-tested (no key).
- Gemini API: gemini-3.6-flash per third-party sources (Google docs not checked directly). Not live-tested (no key).
- Every provider also accepts a custom model ID.
