# Evidence-only native round

> Superseded focus result: the later product repair and one bounded direct-CDP
> run prove Language-opener focus, dismiss, and one-shot recovery. That later
> command exits 1 only because Vite's `/favicon.ico` 404 appears in the strict
> CDP error-log audit. See
> `../focus-failure-repair-20260830/verification.md`. The release hashes in this
> historical report are also superseded for final-source claims.

Date: 2026-08-30

Decision: **release evidence is implementer-PASS for locale persistence,
restart, Back/focus, and natural close. A final direct-CDP run then proved the
canonical debug route error and unchanged route, but directly exposed a native
focus failure: focus moved to `BODY` rather than remaining on the Language
opener. Dismiss/retry stopped at that first evidence failure. Overall state
remains awaiting independent verification and is not ready for archive.**

This round changed no product, test, specification, PRD, design, implementation
plan, or task metadata. It used task-owned evidence directories only.

## Fixed artifacts and isolation

| Artifact | Bytes | SHA-256 | Result |
| --- | ---: | --- | --- |
| `target/release/devsweep-desktop.exe` | 9,826,816 | `C75D08506B97984635EB3799E8BD3D9D5732A5D40A328BBA4B3FEA2D6FD66742` | Matched before and throughout release evidence |
| `target/debug/devsweep-desktop.exe` before `tauri dev` | 14,758,912 | `E16F09D393A81291D7D05F1D655648B16592A456786EAA431E0478CF5B727091` | Matched the dispatch prerequisite |
| `target/debug/devsweep-desktop.exe` after `tauri dev` relink | 14,758,912 | `5677E0FE2DF24B6A454A2CEA378B38985EF6B0760DA12BC7D19E1384929C07D9` | User subsequently fixed this as the sole authorized debug-evidence hash; it matched before launch, after launch, and after close |

Release presentation state was isolated under
`release/localappdata/DevSweep/settings/presentation-v1.json`. The initial
directory was empty. The final document is exactly 39 bytes:

```json
{"schema_version":1,"language":"zh-CN"}
```

Its SHA-256 is
`2CD9EEDF29B04115F701CE73EA055ACCB6AFB96896ADE03BD22189D6B43D2899`.
Every store audit found zero sibling lock/temp files. The attempted debug run
used a separate `debug/localappdata` and left it empty.

## Release evidence matrix

| Clause | State | Direct evidence |
| --- | --- | --- |
| Fixed release identity, real PID/HWND, loopback-only CDP ownership | PASS | `logs/baseline.json`, `logs/release-launch.json`; PID 45252/HWND `0xC615FA`; CDP 9335 owned by its direct WebView2 child on `127.0.0.1` |
| Real native WebView English to Simplified Chinese selection | PASS | Named session `devsweep-shell-release-1d2fcac28664`; `snapshots/release-settings-en.txt`, `snapshots/release-settings-zh-CN.txt`, `logs/release-settings-zh-CN-dom.json` |
| Canonical Chinese shell and `data-locale=zh-CN` | PASS | Same post-selection snapshot/DOM record; HWND-bound PrintWindow capture `screenshots/release-settings-zh-CN-native-printwindow.png` |
| Exact V1 persistence, no lock/temp residue | PASS | `logs/release-store-after-zh.json`; exact bytes/length/hash above |
| First natural last-window close | PASS | `logs/release-natural-close.json`: `CloseMainWindow=true`, 271 ms, main plus six recorded WebView descendants gone, CDP 9335 and lock/temp residue zero |
| No-flag restart consumes persisted Chinese | PASS | PID 34712/HWND `0x4830F80`, CDP 9337; `logs/release-restart-launch.json`, `snapshots/release-restart-initial-zh-CN.txt`, `logs/release-restart-initial-zh-CN-dom.json` |
| Real History Back and opener focus restoration | PASS | Language opened `#/settings`; `agent-browser back` restored `#/clean`; `document.activeElement` was the Chinese Language opener. See `snapshots/release-restart-after-browser-back.txt`, `logs/release-restart-after-browser-back-dom.json`, and `screenshots/release-restart-zh-CN-after-back-native.png` |
| Store remains byte-stable across restart/navigation | PASS | `logs/release-restart-store-before-navigation.json`, `logs/release-restart-store-after-back.json`, `logs/release-restart-close-followup.json` |
| Restart natural close and zero owned residue | PASS with preserved diagnostic | `CloseMainWindow` removed PID 34712; the recording command then exited 1 only because it read `Process.Path` after exit. `logs/release-restart-close-followup.json` independently records no main/WebView/CDP/session/lock/temp residue and unchanged store |
| Final-hash taskbar icon appearance | **UNVERIFIED** | A bottom-screen button labeled `devsweep` could not be uniquely bound to the Tauri HWND because another host/repository window used the same title. `logs/release-associated-icon-audit.json` proves the executable's associated icon is the expected green icon, but that is not a direct taskbar observation |

The first literal `Alt+ArrowLeft` synthesis returned exit 0 but did not change
the native WebView history; it is not counted as evidence. The documented
agent-browser `back` command then exercised the real CDP History Back path and
produced the focus result above.

The early screen-coordinate captures `release-initial-native.png`,
`release-settings-zh-CN-native.png`, `release-taskbar-candidate.png`, and
`release-taskbar-bound-crop.png` are retained only as superseded diagnostics:
another foreground/host window obscured the Tauri window or shared the
`devsweep` title, so they support no PASS claim. The direct native-window
evidence is the HWND-bound `PrintWindow` capture named in the table.

## Debug-only harness boundary

