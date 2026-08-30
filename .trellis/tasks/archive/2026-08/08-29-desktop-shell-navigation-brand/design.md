# Design - Windows Desktop and TUI Shell

## Ownership and ordering

The shell owns navigation, frame/layout tokens, shared presentation state,
versioned locale persistence, brand placement, coordinator interfaces, and the
desktop spec update. The CLI contract owns message catalogues and locale
resolution. Mode children own pages, typed state machines, IPC calls, and
actions. The required order is CLI contract -> icon -> desktop spec update ->
settings/coordinator -> shell product code -> mode presentations.

## Visual topology

Use native Windows chrome. Inside it: compact brand area, centered segmented
five-mode navigation, right-side settings/help, a flexible workbench slot, and a
reserved action/status boundary. At narrow widths, navigation becomes an
accessible labelled menu or horizontal segment strip; names and critical actions
are never icon-only. Tokens define dark neutral surfaces, one DevSweep accent,
semantic feedback, compact spacing/radii, focus rings, and reduced motion.

The shell consumes CLI-owned plural, binary-unit, accelerator, and truncation
metadata. It binds locale-specific accelerators only when unique in the visible
scope. Authority, refusal, warning, and action text never truncates; user paths,
application names, and other data may be visually ellipsized only when the full
value remains accessible and copyable.

Independent check found that the archived Chinese Clean accelerator `Q` collides
with the permanent TUI Quit binding. Trellis has no supported unarchive command,
so AskUserQuestion transferred the minimal correction to this active task while
leaving the archived CLI task immutable. The user selected `accelerator: null`
for `command.clean`; the shell must show no `[Q]` mnemonic and `q` remains Quit.
The later 2026-08-30 AskUserQuestion decision supersedes the earlier
catalogue-edit prohibition only for the additive versioned shell namespace
below. It does not transfer ownership of existing command/non-shell content.

### Canonical shell catalogue V1

`resources/i18n/{en,zh-CN}.json` are the only human-copy source for both TUI and
Desktop shell/settings/persistence/route-failure presentation. The exact
additive namespace is:

| Key | English `other` form | Simplified Chinese `other` form |
| --- | --- | --- |
| `shell.v1.workbench` | `Cleanup plan workbench` | `清理计划工作区` |
| `shell.v1.supporting` | `Supporting destinations` | `支持目的地` |
| `shell.v1.supporting.protection` | `Protection` | `保护` |
| `shell.v1.supporting.rules` | `Rules` | `规则` |
| `shell.v1.supporting.history` | `History` | `历史` |
| `shell.v1.help` | `Help` | `帮助` |
| `shell.v1.settings.action` | `Language` | `语言` |
| `shell.v1.settings.title` | `Language settings` | `语言设置` |
| `shell.v1.settings.instruction` | `Choose a language, then press Enter to save.` | `选择语言，然后按 Enter 保存。` |
| `shell.v1.settings.option.en` | `English` | `英语` |
| `shell.v1.settings.option.zh_cn` | `Simplified Chinese` | `简体中文` |
| `shell.v1.settings.save` | `Save` | `保存` |
| `shell.v1.settings.saving` | `Saving` | `正在保存` |
| `shell.v1.settings.cancel` | `Cancel` | `取消` |
| `shell.v1.persistence.saving` | `Saving language preference…` | `正在保存语言偏好…` |
| `shell.v1.persistence.unavailable` | `Language preference was not changed. Close Settings, check the presentation settings file, and try again.` | `语言偏好未更改。请关闭设置、检查显示设置文件后重试。` |
| `shell.v1.store.loading` | `Loading presentation settings…` | `正在加载显示设置…` |
| `shell.v1.store.unavailable.title` | `Presentation settings unavailable` | `显示设置不可用` |
| `shell.v1.store.unavailable.detail` | `DevSweep could not safely read the presentation settings file. Existing bytes were preserved.` | `DevSweep 无法安全读取显示设置文件；现有字节已保留。` |
| `shell.v1.store.unavailable.recovery` | `Close DevSweep, check the file, and try again.` | `请关闭 DevSweep、检查该文件后重试。` |
| `shell.v1.route.error` | `Could not change destination. The current page remains active.` | `无法切换目标页面；当前页面保持不变。` |
| `shell.v1.route.dismiss` | `Dismiss` | `关闭` |

