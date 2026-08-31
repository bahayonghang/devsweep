# Mole-Inspired DevSweep CLI and Windows Desktop Redesign

## Goal

Redesign DevSweep's command taxonomy and Windows desktop experience around a
coherent set of capability-backed user modes. Mole's public CLI behavior and the
user-supplied Mole for Mac screenshots are research references only. DevSweep
remains an independent MIT implementation with its own name, assets, copy,
Windows interaction patterns, and safety model.

The result must make cleanup, read-only disk analysis, protection, and rule
inspection easier to discover without collapsing DevSweep's
discovery -> untrusted plan -> validation -> dry-run -> confirmation -> execution
boundaries.

## Confirmed Background

- The user replaced the earlier limited UI direction with this five-mode
  programme. The retained foundation parent now records that React/TUI brand
  placement belongs to the desktop-shell child; its earlier approval does not
  authorize implementation of this materially different design.
- The previous parent is retained as a foundation subtree so its history is not
  rewritten. Its sizing child remains `in_progress`, paused, and partially
  unverified; its icon child must later surrender React shell/layout ownership.
- Current DevSweep CLI commands are `tui`, `scan`, `inventory`, `clean`,
  `protect`, and `rules`
  (`crates/devsweep-cli/src/application/cli.rs:14-29`).
- Current desktop behavior is one cleanup workflow backed by scan, cancellation,
  dry-run, confirmation, execution, and protection-list Tauri commands
  (`desktop/src-tauri/src/lib.rs:7-16`; `desktop/src/App.tsx:24-98`).
- Mole's public command families include cleanup, uninstall, optimize, analyze,
  status, purge, installer, and history. They are not one interchangeable safety
  layer (`ref/repo/Mole/lib/core/help.sh:3-79`;
  `ref/repo/Mole/README.md:49-85`).
- The local Mole checkout is conflicted and divergent. The clean locally known
  `origin/main` commit `f92133a4d6277574177e0b1284742072fb3b5bdc` is the
  code-reference baseline. The dirty working tree may supply research leads but
  is not an authoritative contract.
- Mole CLI source is GPL-3.0, while DevSweep is MIT. The Mole name, logo, and Mole
  for Mac assets are separately reserved. DevSweep's existing provenance policy
  already forbids copying or lightly rewriting Mole source, fixtures, rule
  tables, UI text, or assets (`docs/provenance.md:5-35`;
  `ref/repo/Mole/TRADEMARK.md:3-14`).
- The user selected a full five-mode Windows program for this parent. Clean,
  Software, Optimize, Analyze, and Status must each become a real end-to-end
  capability; staged delivery is still required, but none is merely a future
  placeholder or out-of-parent roadmap note.
- Software requires a separate inventory/plan/validation/execution domain. The
  cleanup plan cannot carry installer authority. The smallest safe inventory
  covers ARP registry records, MSI identity, and current-user MSIX packages;
  registry vendor uninstall strings remain untrusted observations.
- Optimize requires a separate closed maintenance catalogue and authorizer. A
  non-elevated v1 can execute a fixed DNS-cache refresh and hand off supported
  Storage/Search/Energy reviews to Windows Settings; arbitrary registry tweaks,
  service changes, security changes, and broad “repair” commands are rejected.
- Status is a bounded, read-only, on-demand collector. CPU, memory, fixed-volume
  capacity, network interfaces, battery, and process metrics are feasible with
  documented Win32 APIs; GPU utilization, temperature, and fan speed must be
  explicit unsupported/unavailable states in the first version.
- The user selected a strict no-elevation policy. DevSweep remains `asInvoker`,
  never launches a UAC flow, and does not introduce a privileged helper. Machine
  MSI and protected-process gaps are reported as inventory/partial evidence, not
  worked around with administrator authority.
- The user selected an immediate breaking CLI redesign. Existing top-level
  command names and current JSON documents are not retained as compatibility
  aliases or a deprecation layer. The new command tree must still preserve the
  underlying plan-validation and explicit-execution safety mechanisms, publish
  new versioned output contracts, and include a complete migration guide.
- The user selected bilingual runtime surfaces. CLI human output, TUI, and
  desktop UI must support English and Simplified Chinese, while machine-readable
  schemas, identifiers, error codes, and audit records remain locale-neutral.

## Requirements

- R1: Clean-room reference boundary.

Pin every Mole-derived research claim to the clean reference commit or identify
it explicitly as screenshot/dirty-worktree evidence. Reimplement only abstract
behavior and information-architecture ideas. Do not copy source, tests, tables,
copy, layouts pixel-for-pixel, brand elements, or planetary artwork.

