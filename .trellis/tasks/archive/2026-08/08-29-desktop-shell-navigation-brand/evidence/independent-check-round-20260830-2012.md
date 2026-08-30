# Independent Phase 2.2 check — artifact rebind required

Date: 2026-08-30

Result: **REJECT — needs final native rebind only**.

The product/spec diff and all independently executed automated gates passed.
`just desktop-build` successfully rebuilt the release executable and NSIS
bundle, but that successful rebuild changed their bytes after the implementer
had captured native evidence. The current artifacts are therefore not the
artifacts bound by the existing PID/HWND/taskbar evidence. No product defect
was found in this round; the remaining completion requirement is to recapture
the final native evidence against the frozen current artifacts without another
build.

## Independent command results

| Command | Exit/result |
| --- | --- |
| `rtk npm test -- src/app-shell/AppShell.test.tsx` | 0; 15/15 |
| `rtk cargo test -p devsweep-core presentation_settings` | 0; 8 pass |
| `rtk cargo test -p devsweep-cli i18n` | 0; 10 pass |
| `rtk cargo test -p devsweep-cli tui::shell` | 0; 4 pass |
| `rtk cargo test -p devsweep-cli tui::app` | 0; 31 pass |
| `rtk cargo test -p devsweep-cli tui::runtime` | 0; 11 pass |
| `rtk cargo test -p devsweep-cli tui::render` | 0; 24 pass |
| `rtk cargo test -p devsweep-desktop debug_native_fault` | 0; 1 pass |
| focused Desktop i18n/coordinator/lifecycle/AppShell/App/styles suite | 0; 67/67 |
| `rtk npm run lint` | 0 |
| `rtk npm run typecheck` | 0 |
| `rtk npm run types:generate` | 0; deterministic generated file unchanged |
| `rtk just desktop-web-check` | 0; 100/100 plus web build |
| `rtk just desktop-test` | 0; 21 pass, 2 ignored fixtures |
| `rtk just desktop-build` | 0; release and NSIS produced |
| `git diff --check d79041fe...` | 0 |
| task `validate` | 0 |
| manifest/lock diff | 0; no changes |
| catalogue parity audit | 0; exact 22 shell keys, existing English unchanged, sole existing Chinese change is Clean `Q -> null` |
| release debug-seam searches | expected no-match exits 1/1 |
| capability audit | 0; exact listen, unlisten, destroy entries |
| task-owned runtime `.trellis` diff search | expected no-match exit 1 |
| `rtk just ci` | 0 |

Complete `just ci` output:
`C:\Users\lyh\AppData\Local\rtk\tee\1788091895_just_ci.log`
(27,955 bytes; SHA-256
`B5D1B1C92FAF9F6F1953C931C1EF41C16AEE4D6E9503D3A49A0D2390D94AE27C`).

## Frozen current artifacts after all gates

| Artifact | Bytes | Modified | SHA-256 |
| --- | ---: | --- | --- |
| `target/release/devsweep-desktop.exe` | 9,827,328 | `2026-08-30T20:05:38.6086619+08:00` | `92C3919D776AC0AE9F04633C26E52BB23BD33D9706F0DA669F55C6F1D62B4E20` |
| `target/debug/devsweep-desktop.exe` | 14,758,912 | `2026-08-30T19:42:32.2022651+08:00` | `F5DFC77BE96E780EC15817B94931C74BF6A9412DF46C128B1902925DF63D65F5` |
| `target/release/bundle/nsis/devsweep_0.2.0_x64-setup.exe` | 3,152,372 | `2026-08-30T20:05:37.6693916+08:00` | `0903D735C1C6C5AC7614410C62F84E82E92E2DC1CE454444C5B9712F8BA39328` |

Final read-only residue audit: zero DevSweep processes, zero DevSweep-owned
WebView processes, zero listeners on the task ports, and zero task-evidence
lock/temp files. Generated `target/` and `desktop/dist/` outputs remain ignored
and are not staged.

Scaling, native High Contrast, and native Reduced Motion remain the
user-approved, non-completion-required **WAIVED/UNVERIFIED** observations.
