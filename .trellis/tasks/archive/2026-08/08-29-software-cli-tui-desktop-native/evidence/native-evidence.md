# Software native evidence status

Date: 2026-08-31. No uninstall was dispatched and no installed package was removed.

## Verified standard-user inventory

`cargo run -p devsweep-cli --bin devsweep -- software inventory --source all --format json --output <evidence>/native-20260831/software-inventory-all.json` exited 0. The machine envelope truthfully reports `outcome: partial`: 1,293 exact entries, 86 selectable current-user MSIX entries, 1,207 manual entries, one partial size, 585 unknown sizes, three partial ARP source partitions, and available ARP/MSI/MSIX source partitions. Every entry reports `last_used.state: unknown` and `reason_code: no_supported_exact_source`.

`target/debug/devsweep.exe software inventory --source msix --format json --output <evidence>/native-20260831/software-inventory-msix.json` exited 0 from a hidden standard-user process. Its envelope reports `outcome: succeeded`: 179 current-user MSIX entries, 86 selectable and 93 manual, with the source `msix_current_user: available`; every last-used value is unknown.

The exact raw envelopes are:

- `native-20260831/software-inventory-all.json`
- `native-20260831/software-inventory-msix.json`
- `native-20260831/integrity.txt` records the medium-integrity user token (`S-1-16-8192`).

## Verified non-destructive plan and preview

One selectable current-user MSIX entry from the captured inventory (`VP9 Video Extensions`, exact package `Microsoft.VP9VideoExtensions_1.2.20.0_x64__8wekyb3d8bbwe`) was used to create a saved plan and to exercise preview only. Plan creation and JSON, English-human, and Simplified-Chinese-human preview commands all exited 0. They agree on the exact tagged ID, current-user MSIX identity, action class, explicit irreversibility, and digest `sha256:4fdcf3cb163849a0e37a1bac6b108e4430b215707a1edae0e3c23bb54347ed46`.

- `native-20260831/software-plan-vp9.json`
- `native-20260831/software-preview-vp9.json`
- `native-20260831/software-preview-vp9-en.txt`
- `native-20260831/software-preview-vp9-zh-CN.txt`

Preview did not dispatch the action and did not remove or modify the installed package.

The process-tree probe could not query `Win32_Process`: every `Get-CimInstance Win32_Process` request returned `Access denied`. The inventory process itself exited 0. Process-tree and no-UAC observation therefore remain `UNVERIFIED`; medium integrity is verified, but it is not silently promoted into those separate claims.

## Fixture and automated presentation evidence

- `desktop/src/api/fixtures/software/inventory.json` covers the complete ordered thirteen-reason refusal matrix, duplicate and long names, exact tagged ARP/MSI/MSIX identities, reported estimate/measured/partial/unknown size evidence, and strict last-used unknown.
- `desktop/src/api/fixtures/software/inventory-partial.json` and `all-msi-manual.json` cover partial sources and all-MSI manual refusal.
- `desktop/src/api/fixtures/software/execution-five-terminal.json` covers exactly removed, reboot-required, still-present, failed, and unknown-after-dispatch; execution `partial` is rejected by Rust and TypeScript contract tests.
- `desktop/src/api/fixtures/software/audit-restart.json` and the TUI audit reducer test prove operation-id-scoped restart recovery to `unknown_after_dispatch`; `SoftwareExecutor::recover_startup` performs re-query only and never calls the removal adapter.
- The TUI TestBackend renders English and Simplified Chinese manual refusal, explicit unknown size/last-used, exact MSI identity, and irreversible restore/reinstall copy at 100 columns. Desktop reducer/component tests cover stale result rejection, exact selectable identity, preview digest, and second confirmation.

## Native desktop evidence boundary

The current sandbox cannot start esbuild, the standard Vitest transform, Vite, or a newly built native desktop binary. Both `just desktop-web-check` and `just desktop-build` stop at Node child-process creation with:

```text
Error: spawn EPERM
    at ChildProcess.spawn (node:internal/child_process:457:11)
```

The existing release executable predates this Software presentation change, so it was not used as misleading evidence. Consequently all real-current-build desktop captures remain `UNVERIFIED`: English/Chinese dense rows and confirmation at 390/800/1024/1440 CSS pixels; 100/125/150/200% WebView2 scale launches; reduced motion/high contrast; keyboard focus and full accessibility tree; long/duplicate names; partial sources; cancel/timeout/unknown visual states; and restart-audit visual state.

