# Rebuild Tauri desktop around Mole-inspired workbench and CLI-backed performance

## Goal

Give the Windows Tauri application a coherent, original five-mode workbench
informed by Mole's information hierarchy, while making every query, analysis,
preview, and cleanup operation use one typed command contract and a measurable
performance path. The user should be able to scan, inspect, select, preview,
confirm, analyze, optimize, and observe status without leaving the desktop
surface or losing DevSweep's fail-closed safety rules.

## User value

The current application has the capability seams, but the workbench still
reads as a collection of mode pages. A persistent visual grammar, truthful
progress/results, and evidence-backed orchestration make the same capabilities
understandable and responsive for repeated developer use.

## Confirmed facts

- The local Mole reference is `ref/Mole` at commit
  `014a25f88db3fd2e5ef65011c38954b827c9a8b1`; its checkout and licence are
  research inputs only. The detailed accept/adapt/reject audit is in
  `research/current-state-and-mole-reference.md`.
- Mole's requested surface groups Clean, Software, Optimize, Analyze, and
  Status behind one entry point. DevSweep already ships those five typed modes
  plus Protection, Rules, and History supporting destinations.
- `desktop/src/api/bridge.ts:20-47` owns the five-mode and supporting-domain
  `DesktopBridge`. Presentation settings use the separate typed bridge at
  `desktop/src/i18n/index.ts:165-167`; window lifecycle and DEV-only fault
  injection use `desktop/src/lifecycle.ts:21-35`. `OperationCoordinator` owns
  heavy-operation cancellation and join; Tauri adapters delegate domain work
  to `devsweep-core`.
- The existing backend/desktop specs require a thin Tauri adapter over core,
  no CLI-crate import, program/argv separation, no permanent delete, explicit
  cleanup preview digest plus second confirmation, inspect-only exclusion, and
  truthful partial/unknown capacity.
- The current shell is already dark/sidebar based, but generic empty/table
  surfaces and light fallback tokens remain in `desktop/src/styles.css`; the
  new task must refine the workbench rather than recreate archived shell work.
- Prior native/resource reports are historical baselines. They do not prove
  the current task's performance or native acceptance.

## Requirements

### R1 — Original Mole-informed workbench

Adapt the reference's five-tool entry point, review-first flow, grouped rows,
progressive detail, treemap drill-down, and stable action summaries into
DevSweep's own Windows visual system. Keep the persistent sidebar, visible page
header, mode-local state, supporting destinations, keyboard navigation, dark
canvas, reduced motion, forced colors, and English/Simplified Chinese. Do not
copy Mole assets, strings, planets, geometry, traffic lights, or macOS-only
features.

### R2 — One typed command path

Every five-mode and Protection/Rules/History domain operation must cross
`DesktopBridge` and a typed Tauri command, decode a closed wire result/error,
and reuse the corresponding core owner used by the CLI. Keep the existing
typed presentation-settings and window-lifecycle adapters as explicit
non-domain exceptions; do not consolidate them merely to create one file.
No React component may invoke Tauri directly. No shell command
string, arbitrary executable, or UI-invented plan/action may cross the boundary.
“Tauri 调用 CLI” means a shared typed service layer used by both the Tauri
adapter and CLI application; the desktop does not launch a second `devsweep`
process or parse its stdout.
The user selected this interpretation; an external CLI child process is out of
scope for this task.

### R3 — Performance and lifecycle

Keep one heavy operation in flight across modes. Preserve cancel-request and
join-before-replace semantics, stale-event rejection, channel backpressure,
bounded traversal/process sampling, and zero owned work after close or route
change. Add a release-build benchmark harness that records raw samples,
host/build/fixture hashes, p95 latency, median and p95 CPU, peak private bytes,
thread count, render/layout samples where applicable, and separately named
cancel-request, acknowledgement, and join times. Apply the operation-specific
timing and completion predicates in the performance child's design; do not
invent a universal 500 ms cancel-to-join promise. Clean dry-run/execute keep
their wait-for-completion semantics and may not be force-killed to meet a gate.
Use fresh paired baseline/candidate measurements with identical host, fixture,
toolchain and release configuration and separately recorded source/binary
hashes. Preserve the frozen thresholds. Invalid evidence and missing required
evidence cannot pass; a measured failure remains `FAIL`.

### R4 — Safety and truthful evidence

Preserve the scan-observation → saved-plan → dry-run digest → second
confirmation → execution funnel. Permanent delete remains unavailable; scanner,
model, and UI layers only create or present plans. Inspect-only targets remain
unselectable. Partial/lower-bound/unknown capacity remains distinct, and copy
never claims that space was already freed.

### R5 — Mode behavior

- Clean keeps visible scope, indeterminate backend progress, cumulative preview,
  grouped targets, selection invalidation, dry-run, confirmation, and truthful
  trash/result copy.
- Software keeps versioned inventory, manual-only unsupported products, explicit
  selection, preview digest, current-user MSIX authority, post-dispatch audit,
  and recovery states.
- Optimize exposes only the closed catalogue and its preview/execute/audit
  lifecycle; no administrator or arbitrary command path is added.
- Analyze remains read-only with bounded traversal, list/treemap drill-down,
  breadcrumb navigation, and partial/unknown sizing states.
- Status remains snapshot/live read-only with bounded intervals, availability
  states, process limits, and stop-and-join behavior; no health score or fake
  zero is introduced.

### R6 — Verification and provenance

