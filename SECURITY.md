# Security Policy

## Reporting a vulnerability

Please report security issues privately using GitHub's private vulnerability reporting: go to the repository's **Security** tab → **Report a vulnerability** (Security Advisories). Do not open a public issue for security problems.

Include what you found, steps to reproduce, the affected version and OS, and the impact as you understand it. You can expect an acknowledgement, and we will keep you updated while a fix is prepared.

## Supported versions

Only the latest release receives security fixes.

## Scope

Retone is a desktop app that handles sensitive material, so these areas are especially in scope:

- **Provider CLIs.** Retone runs the user's installed Claude Code, Codex and Gemini CLIs as child processes and passes them the selected text. Issues such as argument or prompt injection that lets selected text trigger tool use, file access or command execution are in scope.
- **API keys.** Keys for API providers are stored in the OS credential store (Windows Credential Manager / macOS Keychain). Any leak of keys into files, logs, URLs or other processes is in scope.
- **Clipboard and text handling.** Retone temporarily uses the clipboard and simulates copy/paste keystrokes. Leaks or unintended exposure of the user's text are in scope.
- **Network traffic.** Text should go only to the provider the user selected.

Out of scope: vulnerabilities in the third-party CLIs or provider services themselves (report those upstream), and issues that require an attacker to already have control of the user's account or machine.