Every entry has exactly the `other` form, no placeholders, `count: none`,
`accelerator: null`, `group: null`, and `truncation: never`. English/Chinese
keys, forms, placeholder signatures, and metadata are closed and parity-tested.
The namespace contains exactly 22 keys. Supporting registrations carry only the
closed id `protection|rules|history`; the shell exhaustively maps that id to the
corresponding `shell.v1.supporting.*` key. Callers cannot inject translated
labels, and an unavailable destination remains unregistered.
TUI/Desktop may project these entries into typed view copy but may not define a
second string table. When a corrupt/unknown store leaves Desktop without a
trusted locale, it renders the English and Chinese forms of these same canonical
keys together; it never guesses a selected locale. Existing catalogue entries
remain byte-identical except the already approved Chinese Clean accelerator.

The spec update replaces the current neutral-white/gray surface rule with the
deep gray-green token set. It permits only non-informational CSS-native
sweep/orbit motifs that disappear under reduced motion and are never heroes,
illustrations, status evidence, or interaction targets; gradients, glow, glass,
copied geometry, and page-wide ornamental palette changes remain prohibited.

The user explicitly waived the manual 100/125/150/200% display-scaling matrix
and native High Contrast/Reduced Motion observation for this child on
2026-08-30. Retain current-scale captures as diagnostic only for scaling and
exact-dimension claims; independently reliable process, locale, focus, and
persistence facts may still support their non-waived clauses. Label the waived
native evidence `WAIVED/UNVERIFIED`; do not convert browser emulation, CSS tests,
or automated resizing into native PASS. This waiver changes only this child's
completion evidence gate, not the responsive CSS-width, forced-colors,
reduced-motion implementation, or any other non-waived accessibility contract.

## State, coordination, and persistence

`OperationCoordinator` accepts typed operation kind, opaque operation id,
cancellation handle, and join future. It serializes heavy work and rejects stale
completion; it never sees plans/digests/confirmation. Mode reducers remain under
mode-owned directories. `App.tsx` composes registrations and the shell rather
than owning every domain transition.

Coordinator acquisition precedes service invocation. A caller submits a typed
operation identity/cancel handle plus a start closure; the serialized
coordinator cancels and joins the old owner, refuses closed/stale reservations,
and only then invokes the new service. A refused or failed acquisition never
starts the service; start rejection cannot become an unhandled promise; each
lease completes at most once. TUI Scan follows the same cancel/join-before-start
ordering and rejects late events/results from superseded jobs.

The real TUI event loop owns one integrated `ShellComposition`; it must not load
then discard the shell state. The TUI `App` reducer owns the visible shell route,
settings view, and selected presentation locale. Input produces typed shell
events, persistence is dispatched through a runtime effect, and rendering reads
the resulting typed state. A persisted language change is observable in the
real frame and survives a process restart; explicit CLI language remains
session-only and wins without overwriting the store. The user selected this
complete integration route through AskUserQuestion on 2026-08-30.

`PresentationSettingsV1` stores only
`language: Option<PresentationLanguageTag>`. `PresentationLanguageTag` is a
core-owned closed enum serialized exactly as `"en"` or `"zh-CN"`; it is not the
CLI-owned runtime `Locale`. The V1 JSON document is exactly
`{"schema_version":1,"language":"en"|"zh-CN"|null}` with unknown fields denied.
On Windows the single resolver uses
`%LOCALAPPDATA%\DevSweep\settings\presentation-v1.json`; a missing
`LOCALAPPDATA` makes persistence unavailable instead of selecting another root.

The store acquires OS-owned exclusive transaction authority for the full
read/validate/write transaction, writes a same-directory temporary file, flushes
it, and atomically replaces the V1 path. Missing files mean no explicit choice.
Invalid UTF-8/JSON, unknown tags, and unknown/newer schema versions retain the
original bytes, report the store unavailable, and are never overwritten or
silently mapped to English. TUI/Desktop adapters exhaustively map the two stable
tags to/from the CLI runtime `Locale`; no core -> CLI dependency is added.
TUI/Desktop session selection writes through this store. Locale routing retains
source semantics rather than passing an already-resolved locale plus a separate
provenance flag: the application passes the original `Option<Locale>` from the
bare TUI's parsed `--language` argument into TUI composition. The TUI adapter
selects explicit CLI locale, then persisted UI locale, then supported Windows
user locale, then English. A corrupt or unknown persisted document fails the
store closed instead of falling through. Non-interactive CLI resolves only its
explicit flag and Windows locale and never reads persisted UI state. The CLI
`--language` flag remains session-only for bare TUI and is never persisted
without an explicit in-session settings action.