The first debug preparation used `npm.cmd run tauri -- dev --no-watch` with
exact env `DEVSWEEP_TASK_NATIVE_FAULT=route_cancel_once`, isolated debug
`LOCALAPPDATA`, and loopback CDP 9336. Tauri relinked the debug file from
`E16F09D...727091` to `5677E0FE...9C07D9`, so that preparation stopped before
interaction and closed naturally. The user then explicitly fixed
`5677E0FE...9C07D9` and authorized one isolated debug-only run.

For that sole run, Vite was started separately on loopback 4180 so the fixed
debug file would not be relinked. The fixed hash matched before launch, after
launch, and after close. PID 43992/HWND `0x114081A` owned the WebView whose CDP
9336 target list contained exactly the native page
`http://127.0.0.1:4180/#/clean`.

The first `agent-browser connect 9336` named session instead created its own
`about:blank` Chrome target; its snapshot returned no app DOM. The diagnosis
changed once to a fresh named session with explicit global `--cdp 9336`; that
command produced the correct native Clean/Language snapshot. Because the
session was configured with a 10-second idle timeout, the later snapshot
restarted an `about:blank` controlled browser and again returned only
`[agent-browser] launched browser`. This was the same target-binding/snapshot
failure for a second time. Per the stop rule, no Language click was sent, the
one-shot fault was never exercised, and no third connection or application run
was attempted.

The debug window was closed naturally (`CloseMainWindow=true`, 272 ms); all
recorded conhost/WebView descendants and CDP 9336 exited. Vite was ended with
Ctrl+C and port 4180 returned to zero. Debug `LOCALAPPDATA` remained empty.
Evidence:

- `logs/debug-tauri-dev-transcript.txt`
- `logs/debug-hash-drift-and-natural-close.json`
- `logs/debug-fixed-vite-transcript.txt`
- `logs/debug-fixed-launch-followup.json`
- `logs/debug-fixed-cdp-diagnostic.json`
- `logs/debug-fixed-automation-stop-natural-close.json`
- `logs/debug-fixed-agent-browser-close.json`
- `logs/debug-fixed-agent-browser-stream-disable-close.json`
- `logs/debug-fixed-agent-browser-final-close.json`
- `logs/final-residue-and-scope-audit.json`

Therefore canonical debug route-error, route/hash/focus preservation,
dismiss, one-shot recovery, console audit, and native debug screenshot remain
**UNVERIFIED, completion-required**. This round does not infer them from unit
tests and does not represent the debug fault as a release failure.

Both earlier debug named sessions returned `Browser closed`; stream disable
eventually returned success for both. The user later authorized necessary Goal
operations without another question. The remaining agent-browser daemons were
identity-gated and terminated before the final run; one later orphan PID 3264
was likewise verified by exact executable path, absent parent, and listener
state before single-PID termination. The final run did not use agent-browser,
and final agent-browser process/listener residue is zero.

## Final direct-CDP finding

The final authorized diagnostic used the fixed 5677 hash, a separate Vite
listener, `/json/list`, and .NET `ClientWebSocket`; it did not launch Chromium
or `about:blank`. It selected unique target
`3BA717010E836E9BE432412D9081ADE5` owned by the real native WebView on loopback
9341.

The first Language action produced the canonical English alert and kept
`#/clean`; Settings was absent and the installed unhandled/error arrays were
empty. However, after the caught route rejection `document.activeElement` was
`BODY`, not the Language opener. This violates the completion-required focus
clause. The single-run script stopped immediately, so dismiss/retry and native
alert screenshot remain `UNVERIFIED`. Full evidence is under
`final-direct-cdp/verification.md`, `verification.json`, and
`cdp-messages.jsonl`.

## Agent-browser discipline and command outcomes

- Read `agent-browser skills get core` and `agent-browser skills get electron`
  before interaction.
- Used only named sessions:
  `devsweep-shell-release-1d2fcac28664`,
  `devsweep-shell-restart-1d2fcac28664`, and reserved-but-unused
  `devsweep-shell-debug-1d2fcac28664`.
- The initial attach form that combined CDP startup with `tab`/`snapshot`
  returned no usable output within 30 seconds. Changing once to the established
  named session's direct `snapshot` succeeded. The same known attach behavior
  occurred on restart. No evidence action exceeded the three-attempt limit.
- The first Language click shell invocation left `@e5` unquoted and exited 1
  before browser interaction; the quoted ref retry exited 0.
- Release select, snapshots, DOM evaluations, store audits, PrintWindow,
  browser Back, and first close/residue gates exited 0.
- One restart-close evidence command had a parser-only failure before any
  action. Its corrected command closed the app, then exited 1 while reading the
  already-exited process path. The exit-0 follow-up records zero residue.
- Both release named sessions returned `Browser closed`; after explicit stream
  disable and short idle timeout their daemon/listener residue reached zero.
  CDP ports 9335/9337 returned to zero.
- The final read-only audit found no DevSweep, DevSweep WebView, task Vite,
  application CDP, agent-browser, store lock, or temp residue. No final direct-
  CDP DevSweep process required forced termination.

## Remaining boundaries

- Debug route alert and unchanged-route clauses are direct PASS, but native
  focus restoration is **FAIL** because focus moved to `BODY`.
- Dismiss/retry, one-shot recovery, full console audit through recovery, and
  native alert screenshot remain completion-required `UNVERIFIED` because the
  run stopped at the first focus failure.
- Taskbar appearance remains `UNVERIFIED` because no taskbar button could be
  uniquely tied to the final release PID/HWND.
- Native scaling, High Contrast, and Reduced Motion remain the user's existing
  non-completion-required `WAIVED/UNVERIFIED` items. This round did not inspect
  or change those settings.
- Overall status remains **implementer evidence only, awaiting independent
  verification**.
