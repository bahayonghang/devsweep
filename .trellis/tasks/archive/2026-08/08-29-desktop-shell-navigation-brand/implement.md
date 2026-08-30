# Implement - Windows Desktop and TUI Shell

Start only after a later explicit approval, the CLI contract and icon child have
passed, and this child is started. Product UI changes must not begin from the
current planning state.

## 1. Update the governing desktop specification first

1. Rewrite the three owned desktop spec files to define five-mode composition,
   mode-local state, coordinator lifecycle, bilingual presentation settings,
   locale-specific accelerator/truncation/unit behavior, the deep gray-green
   surface palette, bounded CSS-native non-hero motifs, reduced-motion fallback,
   and the retained Clean safety invariants.
2. Run a spec diff review before touching `desktop/src/`; mode tasks consume the
   updated spec and do not edit it independently.

Validation:

```powershell
rtk git diff --check -- .trellis/spec/desktop-frontend
```

Rollback point: restore all three spec files together and stop; never implement
against a specification that still describes only the monolithic cleanup app.

## 2. Add the single presentation-settings owner

1. Add the core-owned closed `PresentationLanguageTag` and exact V1 JSON at
   `%LOCALAPPDATA%\DevSweep\settings\presentation-v1.json`, with a
   cross-process transaction lock, same-directory atomic create/replace,
   unknown-version/tag refusal, byte preservation, and no machine-output
   influence.
2. Add Tauri and TUI adapters with an exhaustive tag <-> CLI runtime `Locale`
   mapping and apply the CLI-owned pure precedence resolver without adding a
   core -> CLI dependency.
3. Test exact serialized bytes/path, missing `LOCALAPPDATA`, two frontends
   reading/writing the same store, concurrent writes, corrupt state, unknown
   locale/version, preserved original bytes, and session-only CLI flag behavior.

Focused validation:

```powershell
rtk cargo test -p devsweep-core presentation_settings
rtk cargo test -p devsweep-cli tui::shell
```

Rollback point: remove store and both adapters as one unit; preserve an existing
unknown-newer store on disk and do not downgrade it.

## 3. Build coordinator and shell registrations

1. Add the typed coordinator and race tests before registering modes.
2. Decompose `App.tsx` into shell/router and mode slots; register only completed
   feature adapters and keep unavailable modes absent.
3. Add TUI navigation/frame, focus restoration, shared brand placement, and
   locale settings without importing domain authority into the shell. Apply the
   CLI-owned accelerator definitions, binary units, and truncation contract with
   collision and full-accessible-text tests in both languages.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli tui