The transaction lock represents OS ownership, never mere sidecar existence. On
Windows the owner holds a real exclusive OS file lock/handle with delete
sharing; the lock name is unlinked or delete-pending while the handle remains
the authority. Normal close and abnormal process exit release ownership by OS
handle lifecycle. If name cleanup is denied, the remaining file is inert after
the handle closes: the next transaction opens it, obtains the OS lock, and may
retry cleanup. Non-Windows builds use an OS lock without a create-new ownership
sentinel. No PID, timestamp, or heuristic stale-lock takeover is allowed.

The atomic same-directory replace is the commit point. The public result matrix
is closed:

| Stage | Public result | Persisted bytes | Lock behavior |
| --- | --- | --- | --- |
| acquire/wait fails | error | unchanged | no ownership claimed |
| existing read/validation fails | error | unchanged | OS ownership released |
| temp create/write/flush/sync or replace fails | error | unchanged; owned temp removal attempted | OS ownership released |
| atomic replace succeeds | success | exact requested V1 bytes | release is handle-lifecycle and cannot turn the committed write into an error |
| lock-name cleanup fails | transaction continues under the held OS lock | governed only by whether replace later commits | residue is not ownership and the next transaction safely acquires after handle close |
| prior process exits abnormally | no API result | last committed bytes retained | OS releases the dead owner's lock; the next transaction safely acquires |

Focused tests inject lock-name cleanup failure, assert post-commit result/bytes
consistency, exercise recovery after a prior owner drops without explicit
release, serialize concurrent writers, and assert zero normal lock/temp residue.

AskUserQuestion selected a strict Desktop failure gate on 2026-08-30. Desktop
owns explicit loading, ready, and unavailable presentation-store states. It must
not render `AppShell` from an OS/English fallback before a successful load; a
load failure renders a stable bilingual store-unavailable error surface that
does not claim a selected locale. A language change awaits a successful write
and validates the returned tag before updating the presentation adapter. A
failed or mismatched write preserves the current locale and exposes recovery
copy without mutating the store or visible language.

### Native close lifecycle

The 2026-08-30 residual-process decision permits terminating only an exact
task-owned DevSweep residual after a failed graceful close and a fresh read-only
match of its executable path, start time, binary hash, and run ownership. PID
50416 was the first explicitly approved application of that rule. It does not
authorize broad process termination or any other cleanup/system action.

The final-source native run exposed the lifecycle gap: `App` cleanup called
`coordinator.close()` only from React effect teardown and discarded the
returned promise, while Tauri accepted the native close independently. The
window could therefore be destroyed before the coordinator drain was consumed,
and the final Rust process remained headless. The repair uses one typed
`DesktopLifecycleController` over an injected `DesktopLifecycleBridge`:

1. The real bridge registers one Tauri `onCloseRequested` callback.
2. The callback does not call `preventDefault()`; it returns the one shared
   drain promise. `OperationCoordinator.close()` is invoked at most once, and
   cancellation failure is consumed only after its join promise settles.
3. The installed Tauri window API awaits that callback and then performs its
   built-in `destroy()` when the event was not prevented. React does not issue
   a competing destroy IPC call, force process exit, or hide the window.
4. React unmount reuses the same drain promise and removes the close listener.
   Late listener registration or close completion cannot create an unhandled
   rejection or a second drain.
5. Last-window destruction is the only application-exit trigger. Rust/Tauri
   event-loop teardown remains authoritative and must naturally return from
   `Builder::run` with no main/child process, listener, coordinator job, store
   lock, or temporary-file residue.

The installed `@tauri-apps/api/window.js` implementation is part of the traced
mechanism: `onCloseRequested` awaits the registered handler and, only when the
event was not prevented, invokes `window.destroy`. The generated
`desktop-schema.json` exposes that IPC as the closed permission
`core:window:allow-destroy`. The application capability therefore contains the
following exact window-authority matrix:

| Window capability | State | Reason |
| --- | --- | --- |
| `core:window:allow-destroy` | allowed exactly once in the permission list | lets the installed listener destroy the last window only after the shared cancel/join drain resolves |
| create/close/resize/move/minimize/maximize/hide and every other `core:window:*` capability | absent | shell lifecycle does not need or receive this authority |

The frontend never calls destroy before or in parallel with the drain. A
static Rust test parses the capability document, requires the exact three-entry
permission list, and thereby rejects a duplicate or any additional capability.

The lifecycle state is closed: `open -> draining -> released_to_tauri -> closed`.
Duplicate close requests share the in-flight drain; Tauri alone owns native
window destruction after the awaited callback returns.
There is no artificial cleanup timeout: the coordinator's owned join is the
authority and must settle. Native evidence applies an external bounded wait
only to classify a failure; product code never abandons a still-owned job when
a timer expires. The close listener has no visible copy because a normal close
remains an operating-system action; a destroy failure keeps the visible window
open rather than displaying a false completion state.

### Debug-only native route fault

The native route-failure path is enabled only by a closed same-source debug
harness. `desktop/src-tauri/src/commands.rs` exposes
`debug_native_fault_mode` only under `#[cfg(debug_assertions)]`. The command
maps the exact process environment value
`DEVSWEEP_TASK_NATIVE_FAULT=route_cancel_once` to the closed wire value
`route_cancel_once`; missing, non-Unicode, or any other value maps to
`disabled`. `desktop/src-tauri/src/lib.rs` places the command behind the same
`#[cfg(debug_assertions)]` entry in `generate_handler!`, so a release binary
does not register or recognize it.