- R2: Capability-backed CLI taxonomy.

Define one responsibility per command, including help, exit status, stdout vs
stderr, human output, JSON/NDJSON stability, aliases, and compatibility. Preserve
the existing plan-validation and explicit-execution mechanisms even though the
public command shapes and document versions are replaced. No command may imply a
capability that has no core service and testable safety contract.

The design must explicitly map, accept, rename, defer, or reject each relevant
Mole command family instead of treating parity as a goal.

- R3: Capability-backed desktop information architecture.

The desktop may expose only modes with real end-to-end contracts. The five
approved modes are:

- **Clean**: existing scan -> completed report -> selection -> dry-run -> second
  confirmation -> execution flow, with Protection and Rules as supporting Clean
  surfaces;
- **Software**: versioned Windows inventory -> explicit selection -> preview ->
  current-user MSIX-only uninstall authorization -> post-state audit; every MSI
  remains inventory/manual in V1;
- **Optimize**: closed maintenance catalogue -> preview -> confirmation -> fixed
  operation or Windows-owned Settings handoff -> audit;
- **Analyze**: read-only inventory with truthful verified/partial/unknown sizing;
- **Status**: bounded snapshot and opt-in live read-only metrics.

No clickable placeholder may be shipped before its backend/IPC/error/validation
contract exists. History is a supporting audit surface, not a sixth product
mode. Installer remains rejected unless a later separately approved task defines
an independent Windows contract.

- R4: Preserve safety and evidence semantics.

- Scans, active previews, and inventory observations never become execution
  authority.
- Permanent deletion remains disabled; reparse points remain no-follow/fail
  closed; Cargo home remains inspect-only; Docker cleanup remains deferred.
- Incomplete capacity remains a lower bound or unknown, never a precise total.
- UI copy describes estimated recoverable capacity. Trash outcomes must not claim
  that capacity is already freed.
- Inspect-only targets remain unselectable, and every changed selection/rescan
  invalidates stale dry-run authority.

- R5: Windows-first original visual system.

Use the screenshots to study mode navigation, dense grouped rows, stable action
summaries, progressive disclosure, analysis hierarchy, and staged feedback.
Create an original DevSweep dark developer-workbench identity using Windows
native title-bar conventions, Segoe UI Variable/system fonts, visible focus,
keyboard equivalence, reduced-motion behavior, and responsive layouts.

Reject fake macOS traffic lights, Mole branding, photo planets, continuous WebGL,
glass effects, invented percentages, oversized “space freed” hero claims, and
page-wide ornamental color changes. Any relaxation of the current no-hero/no-
illustration desktop spec must be written into the spec before UI implementation.
The desktop-shell child owns that spec-first gate and must explicitly reconcile
the gray-green surface palette and restrained CSS-native non-hero motifs with the
current surface-color and decorative-illustration/motion clauses.

- R6: Foundation task preservation.

Retain `08-29-scan-resource-bounds-app-icon` as a foundation subtree:

- the sizing child owns only bounded/cancelable filesystem sizing and accepted
  same-host throughput evidence;
- the icon child is narrowed to the stable generated master, native Tauri icon
  inventory, and 16/32 px Windows validation;
- React shell/header integration moves to the future desktop-shell child;
- neither child resumes until the new parent plan and its dependency order are
  approved.

- R7: Full five-mode delivery with independent safety ownership.

CLI contract, desktop visual contract/shell, Clean workflow, Analyze domain/IPC,
Analyze UI, Protect/Rules, high-risk Windows product-fit work, and final native
integration must have distinct acceptance ownership and rollback boundaries.
The same parent must also own real end-to-end vertical slices for:

- **Software**: Windows application inventory, evidence-backed size and truthful
  last-used state, preview, explicit selection, current-user MSIX-only
  authorization, explicitly irreversible uninstall policy, deterministic
  post-dispatch outcomes/recovery, post-state requery, and audit;
- **Optimize**: a closed catalogue of bounded Windows maintenance operations,
  preview, capability/privilege checks, refusal reasons, execution, and audit;
- **Status**: read-only Windows health and process metrics with bounded sampling,
  unavailable states, snapshot output, and opt-in live refresh.

These three modes require separate child PRDs and may start only after the final
parent plan names their exact authority and the user explicitly approves it.

- R8: Software trust boundary.