rtk just desktop-web-check
```

Rollback point: unregister the five-mode shell and restore the prior Clean-only
composition while retaining independently valid domain code and icon outputs.

## 4. Automated and manual acceptance

User-approved diagnostic exception (2026-08-30): after three full `just ci`
attempts repeatedly failed the Windows concurrent presentation-store test with
an unscoped `PermissionDenied (os code 5)`, one additional diagnostic-first
repair round is authorized. Add stage-specific context to each store I/O
operation, reproduce once under workspace concurrency, change only the Windows
operation identified by that evidence, and rerun the completion gate once. If
that gate still fails, stop without another retry.

User-approved final convergence round (2026-08-30): implement and test the
missing typed supporting-destination registration, real deep-link/Back history
behavior, and bare-TUI persisted-language restart trace. This authorization does
not permit unrelated refactors, new dependencies, domain logic, or changes
outside the approved Exact Change List.

AskUserQuestion route decision (2026-08-30): the user selected complete TUI
integration. Thread `ShellComposition` through the existing TUI app/runtime/render
architecture, expose a real typed language-settings action, persist it through a
runtime effect, render the selected locale, and prove restart behavior. Preserve
the existing single-App reducer, worker ownership, and Clean safety contracts.

Independent-check corrective decision (2026-08-30): the user selected canonical
catalogue repair and transferred its minimal ownership to this task because
Trellis has no supported unarchive command. Change only
`resources/i18n/zh-CN.json` `command.clean.accelerator` from `Q` to `null`, update
the central i18n and real TUI input/render regressions, and prove that `[Q]` is
absent while `q` continues to invoke Quit. Do not change any other catalogue
field or rewrite the archived CLI task.

Independent-check Desktop failure decision (2026-08-30): AskUserQuestion chose
strict fail-closed UX. Add explicit presentation-store loading/ready/unavailable
state; do not render `AppShell` until load succeeds, and render a stable bilingual
store-unavailable surface on failure without OS/English fallback. Await and
validate a successful save response before changing visible locale; on save
failure or mismatched returned tag, keep the prior locale and show recovery copy.
Add load-pending, load-failure, save-failure, and mismatched-response tests.

AskUserQuestion final static-audit convergence decisions (2026-08-30):

1. Extend the canonical English/Chinese JSON catalogues with only the exact
   additive `shell.v1.*` entries frozen in `design.md`. This supersedes the old
   `Q -> null only` edit restriction only for those additions. Retain the
   approved Chinese Clean `null`; preserve every other existing key/text/form/
   placeholder/metadata byte. Replace TUI/Desktop shell string tables and
   store/route hard-coded copy with typed projections from the canonical
   catalogue. Validate exact key, form, placeholder, plural/count,
   accelerator/group, and truncation parity in Rust and TypeScript.
2. In one combined evidence-driven repair round, close five deterministic
   findings: (a) Desktop service invocation before coordinator ownership,
   including refusal/cancel-failure/unmount races; (b) parallel second TUI Scan
   and stale promotion; (c) non-IEC Clean UI byte labels and duplicate
   formatting; (d) unhandled route cancel/join rejection with localized
   recovery while preserving history/focus; and (e) missing Segoe UI Variable
   precedence. Update verification evidence without claiming independent PASS.

AskUserQuestion final bounded repair and evidence decisions (2026-08-30):

1. Extend the canonical namespace from 19 to exactly 22 keys with
   `shell.v1.supporting.{protection,rules,history}` using the exact forms and
   metadata in `design.md`. Replace caller-supplied supporting labels with the
   closed id -> canonical key mapping and keep unavailable destinations absent.
2. Replace create-new sidecar ownership with real OS-owned lock lifecycle. The
   atomic replace is the commit point; a committed write cannot be reported as
   failed by later cleanup. Inject cleanup failure and abnormal-owner release,
   verify the next transaction safely acquires, retain bounded Windows
   contention codes 5/32/80/183, and assert normal lock/temp residue is zero.
3. Add real `App` plus injected bridge integration assertions for old
   cancel/join before new scan/dry-run/execute service start, closed/refused and
   sync/async start failure, and successful/rejected unmount close without
   unhandled promises or surviving work. Coordinator-only assertions are not
   sufficient.
4. Replace TUI raw persistence failure view state with one stable typed failure
   projected through canonical `shell.v1.persistence.unavailable`. Raw backend
   diagnostics, if retained for internal diagnostics, never enter render state.
5. After focused/broad gates and final hashes are fixed, collect new TUI and
   Desktop Windows evidence under a task-owned process-level `LOCALAPPDATA`.
   Record exact environment, initial/final bytes and hashes, final binary hashes,
   graceful shutdown, and zero process/listener/lock/temp residue. Old native
   evidence is superseded for final-source claims. Scaling, native High Contrast,
   and native Reduced Motion remain `WAIVED/UNVERIFIED`.
6. Keep overall task/evidence state `awaiting independent verification`; final
   source evidence and implementer gates are not independent PASS.

AskUserQuestion final Desktop lifecycle decisions (2026-08-30):

1. After recording a failed task-owned DevSweep residual's name, full path,
   start time, run ownership, and matching binary hash, poll once. Only if every
   identity field still matches, terminate that exact single PID and record the
   command/result. This covered PIDs 35600 and 50416 and is the bounded default
   for later residuals in this Goal; never terminate a different process.
2. Add the frozen `desktop/src/lifecycle.ts` and
   `desktop/src/lifecycle.test.ts` seam before changing lifecycle behavior.
   Register one Tauri close listener without preventing the native close,
   return/await one shared coordinator drain, consume cancel rejection after
   join, and let the installed Tauri listener perform its single built-in
   destroy only after the callback resolves. Prove exactly-once cleanup.
   Do not force process exit, self-kill, hide the window, or abandon work.
3. Add `debug_native_fault_mode` only under `#[cfg(debug_assertions)]` and only
   to the debug `generate_handler!` match arm. It recognizes solely the exact
   process value `DEVSWEEP_TASK_NATIVE_FAULT=route_cancel_once`; release does
   not register or request it. The frontend decodes only
   `disabled|route_cancel_once` and arms one shell-only rejection. No arbitrary
   command, path, argv, shell string, cleanup behavior, or persisted switch is
   accepted.
4. Add focused Rust/TypeScript tests for clean close, active-work close,
   cancel rejection after join, exactly-once drain and Tauri-owned destroy,
   late registration/unmount, closed fault decoding, one-shot route failure,
   unchanged route/hash/focus, dismiss/recovery, and disabled delegation.
5. Run focused tests, lint, typecheck, `desktop-web-check`, `desktop-build`,
   release seam-string audit, `git diff --check`, task validation, and
   `just ci`; then freeze new debug/release/NSIS hashes.
6. Use new task-owned `LOCALAPPDATA` roots for the final release en -> zh save,
   no-flag zh restart, Settings Back/focus, width/icon, and graceful last-window
   close evidence. Separately run the same-source debug build with the exact
   opt-in and prove canonical route-error alert, stable route/hash/focus,
   dismiss/recovery, natural exit, and zero process/listener/lock/temp residue.
   Do not reuse old captures as PASS, and do not change scaling, High Contrast,
   or Reduced Motion.

Third-round ACL repair (AskUserQuestion, 2026-08-30):