Record bilingual keyboard/rendering evidence at 390, 800, 1024, and 1440 CSS
pixels, reduced motion, and forced colors. Required scaling evidence is
WebView device scale 100/125/150/200% plus the current actual Windows display
scale, without changing user display settings. Record other actual Windows
scales as `UNVERIFIED`, outside this round's completion gates. Label CSS
viewport emulation separately from real window dimensions.
Verify the current unsigned NSIS build's identity and the matching executable's
native window identity without installation, signing, or publishing. Run
desktop and Rust gates, keep machine JSON/NDJSON locale-neutral, and preserve
unrelated working-tree dirt.

## Key decisions

- **Shared typed service:** Tauri and the CLI reuse one in-process service/core
  path. This preserves the current thin-adapter rule, avoids a JSON/process
  round-trip, and keeps cancellation, plan authority, and audit in one owner.
  A literal external CLI child process is out of scope for this task.
- **Original reference adaptation:** Mole contributes information hierarchy
  ideas only; DevSweep keeps its own Windows identity, copy, safety semantics,
  and accessibility contract.
- **2026-09-21 review revision:** retain typed settings/lifecycle adapters as
  non-domain exceptions (TPR-04). The user selected four WebView scale factors
  plus the unchanged current Windows scale; other actual OS scale factors are
  non-gating `UNVERIFIED` evidence (TPR-05). Package acceptance covers the local
  build and running executable, not installer execution or installed shortcuts.
- **Evidence before implementation:** this task may write planning artifacts
  and manifests, but no product file or `task.py start` runs until the final
  summary is reviewed and explicitly approved.

## Acceptance Criteria

- [ ] AC1 (R1, R6): The task research pins the local Mole commit, source URL,
      licence/trademark boundary, and accept/adapt/reject matrix; no Mole
      source, asset, string, or geometry enters production or tests.
- [ ] AC2 (R1): Clean, Software, Optimize, Analyze, and Status share one
      original workbench grammar with persistent navigation, visible headings,
      stable action/status surfaces, progressive detail, and truthful empty,
      loading, partial, error, canceled, preview, and result states in both
      locales at the four layout widths.
- [ ] AC3 (R2, R4): Every five-mode/supporting-domain query/analysis/preview/cleanup path has a
      typed bridge command, closed decoder, structured error, and fixture that
      matches the corresponding CLI/core semantics; typed settings/lifecycle
      exceptions have their own decoder/lifecycle tests. No component invokes
      Tauri directly and no path bypasses plan/digest/confirmation authority.
- [ ] AC4 (R3): The release benchmark records raw samples and manifests for
      idle, Clean, Analyze, Software, Optimize, and Status. The frozen
      five-mode thresholds, operation-specific cancellation/completion
      predicates, no-overlap rule, and same-live-PID Status post-stop quiescence
      all pass. Clean has a fresh comparable baseline and all three real
      elapsed/private-bytes/thread comparisons; no recorded-only or invalid
      comparison is a pass.
- [ ] AC5 (R3, R4, R5): Cross-mode tests prove single-flight coordination, stale event
      rejection, route-close cancellation, inspect-only refusal, selection and
      rescan invalidation, partial/unknown preservation, and no owned work
      after unmount.
- [ ] AC6 (R6): Required native rows pass for keyboard/focus, bilingual copy,
      reduced motion, forced colors, four WebView scale factors, the unchanged
      current Windows scale, local package/executable identity, and no
      unintended UAC/elevated child. Other actual Windows scales remain
      explicitly non-gating `UNVERIFIED`; missing required evidence blocks
      completion instead of becoming a pass.
- [ ] AC7 (R6): `just ci`, desktop types/lint/typecheck/test/build, read-only generated IPC
      checks, and `git diff --check` pass; unrelated dirt is unchanged.
- [ ] AC8 (R6): Parent and recursive child PRDs/designs/implement plans plus real
      `implement.jsonl` and `check.jsonl` entries pass Trellis validation and
      independent plan review before any child starts.

## Child map

| Child                           | Responsibility                                                                                                                                                                     | Ordering                                               |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| `desktop-mole-workbench-ux`     | Original shell/mode composition, visual tokens, responsive/accessibility polish, and UI evidence.                                                                                  | First UI child; consumes the existing typed contracts. |
| `desktop-tauri-cli-service`     | Resolve the selected Tauri↔CLI service boundary, command parity, stream/error mapping, and cross-surface fixtures.                                                                 | Before performance integration.                        |
| `desktop-operation-performance` | Coordinator/backpressure/cancellation improvements, release benchmark harness, paired Clean baseline, same-live-PID Status stop window, and the operation cancel/completion table. | After service boundary; may share UI fixtures.         |
| `desktop-native-acceptance`     | Native Windows matrix, local package/executable identity, resource gates, and final cross-mode evidence.                                                                           | Last; depends on all prior children.                   |

The parent owns requirement convergence, dependency links, shared spec updates,
and final integration review. It is not an implementation target.

## Out of scope

- Mole GPL code/assets/copy, pixel-identical recreation, or macOS-only menu-bar,
  Touch ID, fan, screen, or broad privilege features.
- Permanent deletion, arbitrary commands, UAC elevation, privileged helpers,
  background daemons, telemetry, new production dependencies, or unrelated
  cleanup providers.
- Reopening archived shell tasks, changing TUI behavior without a traced
  contract need, publishing, pushing, committing, archiving, or starting a
  child before the final review is approved.
- Launching an external packaged CLI process from Tauri; process isolation,
  stdout parsing, and child-process lifecycle would require a separate task.