Software inventory and uninstall must use a versioned, separate domain with
tagged application identities, immutable inventory fingerprints, an untrusted
selection plan, opaque validated strategies, a preview digest, single-flight
execution, durable audit, and live installed-state revalidation.

- Enumerate ARP records from HKCU/HKLM and both registry views, authoritative MSI
  products for machine/current-user contexts, and current-user MSIX packages.
- Never execute or serialize `UninstallString`, `QuietUninstallString`,
  `DisplayIcon`, arbitrary command lines, or shell text.
- Reconstruct executable strategies only for validated current-user MSIX package
  identities. Every MSI record, including current-user managed/unmanaged and
  machine contexts, remains visible but unselectable with a stable V1
  manual-management reason.
- Registry-only vendor apps remain inventory/manual handoff until a trusted
  product-specific adapter exists.
- Never infer related-data ownership from display name or publisher. Unknown
  related data is retained.
- Every uninstall is irreversible from DevSweep's perspective; cancellation or
  timeout after dispatch is re-queried and otherwise reported `unknown`.
- Before a Software side effect, durably record `dispatch_started`; never retry
  it automatically. On restart, requery the exact tagged identity and finish as
  `removed`, `reboot_required`, `still_present`, `failed`, or
  `unknown_after_dispatch`. Software execution has no independent `partial`
  terminal; `partial` remains an inventory/source evidence state only.
- Size evidence is versioned and tagged by basis. MSI/ARP may expose only a
  clearly labelled reported estimate; current-user MSIX may expose a bounded
  measured installed-location value/lower bound. Last-used is a real typed
  field, but V1 reports `unknown/no_supported_exact_source` unless a documented
  exact-identity Windows provider exists; installation/update dates and
  heuristic usage artefacts are not substitutes.

- R9: Optimize maintenance boundary.

Optimize must use a versioned closed catalogue, preview/confirmation digest,
maintenance-specific authorizer, exclusive locked journal, one operation in
flight, fixed program/API ownership, pre-dispatch cancellation, and
`unknown_after_dispatch` outcomes when a system operation cannot be proven.

The default non-elevated catalogue contains:

- fixed-system-path DNS resolver cache refresh with fixed argv;
- allowlisted Windows Settings handoffs for Storage recommendations, Search
  indexing, and Energy recommendations where the OS supports them;
- read-only guidance for drive optimization, system integrity, filesystem check,
  and network reset, without executing those administrator actions.

Optimize must never disable or reconfigure security software, UAC, Windows
Update, firewall, power/pagefile/hibernation, arbitrary services, or user-supplied
commands.

The application must not request administrator rights for Optimize. Administrator
maintenance stays read-only guidance or a Windows-owned Settings handoff.

- R10: Status sampling boundary.

Status must provide one versioned snapshot and an explicitly started live mode.
It must not create a background service, notification agent, tray monitor,
scheduled task, persistent history, or synthetic health score.

- Snapshot rate window: 500 ms. Live default: 2 s, bounded to 1-60 s.
- CPU/memory/network each tick; processes every 4 s; volumes/battery every 30 s.
- Default 15 processes, maximum 100 returned and 4096 enumerated; process detail
  work has a 150 ms cooperative budget and returns partial when exceeded.
- At most one sample is in flight. Missed ticks are skipped, never backfilled in
  parallel.
- Live Status is mutually exclusive with Scan, Analyze, Software, and Optimize;
  leaving the mode cancels and joins it.
- Metric DTOs distinguish available, partial, unavailable, permission-denied,
  and unsupported; missing hardware is never rendered as zero.
- The first version collects process name/PID/CPU/memory/I/O only, not command
  line, full executable path, username, or long-term history.
- JSON snapshot, NDJSON live, and Tauri IPC use the one frozen Status V1 wire
  schema from the CLI contract: integer bytes/basis-points/timestamps, monotonic
  elapsed durations, the closed availability union, explicit process
  truncation/budget metadata, monotonic event sequence, and deterministic
  terminal/broken-pipe lifecycle. Adapters may not invent surface-local shapes.

- R11: Breaking CLI command contract.

Replace the current top-level command taxonomy with one organized around the five
product modes:

- `clean`: discovery, saved-plan creation, preview/dry-run, confirmed execution,
  protection, and rule discovery through explicit subcommands;
- `software`: list, preview, and current-user uninstall through its separate
  software plan domain;
- `optimize`: list, preview, and run the closed maintenance catalogue;
- `analyze`: read-only disk analysis and versioned JSON output;
- `status`: snapshot and explicit live/NDJSON output;
- supporting read-only `history` for versioned audit queries.