`desktop/src/lifecycle.ts` owns the typed Tauri window adapter, closed decoder,
exactly-once close controller, and a one-shot shell-coordinator wrapper. Only a
Vite development build asks the debug command for the closed mode. When armed,
the wrapper rejects exactly the next shell `cancelAndJoin` request before
delegation, then permanently disarms. Existing `AppShell.navigate` behavior
keeps the prior route/hash/focus, renders canonical
`shell.v1.route.{error,dismiss}` copy, and permits dismiss/retry. It cannot
affect Clean service calls, plans, digests, execution, paths, argv, or persisted
settings. With the harness disabled, the wrapper delegates unchanged operation
semantics to the real coordinator. Release web and Rust artifacts are searched
to prove the command and opt-in string are absent.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `.trellis/spec/desktop-frontend/component-guidelines.md` | five-mode composition, deep gray-green surfaces, bounded non-hero CSS motifs, bilingual accelerator/truncation/unit rules, and retained Clean UI safety contract |
| `.trellis/spec/desktop-frontend/state-management.md` | mode-local reducers, coordinator, locale store, retained Clean authority rules |
| `.trellis/spec/desktop-frontend/index.md` | updated pre-development and completion checklist |
| `crates/devsweep-core/src/presentation_settings.rs` | core-owned closed language tag, exact Windows path, versioned JSON, cross-process lock, and atomic locale-only store |
| `crates/devsweep-core/src/lib.rs` | export presentation settings interface |
| `crates/devsweep-cli/src/application/mod.rs` | route the original parsed `Option<Locale>` into bare TUI composition and resolve a final locale only for non-interactive commands; no argv reparse, persistence, machine-output, or domain behavior change |
| `resources/i18n/en.json` | add exactly the canonical `shell.v1.*` V1 entries; preserve every existing entry byte-for-byte |
| `resources/i18n/zh-CN.json` | retain the approved `command.clean.accelerator: null` and add the exact parity-matched `shell.v1.*` V1 entries; preserve every other existing entry byte-for-byte |
| `crates/devsweep-cli/src/i18n/mod.rs` | central closed catalogue/parity/metadata validation and typed canonical shell-copy access; existing CLI/machine behavior unchanged |
| `crates/devsweep-cli/src/tui/mod.rs` | declare and compose the private `shell` module only; no mode domain logic or authority |
| `crates/devsweep-cli/src/tui/shell/` | consume canonical `shell.v1.*` copy only; navigation/frame/settings/coordinator adapter and tests |
| `crates/devsweep-cli/src/tui/app/` | integrate typed shell route/settings/locale state and prevent a second Scan from starting before the current heavy job is canceled and joined; retain all Clean safety invariants |
| `crates/devsweep-cli/src/tui/runtime/` | cancel/join-before-start Scan lifecycle, stale-result rejection, typed presentation effects, and race tests without domain authority |
| `crates/devsweep-cli/src/tui/render/` | render the real shell navigation/settings/locale state and focused `TestBackend` coverage without side effects |
| `desktop/src/app-shell/` | typed shell routes/focus plus canonical supporting-id labels and localized cancel/join failure surface and recovery tests |
| `desktop/src/state/operation-coordinator.ts` | serialized reservation/start/lease lifecycle proving cancel+join before service invocation, closed/refused behavior, and exactly-once completion |
| `desktop/src/i18n/` | closed adapter over canonical catalogues, no independent shell copy table; TS parity/metadata/collision tests |
| `desktop/src/App.tsx` | shell composition and coordinator-mediated start closures for Scan/dry-run/execute |
| `desktop/src/App.test.tsx` | call-order, refusal, cancellation-failure, unmount, language, and lifecycle tests |
| `desktop/src/lifecycle.ts` | typed Tauri close bridge/controller plus closed debug-only one-shot route-fault projection; no domain authority |
| `desktop/src/lifecycle.test.ts` | exactly-once drain, rejection consumption after join, closed fault decode, and disabled-delegation tests |
| `desktop/src/components/format.ts` | delegate actual Clean capacity display to the canonical IEC binary helper; preserve byte values |
| existing tests consuming `desktop/src/components/format.ts` | IEC boundary regressions at 1023, 1024, 1048576 and higher units; no duplicate unit algorithm |
| `desktop/src/styles.css` | original tokens, responsive shell, focus/motion/high-contrast rules |
| `desktop/src/styles.test.ts` | token/responsive/reduced-motion plus `Segoe UI Variable`-first font regression tests |
| `desktop/src-tauri/src/commands.rs` | typed presentation-setting read/write commands only |
| `desktop/src-tauri/src/lib.rs` | register setting commands/coordinator state and statically assert the exact close capability list |
| `desktop/src-tauri/src/main.rs` | lifecycle entrypoint assertions/comments only if required by the proven Tauri close mechanism; no forced process exit |
| `desktop/src-tauri/capabilities/default.json` | add only `core:window:allow-destroy` so the installed Tauri close listener can destroy the last window after drain |
| `resources/i18n/zh-CN.json` | set only `command.clean.accelerator` from conflicting `Q` to `null`; no other canonical field change |

Mode tasks do not edit the three spec files or the locale store. If a later mode
finds a spec defect, it returns to this owner before implementation continues.

The combined 2026-08-30 static-audit repair is confined to the paths above and
the task's own planning/evidence files. Verification evidence remains
`awaiting independent verification` until a separate `trellis-check` accepts
the final diff. Native scaling, High Contrast, and Reduced Motion stay
`WAIVED/UNVERIFIED` and non-completion-required for this child.

The final bounded 2026-08-30 repair additionally owns this task's `task.json`
metadata and evidence, and the already-listed store, catalogue, TUI, Desktop
i18n/App/coordinator/AppShell paths. `SupportingDestinationRegistration` carries
only the closed destination id and renderer; `AppShell` exhaustively maps that
id to one of the three new canonical keys. Real App integration tests, not only
coordinator unit tests, record cancel -> joined settlement -> new
scan/dry-run/execute bridge invocation, close/unmount rejection consumption,
and zero unhandled/surviving work. TUI persistence failure projects only
`shell.v1.persistence.unavailable`; raw backend error text is not view state.

The final lifecycle round additionally owns the two frozen
`desktop/src/lifecycle{,.test}.ts` paths and the existing App/AppShell/
coordinator/Tauri command-registration files listed above. No other product or
test path is authorized. The release build has no registered debug fault
command and no opt-in string; the debug harness requires both a debug build and
the exact process-local environment value.

## Rollback

Feature-gated registrations permit atomic rollback to the previous Clean-only
shell. Rollback removes shell/coordinator/adapters together, preserves the icon
outputs and any readable V1 locale file, and never exposes a partial mode.