1. Preserve the frontend handshake: no `preventDefault`, no competing destroy,
   and one shared coordinator drain that consumes cancellation failure only
   after join.
2. Add exactly `core:window:allow-destroy` to
   `desktop/src-tauri/capabilities/default.json`. Do not add create, close,
   resize, move, minimize, maximize, hide, or any other capability.
3. Parse the capability in a Rust test and require the exact permission array
   `core:event:allow-listen`, `core:event:allow-unlisten`, and
   `core:window:allow-destroy`, each once. Audit the generated desktop schema to
   prove the permission maps only to the `destroy` command.
4. Rebuild all final artifacts and run the sole remaining native round. A
   residual process makes that round fail even when its identity-gated
   termination is authorized; do not start a fourth round.

Focused validation for the lifecycle round:

```powershell
rtk cargo test -p devsweep-desktop debug_native_fault
rtk npm test -- src/lifecycle.test.ts src/App.test.tsx src/app-shell/AppShell.test.tsx src/app-shell/operation-coordinator.test.ts
rtk npm run lint
rtk npm run typecheck
rtk just desktop-web-check
rtk just desktop-build
rtk git diff --check
python ./.trellis/scripts/task.py validate .trellis/tasks/08-29-desktop-shell-navigation-brand
rtk just ci
```

Preserve complete first-failure and repair logs. Change diagnostic method on a
second occurrence of the same failure, and stop after three failed attempts.
Native release and debug harness each receive at most three evidence attempts.

Focused validation for this final bounded round:

```powershell
rtk cargo test -p devsweep-core presentation_settings
rtk cargo test -p devsweep-cli i18n
rtk cargo test -p devsweep-cli tui::shell
rtk cargo test -p devsweep-cli tui::app
rtk cargo test -p devsweep-cli tui::runtime
rtk cargo test -p devsweep-cli tui::render
rtk npm test -- src/i18n/index.test.ts src/app-shell/operation-coordinator.test.ts src/app-shell/AppShell.test.tsx src/App.test.tsx
rtk npm run lint
rtk npm run typecheck
rtk just desktop-web-check
rtk just desktop-build
rtk git diff --check
python ./.trellis/scripts/task.py validate .trellis/tasks/08-29-desktop-shell-navigation-brand
rtk just ci
```

Every failure narrows the next change from new evidence. On the second
occurrence of the same failure class, change the diagnostic method; stop after
three failed evidence-based attempts. Preserve independent-check logs.

Focused validation order for this round:

```powershell
rtk cargo test -p devsweep-cli i18n
rtk cargo test -p devsweep-cli tui::shell
rtk cargo test -p devsweep-cli tui::app
rtk cargo test -p devsweep-cli tui::runtime
rtk npm test -- src/i18n/index.test.ts src/app-shell/operation-coordinator.test.ts src/app-shell/AppShell.test.tsx src/App.test.tsx src/styles.test.ts
rtk npm run lint
rtk npm run typecheck
rtk just desktop-web-check
rtk just desktop-build
rtk git diff --check
python ./.trellis/scripts/task.py validate .trellis/tasks/08-29-desktop-shell-navigation-brand
rtk just ci
```

Before product changes, audit the two catalogue files so all pre-existing
entries remain unchanged except the already approved Chinese Clean accelerator.
Save each command, exit code, and complete log under task-owned evidence. The
implementation report must say `awaiting independent verification`; it must not
mark the independent-check gate PASS. If any one finding repeats, narrow the
next repair from the new failure and change diagnostic method on its second
occurrence; stop after three failed evidence-based attempts.

```powershell
rtk just desktop-web-check
rtk just desktop-build
rtk git diff --check
rtk just ci
```

- On the native Windows application, record English/Chinese screenshots and
  keyboard traces at 390/800/1024/1440 CSS px and 100/125/150/200% scaling. The
  user changes display scaling manually; the agent must not modify the system
  scaling setting.
- User evidence waiver (2026-08-30): do not request or collect the manual
  100/125/150/200% scaling sequence or native High Contrast/Reduced Motion
  observation for this child. Preserve existing 125% material as diagnostic
  only for scaling and exact-dimension claims; reliable process, locale, focus,
  and persistence facts may still support their non-waived clauses. Record the
  skipped evidence as `WAIVED/UNVERIFIED`, non-completion-required; never
  substitute CSS emulation, CSS tests, or DPI-virtualized crops for native
  evidence.
- Verify visible focus, high contrast, reduced motion, title/taskbar/header icon,
  locale-specific accelerators, collision handling, unit boundary values,
  long-label/user-data truncation with full accessible text, rapid mode switching,
  app close, and zero owned work after cancel/join.
- Record persisted-language restart behavior from both TUI and desktop; confirm
  machine JSON is byte-identical across locale changes.
- Stop and return to planning if the shared store needs fields beyond locale,
  if a mode needs shell authority, or if a mode cannot be hidden atomically.