Running `devsweep` without a subcommand opens the interactive TUI. The old
top-level `tui`, `scan`, `inventory`, `protect`, and `rules` command shapes are
removed rather than kept as aliases. The existing `clean --plan` behavior is
re-expressed under explicit Clean preview/execute subcommands; execution still
requires a saved untrusted plan, current preview digest, and explicit execute
authority.

The CLI child must own exact subcommand/flag grammar, help, examples, exit-code
classes, stdout/stderr separation, TTY behavior, JSON/NDJSON schemas, broken-pipe
behavior, generated docs, and a table mapping every old invocation to its new
equivalent or deliberate removal.

- R12: Bilingual runtime contract.

CLI human output, the TUI, and the desktop UI must share versioned English and
Simplified Chinese message catalogues rather than maintain independent literal
translations. Locale selection follows one deterministic contract:

- explicit `--language en|zh-CN` wins for human-readable CLI output;
- TUI and desktop expose and persist the same explicit language choice;
- without an explicit choice, resolve the supported Windows user locale and
  fall back to English for unknown or incomplete locales;
- JSON, NDJSON, stable field names, enum values, error codes, plan digests, and
  audit records never vary by locale;
- diagnostics may add localized human text only alongside stable codes;
- interpolation, pluralization, units, accelerators, truncation, and layout must
  be tested in both languages, including 100%, 125%, 150%, and 200% Windows
  scaling for native UI evidence.

Missing translation keys fail validation rather than silently shipping mixed
language. User-controlled paths, registry text, application names, and command
output are data and must never be used as catalogue keys or interpreted as
markup.

- R13: Deterministic resource acceptance.

Final integration must use a same-host, release-build protocol with a recorded
host/build manifest, fixed fixtures, one warm-up plus five measured repetitions,
200 ms process sampling, named statistics, and numeric CPU/private-memory/thread/
latency thresholds for idle, Status snapshot/live, Clean, Analyze, Software, and
Optimize. A threshold failure is a stop/go failure, not a subjective "material
regression" judgment. OS-owned Settings work is reported separately from the
DevSweep process.

## Acceptance Criteria

- [ ] AC1 (R1): Research records the pinned clean Mole commit, dirty-worktree
      status, GPL/trademark boundary, screenshot provenance, and explicit
      accept/adapt/reject decisions.
- [ ] AC2 (R2): A command matrix covers every current DevSweep command and each
      considered Mole family, with responsibility, compatibility, output,
      side-effect level, Windows mapping, and disposition.
- [ ] AC3 (R2, R3): Every planned CLI command and desktop mode traces to a real
      core service/DTO/IPC owner or is marked research/deferred; no empty mode is
      scheduled for product implementation.
- [ ] AC4 (R4): The design traces discovery, preview, plan validation, dry-run,
      confirmation, execution, inventory, and protection as separate states and
      keeps the current fail-closed rules intact.
- [ ] AC5 (R5): The desktop brief specifies navigation, Clean and Analyze
      topology, states, responsiveness, accessibility, visual tokens, native
      Windows behavior, and details explicitly rejected from the references;
      the shell-owned spec update resolves the surface palette and permitted
      CSS-native non-hero motif clauses before any mode UI begins.
- [ ] AC6 (R6): The old parent is linked as a foundation subtree; the sizing
      checkpoint is recorded as paused/partially unverified; icon React ownership
      is removed before any shell implementation begins.
- [ ] AC7 (R7): The final parent/child graph includes real Software, Optimize,
      and Status vertical slices plus explicit dependencies, file and
      contract ownership, focused checks, native evidence gates, rollback points,
      and approval requirements for every high-risk Windows behavior.
- [ ] AC8 (R7): Parent and recursive child PRDs/designs/implement plans/manifests pass Trellis
      validation and independent plan review before any `task.py start`.
- [ ] AC9 (R8): Software plans contain no argv/uninstall strings; hostile
      registry fixtures cannot become executable strategies; all MSI contexts
      are manual-only; exact current-user MSIX identity, stale preview,
      protected entries, durable dispatch/restart recovery, post-dispatch
      unknown, reboot-required, and partial inventory cases are covered; the
      five execution terminals contain no `partial`, and no Software flow
      triggers UAC.
