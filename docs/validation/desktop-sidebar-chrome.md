# Desktop sidebar chrome validation

Web and native evidence for the sidebar workbench, page headers, card
surfaces, and glyph tile rows after
`09-13-desktop-modes-card-restyle`. Device scale uses CSS pixels through
CDP `Emulation.setDeviceMetricsOverride`. The host OS global scale was
not changed.

This page points at **archived** evidence from
`.trellis/tasks/archive/2026-09/09-13-desktop-modes-card-restyle/evidence/`.

Web PASS rows are fixture Vite (`npm run dev:fixture`, port 4180) driven
by CDP Chrome. They are not a native WebView2 PASS. Native rows below
are UNVERIFIED.

Evidence root used below:

`.trellis/tasks/archive/2026-09/09-13-desktop-modes-card-restyle/evidence/`

## Environment

Windows 11 build 26200, AMD64, rustc 1.98.0, Node v26.7.0, Chrome
`C:\Program Files\Google\Chrome\Application\chrome.exe`. Working tree
on branch `dev` after `21c00b5`. Fixture language starts English; zh-CN is set through
`#/settings` then the Language select. Isolated native
`LOCALAPPDATA` under `evidence/native-localappdata/`. Desktop release
SHA-256
`237706b3a1ab1f8e2d79a1c5501c354434ea6109223e749f59fa11e0707f3afe`
(`target/release/devsweep-desktop.exe`). Capture log:
`evidence/capture-log.jsonl`. Driver:
`evidence/resources/capture.mjs`.

Web screenshots record shell chrome, page headers, and empty-state
cards. Fixture inventory/catalogue/snapshot clicks did not populate
list rows in this capture; tile-row markup is covered by mode tests.
The 390/800 CSS-pixel captures were recaptured after the top-strip
overlap fix (`min-width: max-content` on destination lists).

## Scenario matrix

| Scenario | Method | Status | Artifact |
| --- | --- | --- | --- |
| English / Chinese | Settings language then each primary mode and support route | PASS (web fixture) | `evidence/web/en-*-{390,800,1024,1440}.png`, `zh-*-{390,800,1024,1440}.png` |
| Keyboard | Tab from Clean tab; 16 steps | PASS (web fixture) | `evidence/web/keyboard-walk.json`; log `keyboard_walk` |
| Reduced motion | CDP `Emulation.setEmulatedMedia` `prefers-reduced-motion: reduce` | PASS (web fixture) | `evidence/web/en-*-reduced-motion.png` |
| Forced colors | CDP `Emulation.setEmulatedMedia` `forced-colors: active` | PASS (web fixture) | `evidence/web/en-*-forced-colors.png` |
| Widths 390 / 800 / 1024 / 1440 | `Emulation.setDeviceMetricsOverride` | PASS (web fixture) | per-route files below |
| Native 1080x720 and 900x600 | Release exe, isolated `LOCALAPPDATA`, WebView2 `--remote-debugging-port` | UNVERIFIED | `evidence/native/UNVERIFIED.json`; CDP json/list never showed `tauri.localhost` |
| OS display scale | unchanged | n/a | not modified |

## Per-route web captures

Paths are relative to `evidence/web/`.

| Route | EN 390 | EN 800 | EN 1024 | EN 1440 | zh-CN 390 | zh-CN 800 | zh-CN 1024 | zh-CN 1440 | Reduced motion | Forced colors |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `#/clean` | `en-clean-390.png` | `en-clean-800.png` | `en-clean-1024.png` | `en-clean-1440.png` | `zh-clean-390.png` | `zh-clean-800.png` | `zh-clean-1024.png` | `zh-clean-1440.png` | `en-clean-reduced-motion.png` | `en-clean-forced-colors.png` |
| `#/software` | `en-software-390.png` | `en-software-800.png` | `en-software-1024.png` | `en-software-1440.png` | `zh-software-390.png` | `zh-software-800.png` | `zh-software-1024.png` | `zh-software-1440.png` | `en-software-reduced-motion.png` | `en-software-forced-colors.png` |
| `#/optimize` | `en-optimize-390.png` | `en-optimize-800.png` | `en-optimize-1024.png` | `en-optimize-1440.png` | `zh-optimize-390.png` | `zh-optimize-800.png` | `zh-optimize-1024.png` | `zh-optimize-1440.png` | `en-optimize-reduced-motion.png` | `en-optimize-forced-colors.png` |
| `#/analyze` | `en-analyze-390.png` | `en-analyze-800.png` | `en-analyze-1024.png` | `en-analyze-1440.png` | `zh-analyze-390.png` | `zh-analyze-800.png` | `zh-analyze-1024.png` | `zh-analyze-1440.png` | `en-analyze-reduced-motion.png` | `en-analyze-forced-colors.png` |
| `#/status` | `en-status-390.png` | `en-status-800.png` | `en-status-1024.png` | `en-status-1440.png` | `zh-status-390.png` | `zh-status-800.png` | `zh-status-1024.png` | `zh-status-1440.png` | `en-status-reduced-motion.png` | `en-status-forced-colors.png` |
| `#/protection` | `en-protection-390.png` | `en-protection-800.png` | `en-protection-1024.png` | `en-protection-1440.png` | `zh-protection-390.png` | `zh-protection-800.png` | `zh-protection-1024.png` | `zh-protection-1440.png` | `en-protection-reduced-motion.png` | `en-protection-forced-colors.png` |
| `#/rules` | `en-rules-390.png` | `en-rules-800.png` | `en-rules-1024.png` | `en-rules-1440.png` | `zh-rules-390.png` | `zh-rules-800.png` | `zh-rules-1024.png` | `zh-rules-1440.png` | `en-rules-reduced-motion.png` | `en-rules-forced-colors.png` |
| `#/history` | `en-history-390.png` | `en-history-800.png` | `en-history-1024.png` | `en-history-1440.png` | `zh-history-390.png` | `zh-history-800.png` | `zh-history-1024.png` | `zh-history-1440.png` | `en-history-reduced-motion.png` | `en-history-forced-colors.png` |

zh-CN reduced-motion and forced-colors passes were not captured. Marked
UNVERIFIED: one EN pass was taken for each media feature; a second
locale pass was not required by the protocol and was not repeated.

## Native

| Window | Status | Reason |
| --- | --- | --- |
| 1080x720 | UNVERIFIED | WebView2 remote debugging did not expose `tauri.localhost`. `cargo build -p devsweep-desktop --release` succeeded. SHA-256 `237706b3a1ab1f8e2d79a1c5501c354434ea6109223e749f59fa11e0707f3afe`. First attach: `app-shell did not mount`. Second attach: `cdp timeout port 2208` with json/list showing only leftover `http://127.0.0.1:4180/`. |
| 900x600 | UNVERIFIED | Same attach failure. No screenshot. |

See `evidence/native/UNVERIFIED.json` and `evidence/native/binary-sha256.txt`.
