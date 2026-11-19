# macOS release-candidate test checklist

Run on a real Mac before calling a macOS build "working". Note the macOS version and chip
(Apple Silicon / Intel) at the top of your results. Tick each box or write what happened.

## 0. Get the build
- [ ] GitHub → Actions → **macOS release candidate** → latest green run → download the
      `Retone-macOS-universal-…` artifact (a .zip containing `Retone_x.y.z_universal.dmg`).
- [ ] Unzip, open the `.dmg`, drag **Retone** to **Applications**.

## 1. First launch (unsigned build)
- [ ] Double-click Retone in Applications → macOS says it can't verify the developer. Click **Done** / **Cancel**.
- [ ] **Right-click (or Ctrl-click) Retone → Open → Open.** It should start.
  - macOS 15+: if there's no Open button, go to **System Settings → Privacy & Security**, scroll down, click **Open Anyway** next to Retone.
  - If it says the app "is damaged": run `xattr -dr com.apple.quarantine /Applications/Retone.app` in Terminal, then open it again. (Write down that this happened.)
- [ ] No Dock icon appears (Retone is a menu-bar app).
- [ ] A small outlined-pill icon appears in the **menu bar**. It's black in light mode and white in dark mode. Switch appearance to check both.
- [ ] The Settings window opens on first run.

## 2. Accessibility permission
- [ ] Settings shows "Accessibility: not allowed" with an **Open System Settings** button.
- [ ] Click it → macOS shows the "Retone would like to control this computer" prompt and/or opens **Privacy & Security → Accessibility**. Retone is listed there.
- [ ] Turn Retone **on**. Back in Retone Settings, click **Re-check** → shows "allowed".
      (If it still says not allowed, quit Retone from the menu bar and reopen it.)
- [ ] With permission turned **off**, press ⌥⌘P with text selected → the pill shows
      "Allow Retone under Accessibility…" (not "nothing selected"). Turn it back on.

## 3. Provider
- [ ] Pick a provider you have (Claude Code / Codex / Gemini CLI signed in, or an API key) → **Test** passes.
- [ ] Finish first-run setup. The floating pill appears at the top of the screen.

## 4. Quick polish: ⌥⌘P (Option-Command-P)
Try each app and write ✅/❌ next to it:
- [ ] TextEdit  - [ ] Notes  - [ ] Safari or Chrome text box (e.g. Gmail compose)  - [ ] Slack or Teams  - [ ] Mail or Outlook
For each: select a clumsy sentence → press ⌥⌘P →
- [ ] the pill shows it's working, then the selection is **replaced** by the polished text.
- [ ] Your clipboard is restored. Copy some unrelated text *before* the test; after it, ⌘V pastes that text back, not the rewrite.
- [ ] Also try with an **image** on the clipboard first (e.g. ⌘⇧⌃4 screenshot to clipboard): after the rewrite, the image is still on the clipboard.
- [ ] Press ⌥⌘P with **nothing selected** → a "nothing selected" style hint, no change to the document.

## 5. Polish with options: ⌥⌘O
- [ ] Select text → ⌥⌘O → options window opens with the text → pick a tone/length → Go → text replaced in the original app.
- [ ] Cancel the options window → nothing changes in the original app.

## 6. Floating pill and menu bar
- [ ] Click the pill with text selected in another app → quick polish works (it should paste back into that app, not into Retone).
- [ ] Right-click the pill → menu: Polish with options…, Settings…, Hide floating button, Quit.
- [ ] Click the menu-bar icon → same menu opens; **Show/hide floating button** toggles the pill; **Quit Retone** quits.
- [ ] Drag the pill somewhere else; quit and relaunch → it stays where you put it.

## 7. Shortcuts
- [ ] Change a shortcut in Settings (e.g. ⌃⌥P) → the new one works and the old one doesn't.
- [ ] If ⌥⌘P clashes with an app you use, note which app.

## 8. Other
- [ ] Non-English keyboard layout (if you use one): ⌥⌘P still copies/pastes correctly. (Known limit: Retone sends the physical C/V keys.)
- [ ] Launch at login (if enabled in Settings) works after a restart.
- [ ] Log file location if anything fails: `~/Library/Logs/app.retone.desktop/` — attach it to the report.

## Notes
- Each new unsigned build has a different code signature, so macOS may ask for Accessibility again
  after updating. Remove the old Retone entry (–) in the Accessibility list and add the new one.
