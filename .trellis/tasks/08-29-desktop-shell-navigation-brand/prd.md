# Build Windows desktop and TUI shell, navigation, and brand system

## Goal

Implement the original DevSweep five-mode shell, navigation, shared lifecycle,
single bilingual presentation-setting owner, generated-brand placement,
responsive Windows behavior, and the governing desktop spec update.

## Requirements

- R1: Build one responsive shell for Clean, Software, Optimize, Analyze, and
  Status. Protection, Rules, History, Settings, and language are supporting
  destinations. An unavailable mode is absent, never a clickable placeholder.
- R2: Use native Windows chrome, original DevSweep icon, restrained gray-green
  tokens, system typography, and structural depth. Reject traffic lights,
  planets, glass, copied geometry, and page-wide ornamental palette shifts.
- R3: Exclusively own the versioned persisted TUI/Desktop locale preference and
  both adapters. Core owns only a closed `PresentationLanguageTag` (`en|zh-CN`)
  and never imports CLI; the TUI/Desktop adapters exhaustively map that tag to
  the CLI task's runtime `Locale`, catalogue, and pure resolver. Do not redefine
  catalogue keys or CLI locale precedence. Persist only
  `{schema_version, language}` at the fixed Windows path through a cross-process
  lock and atomic replacement.
- R4: Own one operation coordinator shared by TUI and desktop adapters. A mode
  change cancels and joins Scan/Analyze/Software/Optimize/live Status before
  another heavy operation starts. The coordinator never grants domain authority.
- R5: Own deterministic routing/deep links, focus restoration, keyboard
  navigation, reduced motion, live regions, error presentation, high contrast,
  locale-specific accelerator binding/collision handling, non-destructive
  truncation with full accessible text, and 390/800/1024/1440 responsive
  behavior. Consume the CLI-owned plural/unit/accelerator/truncation contract;
  do not redefine catalogue metadata.

  Corrective ownership decision (2026-08-30): independent check proved the
  archived Chinese `command.clean` accelerator `Q` conflicts with the real TUI
  Quit key and is not an executable navigation binding. Through AskUserQuestion
  the user transferred the minimal canonical correction to this task and chose
  `accelerator: null`.

  Superseding catalogue decision (2026-08-30): AskUserQuestion selected one
  canonical shared English/Chinese catalogue for all shell, settings,
  persistence, and route-failure copy. This supersedes the earlier prohibition
  on every catalogue change other than `Q -> null` only to authorize the exact
  additive `shell.v1.*` key set frozen in `design.md`. Existing command and
  non-shell keys, text, forms, placeholders, count, accelerator, group, and
  truncation metadata remain byte-for-byte unchanged; the approved Chinese
  Clean `accelerator: null` remains the sole mutation to an existing entry.

  Final independent-audit decision (2026-08-30): AskUserQuestion authorized one
  bounded repair round and final-source native evidence. The canonical shell
  namespace is exactly 22 keys: the existing 19 plus closed supporting labels
  `shell.v1.supporting.protection`, `shell.v1.supporting.rules`, and
  `shell.v1.supporting.history`. Supporting registrations identify a closed
  destination id and never accept caller-supplied bilingual labels.
- R6: Before any product UI edit, update
  `.trellis/spec/desktop-frontend/component-guidelines.md`,
  `state-management.md`, and `index.md` from the Clean-only contract to the
  five-mode shell/mode-local-state contract while retaining every cleanup
  observation/selection/digest/confirmation safety invariant. The update also
  replaces the conflicting surface-palette clause with the original deep
  gray-green token system and narrowly permits non-informational CSS-native
  sweep/orbit motifs subject to reduced motion, while continuing to prohibit
  heroes, decorative illustrations, gradients/glow/glass, and copied geometry.
- R7: Integrate the icon child's stable master/native outputs. This child owns
  React/TUI placement, accessible naming, responsive tests, and native shell
  evidence; it does not regenerate native icon files.

## Acceptance Criteria

- [ ] AC1 (R1, R5): Five primary modes and supporting destinations have tested
      routing, deep-link/back behavior, absent-feature behavior, focus restore,
      and keyboard navigation.
- [ ] AC2 (R4): Coordinator race tests cover rapid switching, cancel request,
      join-before-start, stale events, close/unmount, and no surviving owned work.
- [ ] AC3 (R2, R5): English and Chinese preserve authority/actions and full
      accessible user data at all target widths; locale-specific accelerators
      are collision-free, binary units match the CLI contract, truncation never
      hides authority/action text, and native 100/125/150/200% scaling,
      keyboard, high contrast, and reduced-motion evidence passes.

      User evidence waiver (2026-08-30): the user explicitly skipped the manual
      100/125/150/200% display-scaling matrix and subsequently waived native
      High Contrast and Reduced Motion observation for this child. Existing
      125% captures remain diagnostic only for scaling and exact-dimension
      claims; independently reliable process, locale, focus, and persistence
      facts may still support their non-waived clauses. Missing, DPI-virtualized,
      High Contrast, and Reduced Motion native evidence stays
      `WAIVED/UNVERIFIED`, is never represented as PASS, and is not
      completion-required for this child. All other AC3 clauses remain required.
