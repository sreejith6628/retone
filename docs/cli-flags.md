# CLI flags (verified 2026-10-11, Windows)

## Claude Code — 2.1.294
`claude -p --model <m> --tools "" --setting-sources "" --strict-mcp-config --no-session-persistence --system-prompt-file <f>`; draft on stdin.
Status: `claude auth status` → JSON with `loggedIn`.

## Codex CLI — codex-cli 0.162.1
- `codex exec [PROMPT]`: prompt read from stdin when omitted or `-`.
- Used: `--skip-git-repo-check`, `--ephemeral` (no session files), `--ignore-user-config` (no user MCP/config; auth still from CODEX_HOME), `--sandbox read-only`, `--color never`, `-m <model>`, `-o/--output-last-message <file>`.
- No system-prompt flag → system prompt prepended to stdin.
- Tools switched off with `-c features.<name>=false` for every name in `DISABLED_FEATURES` (crates/retone-core/src/providers/codex_cli.rs: apps, plugins, shell_tool, unified_exec, multi_agent, browser_use*, computer_use, in_app_browser, view_image, skill_*, and more) plus `-c web_search=disabled`. Use `-c`, not `--disable`: `--disable` errors on feature names a given Codex version doesn't know. Without these, `codex exec` offers the account's connected ChatGPT apps (e.g. Outlook/Teams send) and web search with no approval (verified on 0.162.1, 2026-11). Re-check with `codex features list` and a "list every tool you can call" prompt on each Codex upgrade.
- Status: `codex login status` → exit 0 + "Logged in using ChatGPT" when signed in.
- Models: no reliable list for ChatGPT-plan auth → "CLI default" + custom ID.

## Gemini CLI — 0.63.0
- `-p/--prompt <text>`: headless; appended to stdin input.
- `-m <model>`, `-o/--output-format json` (`{"response": ...}`).
- `--approval-mode plan` = read-only mode. **Never** `-y/--yolo` or `--approval-mode yolo`.
- All tools denied by a per-job `--policy <file>` (`toolName="*"` deny and `mcpName="*"` deny, priority 999), kept outside the working folder because Gemini lists that folder to the model. `tools.exclude` in project settings is still written but is not honoured in plan mode on 0.63.0 (read_file, list_directory, google_web_search, invoke_agent stayed available). Gemini records sessions under `~/.gemini/tmp/<slug>/` and `~/.gemini/history/<slug>/` with no opt-out; after each job Retone looks up its job folder in `~/.gemini/projects.json` (or `$GEMINI_CLI_HOME/.gemini`) and deletes those two folders, only if the slug matches `retone-[a-z0-9-]+`.
- Auth: OAuth credentials under `~/.gemini/`.

## Curated model IDs (resources/models.json) — checked 2026-10-11
- Claude CLI aliases: sonnet/haiku/opus (verified working). Anthropic API: claude-*-5-5 (current family).
- Gemini CLI: IDs taken from the CLI 0.63.0 bundle (gemini-3-flash-preview, gemini-2.5-pro; default = CLI's own).
- OpenAI: gpt-5.4-mini / gpt-5.4 / gpt-5-mini per developers.openai.com/api/docs/models/all. Not live-tested (no key).
- Gemini API: gemini-3.6-flash per third-party sources (Google docs not checked directly). Not live-tested (no key).
- Every provider also accepts a custom model ID.