- [ ] AC10 (R9): Optimize exposes only the approved closed catalogue; dry-run and
      execute resolve identical fixed operations; audit-lock, stale preview,
      cancellation, timeout, unknown-after-dispatch, and unsupported OS cases
      fail closed before later operations, and native validation observes no UAC
      or administrator child process.
- [ ] AC11 (R10): Status JSON/NDJSON and desktop IPC preserve metric availability
      states and the exact V1 primitive fields/units/time bases, bounded
      intervals/process counts and truncation metadata, event sequence,
      single-flight cancellation, broken-pipe internal terminal/join behavior,
      resource-budget evidence, and no background work after leaving the mode.
- [ ] AC12 (R11): CLI definition, help snapshots, command/flag conflicts,
      exit-code classes, TTY/non-TTY behavior, JSON/NDJSON fixtures, migration
      table, English/Chinese reference docs, and removal of every old command
      shape, including the deliberate non-conversion of `clean --audit-log`, are
      tested; no execution path weakens plan/digest confirmation.
- [ ] AC13 (R12): English and Simplified Chinese catalogue completeness,
      deterministic locale precedence/fallback, persisted TUI/Desktop choice,
      localized CLI human snapshots, injection-safe interpolation, English/
      Chinese plural selection, binary byte/unit formatting, locale-specific
      accelerator definitions/collision checks, non-destructive truncation,
      keyboard and scaling layouts, and locale-invariant JSON/NDJSON/error/audit
      fixtures are tested across all five modes.
- [ ] AC14 (R13): The final resource record contains the frozen host/build
      manifest, fixtures, warm-up and five samples per workload, raw samples,
      computed statistics, every numeric threshold, and an unambiguous pass/fail
      result; all thresholds pass before release readiness is claimed.

## Out of Scope for the Planning Baseline

- Copying or porting Mole GPL code, tests, rule data, copy, brand, or proprietary
  Mac app assets.
- Pixel-identical recreation of the supplied screenshots.
- macOS Touch ID, Homebrew/self-update/remove flows, AppleScript/Finder behavior,
  or macOS process/thermal/battery probes.
- Restoring permanent deletion, weakening plan validation, or making inventory
  output executable.
- Publishing, pushing, adding any dependency outside the separately approved
  Windows-only `windows = =0.56.0` declaration and its five named features, or
  resuming product-code implementation before a new final plan is approved.

## Resolved Product Decision

The parent covers a full five-mode Windows program: Clean, Software, Optimize,
Analyze, and Status. Delivery remains staged through independently testable child
tasks, but all five modes belong to this parent and must have real contracts.

The entire program remains standard-user only. It does not request UAC, elevate
the Tauri webview, ship a privileged helper, uninstall any MSI
products, inspect other users' packages, or execute administrator maintenance.

The CLI is an immediate breaking redesign. It does not retain old top-level
commands as aliases or warnings. A new mode-oriented command tree, versioned
schemas, and explicit migration documentation replace them while keeping the
security invariants.

Runtime CLI human output, TUI, and desktop UI are bilingual in English and
Simplified Chinese. They share one catalogue and locale-selection contract;
machine-readable interfaces and audit identity stay language-neutral.

The user separately approved the exact Windows-target direct declaration
`windows = =0.56.0` with only `ApplicationModel`, `Foundation`,
`Foundation_Collections`, `Management_Deployment`, and
`Win32_System_WinRT`. This approval does not cover another version, feature,
package, frontend dependency, or alternative MSIX binding, and implementation
still waits for approval of the final planning summary.

The visual direction is structural depth, not atmospheric imitation. DevSweep
adapts the reference navigation, information hierarchy, dense row/detail
patterns, progressive disclosure, stable bottom actions, treemap browsing, and
staged feedback. It uses an original restrained dark Windows workbench, original
sweep/orbit motifs and generated icon, and does not recreate photographic
planets, Mole's per-page palette, traffic lights, brand, or pixel geometry.

The user separately approved a bounded native-evidence protocol. Clean may move
only task-owned disposable temporary fixtures to the Windows Recycle Bin and
never permanently delete them. Optimize may run only the fixed
`ipconfig.exe /flushdns` action and open the three frozen Settings URIs without
changing a setting. Software may uninstall no package until its exact disposable
current-user MSIX identity is displayed and the user confirms that target at the
task-stage checkpoint; the protocol authorizes no package installation or
signing. Windows scaling changes remain user-operated at 100/125/150/200%; the
agent runs the application and records evidence but does not change display
settings. A missing target, fixture, manual scaling observation, or direct native
evidence remains `UNVERIFIED` and pauses the programme.
