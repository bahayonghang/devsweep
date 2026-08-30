# Final-source Windows native evidence

This report supersedes earlier native PASS claims for the final-source binaries.
It is implementer evidence only and does not claim independent verification.

## Fixed artifacts

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `target/debug/devsweep.exe` | 6,935,040 | `85BD199F6E9668749B360A694555F854FCE0F16F25BE22E9C0AE4FF359D6405E` |
| `target/release/devsweep-desktop.exe` | 9,791,488 | `64C69BB3568ACE9D343196C2F47BB1053E42C1C41FF1A3DF8957C59B8BF08D44` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,117,667 | `47CA06EA5F2E6730526A778F95AEEDD187639DBEF93DB2835C7B8D1717266914` |

## TUI

The final debug CLI was exercised through real Windows ConPTY/process runs using
task-owned `LOCALAPPDATA` roots. The process transcripts under `tui/` and
`tui-failure/` prove:

- PASS: real `[p] Language` action, English to Chinese save, and exact 39-byte
  `{"schema_version":1,"language":"zh-CN"}` document.
- PASS: restart without `--language` rendered the persisted Chinese shell.
- PASS: `--language en` overrode the persisted choice for that session without
  changing the document bytes.
- PASS: `q` entered the Quit guard; `c` canceled/joined and each process exited 0.
- PASS: the safely injected corrupt-document save path rendered only canonical
  English persistence-unavailable copy; raw backend diagnostics did not enter UI.
- PASS: all TUI runs left zero owned process/listener/lock/temp residue.

## Desktop matrix

| Final-source native clause | State | Direct evidence/boundary |
| --- | --- | --- |
| Native Tauri launch and English Clean shell | PASS | `desktop/screenshots-round3/00-initial-en.png`; final release hash above |
| Original header icon and one product brand | PASS for visible header presence | `00-initial-en.png`; exact native size/crispness is not inferred |
| Keyboard activation of Settings | PASS | `desktop/screenshots-round3/05-after-settings-enter.png` |
| Chinese locale selection and persisted restart | UNVERIFIED | Round 3 never produced the presentation document; earlier captures are superseded |
| Back restoration to the Settings opener | UNVERIFIED | Settings opened, but the subsequent key sequence did not yield a trustworthy Back/focus capture |
| Route cancel/join failure and accessible recovery/dismiss | UNVERIFIED, completion-required | The production bridge provides no safe input that forces `cancelAndJoin` rejection; tests cannot substitute for requested native proof |
| 390/800/1024/1440 responsive contracts | PASS automated only | CSS/type/tests/web gate; native exact dimensions remain waived/unverified |
| 100/125/150/200% display scaling | WAIVED/UNVERIFIED | User waiver; no system setting was changed |
| Native High Contrast | WAIVED/UNVERIFIED | User waiver; CSS contract remains automated evidence only |
| Native Reduced Motion | WAIVED/UNVERIFIED | User waiver; CSS contract remains automated evidence only |
| Graceful Desktop process exit | FAIL | `CloseMainWindow` and `WM_CLOSE` closed all top-level windows but PID 35600 remained after bounded waits |
| Zero Desktop process residue | FAIL | PID 35600 remained headless; listeners and task-owned lock/temp/store residue were zero |

## Three-attempt boundary and blocker

The first final-source Desktop automation used DPI-virtualized captures but missed
the language control. The second attempt established that UI Automation exposes
only the WebView pane and that ordinary screen-copy captures can capture the wrong
foreground application; `PrintWindow` was adopted as the changed diagnostic. The
third attempt used target-HWND foreground attachment plus `PrintWindow`, reached the
real Settings page, but did not complete a trustworthy locale save/Back trace and
left the release process headless after graceful close requests. The round therefore
stopped without a fourth app launch.

The missing route-error native path and the failed zero-process-residue gate are
completion-required under the final bounded evidence decision. This task is **not
ready for independent acceptance or archive**. A new user-approved product/diagnostic
decision is required; automated App/AppShell tests and browser/CSS evidence must not
be promoted to native PASS.

Final evidence-only validation after this report was written:

- `rtk git diff --check`: exit 0.
- `python ./.trellis/scripts/task.py validate
  .trellis/tasks/08-29-desktop-shell-navigation-brand`: exit 0; 7 implement and
  3 check context entries valid.
- Read-only residue audit: PID 35600 remained; zero listeners and zero task-owned
  presentation-store/lock/temp files.