A live interactive terminal capture of the TUI keyboard path was also not available; that path remains `UNVERIFIED` beyond the 100-column TestBackend render and reducer/runtime tests described above.

For source-level evidence only, `vite-sandbox-shim.cjs` and `vitest.sandbox.config.mjs` replace esbuild with TypeScript's in-process transpiler and suppress Vite's optional Windows network-drive probe. The Software-only suite then passed all 8 tests in 3 files. This does not replace the authoritative project configuration or native WebView2 evidence; it only verifies the new reducer, parity, and component tests without asking the restricted host to create a child process.

## Explicitly unverified destructive/field evidence

No separately confirmed disposable current-user MSIX target was supplied. Per the execution checkpoint, uninstall, post-state, reboot, timeout/cancel/unknown-after-dispatch, crash/restart audit replay, and proof of no redispatch remain `UNVERIFIED`. No arbitrary installed package was executed as a substitute. Free-space recovery and activity/last-used claims were not made.


## Main-session native capture (2026-08-31, post-repair current build)

Driver `capture-native-software.mjs` (analyze-task CDP pattern: real release
binary sha256 recorded in-log, loopback WebView2 debugging ports 9361-9364,
isolated LOCALAPPDATA per profile, graceful Browser.close + taskkill fallback,
audit trail in `native-20260831/capture-log.jsonl`):

- `sw-01-inventory-en-1440.png` + `sw-02-inventory-en-{1024,800,390}.png` —
  real standard-user inventory, 1,293 rows, summary bar
  "0 selected | reported estimate 0 B | measured package 0 B | partial at
  least 0 B | 0 unknown".
- `sw-05-inventory-zh-1440.png`, `sw-06-inventory-zh-390.png` — zh-CN
  ("软件清单：1293 个条目；86 个可选择；1207 个需手动处理。"), persisted-store
  audits in-log (`{"schema_version":1,"language":"zh-CN"}` for the zh phase).
- `sw-dpi-{100,125,150,200}-inventory-en-1440.png` — four separate real app
  launches; probes recorded devicePixelRatio 1/1.25/1.5/2 with the window
  viewport shrinking 1000×750 → 500×375 (WebView2 scale pipeline; the user's
  OS display setting was not modified).
- `axtree/software-inventory-en-full-axtree.json.gz` (gzip of the 44,045-node CDP
  accessibility tree; 28.7 MB to 817 KB) of the real inventory screen. Kept
  local-only by `.trellis/.gitignore` per the large-evidence policy; sha256
  `a9c38c1fc5e1aa97...` records its content for audit.
- `software-inventory-all.json`, `software-inventory-msix.json`,
  `software-plan-vp9.json`, `software-preview-vp9.json`,
  `software-preview-vp9-{en,zh-CN}.txt` — real CLI/preview artifacts with the
  VP9 Video Extensions digest.
- Keyboard focus probe (`keyboard_select`): the first row's checkbox is a
  disabled manual entry, so Space did not toggle it — consistent with the
  all-manual-rows-disabled rule; adjudicate against component tests.

Data-not-defect note for adjudication: several real registry entries on this
host carry the literal string `${{arpDisplayName}}` (unexpanded template with
double braces, as stored) as their DisplayName (a third-party installer's
unexpanded template). DevSweep surfaces the raw registry value as data rather
than fabricating a name; visible in `sw-05-inventory-zh-1440.png` and in the
saved inventory JSON (5 entries, all ARP `hidden_entry` manual). Independent
check adjudication: faithful behavior — display strings are data, the raw value
is preserved, and the entries remain visible-but-unselectable.

Remaining task-doc-sanctioned UNVERIFIED: real uninstall/post-state/reboot/
cancel-timeout/crash-restart scenarios (await a user-confirmed disposable
current-user MSIX target; no fixture may be installed or signed), live
interactive TUI keyboard capture, process-tree/no-UAC probe (sandbox access
denied), free-space recovery and activity/last-used claims (prohibited).

Additional boundary recorded by the independent check: the capture set shows
the inventory screen only; the second-confirmation dialog, reduced-motion, and
high-contrast appearances were not captured natively. Their Software behavior
is covered by the desktop component tests (second confirmation, exact
identities, digest) and the TUI TestBackend confirmation render, not by native
screenshots. No native claim above covers them.