- [ ] AC4 (R3): TUI and desktop read/write the same versioned locale store;
      V1 bytes/path and tag mapping match the design; corrupt, unknown-tag, and
      unknown-newer state is retained and fails safely; session changes preserve
      mode state, and machine output is unchanged.
      The atomic replace is the commit point: after it succeeds the save API
      returns success even if best-effort lock-name cleanup is unavailable.
      Ownership is an OS-managed handle/lock, not sidecar existence, so normal
      release, cleanup failure, or abnormal prior-owner exit cannot permanently
      block the next transaction and normal Windows transactions leave no lock
      or temporary-file residue.
- [ ] AC5 (R6): The three owned desktop spec files are updated before product UI,
      mode tasks treat them as post-shell constraints, and retained Clean safety
      clauses remain traceable. The diff explicitly resolves the current surface
      palette and decorative illustration/motion clauses with the bounded
      gray-green/non-hero motif policy from R6.
- [ ] AC6 (R7): The original icon is crisp at shell/native sizes, has one
      accessible product name, and no runtime/reference path enters `.trellis`.
- [ ] AC7 (R1, R4, R5): Desktop web checks, TUI tests, Tauri build, focused
      accessibility checks, `git diff --check`, and `just ci` pass.

## Out of Scope

- Mode-specific data, rows, treemap, collectors, plans, execution, or audit.
- Recreating Mole artwork, changing native window controls, editing canonical
  command/non-shell catalogue entries beyond the approved Chinese Clean
  accelerator correction, or persisting settings other than the locale tag.

## Final static-audit convergence decision

AskUserQuestion authorized one combined evidence-driven repair round on
2026-08-30. It is limited to: canonical `shell.v1.*` catalogue consumption,
cancel-and-join-before-start ordering for Desktop and TUI heavy work, canonical
IEC byte formatting in the Clean UI, fail-safe localized route-transition
errors without focus/history regression, Segoe UI Variable precedence, and
truthful verification evidence. Automated success leaves this task awaiting an
independent check; it is not itself independent PASS evidence.

## Final bounded audit and evidence decision

AskUserQuestion authorized a final bounded repair and final-source native
evidence round on 2026-08-30. It closes the store commit/lock-lifecycle defect,
real Desktop bridge/coordinator ordering and unmount rejection handling, TUI
raw persistence-error leakage, and canonical supporting-destination labels.
Final native evidence must use a task-owned process-level `LOCALAPPDATA`, final
binary hashes, graceful shutdown, and zero owned process/listener/lock/temp
residue. Earlier native captures are superseded for final-source PASS claims.
Display scaling, native High Contrast, and native Reduced Motion remain the
user-approved non-completion-required `WAIVED/UNVERIFIED` items. Automated and
native implementer evidence leaves the task `awaiting independent verification`.

## Final Desktop lifecycle and native-fault decision

AskUserQuestion authorized one additional bounded repair on 2026-08-30 after
the final release window was destroyed but its headless main process survived.
It also authorized termination of PID 35600 after a read-only identity check
proved its name, executable path/hash, start time, command line, and windowless
state matched that failed native run. A later AskUserQuestion authorized the
same identity-gated single-process action for PID 50416 and, for the remainder
of this Goal, for only a solitary residual DevSweep process whose path, start
time, executable hash, and run ownership all match a failed task-owned native
run after graceful close. This default never extends to another process or
another high-risk action.

This round must replace fire-and-forget React unmount cleanup with a real Tauri
last-window close handshake: do not prevent the native close, return one shared
coordinator drain from the Tauri callback, consume cancellation failure only
after join settles, and allow the installed Tauri listener to perform its
single built-in destroy only after that callback resolves. Forced
`std::process::exit`, self-kill, hiding the window, and abandoning joined work
are prohibited.

The user also authorized a closed debug-only native fault harness for the
existing route-cancel/join error branch. It is compiled and registered only in
debug Rust builds, requires the exact process-local opt-in
`DEVSWEEP_TASK_NATIVE_FAULT=route_cancel_once`, and returns only the closed
`disabled|route_cancel_once` mode. It accepts no command, path, argv, shell
string, or cleanup authority. Release Rust does not register the command and
the production frontend does not request it. Native evidence must distinguish
the final release normal path from the same-source debug fault path.

The third-round native diagnosis proved the installed Tauri
`onCloseRequested` adapter awaits the shared drain and then invokes the guarded
`window.destroy` command. The generated desktop ACL schema names that command's
exact permission `core:window:allow-destroy`; the existing capability omitted
it, so native close rejected after a correct drain and left the window/process
alive. AskUserQuestion authorized adding that one permission only. It grants no
window create, close-before-drain, resize, move, minimize, maximize, hide, or
system authority.
