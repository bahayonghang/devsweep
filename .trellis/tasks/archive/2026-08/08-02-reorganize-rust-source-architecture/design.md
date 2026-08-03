# Rust Source Architecture Design

## Design Goal

Reorganize `devsweep` into deep Rust modules whose interfaces express the
product's safety contracts while their private implementations absorb platform,
filesystem, scanning, validation, execution, and TUI complexity.

The crate is binary-oriented. The final Rust interface is intentionally small
and may break the current accidental `devsweep::<internal_module>` paths. CLI,
TUI, JSON, audit, safety, and supported-platform behavior remain compatible.

## Design Principles

1. Organize by ownership and behavior, not by file length alone.
2. Keep dependency direction acyclic and make `model` a dependency root.
3. Keep one authoritative owner for every rule, trusted action, serialized
   contract, path-safety decision, and side-effect transition.
4. Expose internal seams only when production and test adapters both use them.
5. Keep tests at the same interface used by callers. Do not preserve tests that
   only assert a retired file layout or private forwarding function.
6. Use `pub(crate)` and private items by default. `pub` is reserved for the
   deliberate crate entrypoint.
7. Preserve fail-closed behavior whenever a move could change safety semantics.

## Target Module Tree

Exact helper filenames may be coalesced when a proposed file would be only a
pass-through. The ownership map and dependency direction are normative.

```text
src/
├── main.rs                       # calls devsweep::run() only
├── lib.rs                        # private module wiring + pub fn run()
├── application/
│   ├── mod.rs                    # parse, initialize, dispatch
│   ├── cli.rs                    # clap shapes only
│   └── commands.rs               # command handlers, input decoding, CLI output
├── model/
│   ├── mod.rs                    # crate-private model interface
│   ├── plan.rs                   # plan, target, intent, action, risk, evidence
│   └── scan.rs                   # report, health, diagnostics, totals, warnings
├── plan/
│   ├── mod.rs                    # UntrustedPlan -> ValidatedPlan interface
│   └── digest.rs                 # private canonical manifest/digest
├── rules/
│   ├── mod.rs                    # rule lookup/catalogue/resolve interface
│   ├── definitions.rs            # all rule tables and documentation metadata
│   └── registry.rs               # trusted intent -> action reconstruction
├── cargo_metadata.rs             # neutral Cargo metadata probe and scope
├── filesystem/
│   ├── mod.rs                    # narrow crate-private filesystem interface
│   ├── identity.rs               # lexical and live file identity
│   ├── reparse.rs                # no-follow symlink/reparse probing
│   ├── sizing.rs                 # bounded/cancelable size walk
│   └── containment.rs            # path containment/current-exe helpers
├── process/
│   ├── mod.rs                    # ProcessRunner request/result interface
│   ├── cancel.rs                 # cancellation observers/tokens
│   ├── capture.rs                # bounded output and sanitization
│   └── tree.rs                   # platform process-tree ownership/termination
├── scan/
│   ├── mod.rs                    # Sweeper, options, progress, merge/rank pipeline
│   ├── ranking.rs                # pure plan ranking/default-selection pass
│   ├── project/
│   │   ├── mod.rs                # ProjectScanner interface and traversal owner
│   │   ├── cargo.rs              # Rust project/workspace discovery cache
│   │   ├── dedupe.rs             # root/target normalization and dedupe
│   │   └── rescan.rs             # reviewed target size refresh
│   └── global/
│       ├── mod.rs                # GlobalProviderScanner interface
│       ├── probe.rs              # environment/tool probing adapter
│       └── targets.rs            # command-backed and known-cache targets
├── inventory/
│   ├── mod.rs                    # read-only inventory interface/report
│   └── pnpm.rs                   # bounded pnpm reference inspection
├── execution/
│   ├── mod.rs                    # Executor interface and orchestration
│   ├── command.rs                # command/trash adapters
│   ├── audit.rs                  # durable JSONL journal and replay
│   └── safety/
│       ├── mod.rs                # SafetyPolicy authorization funnel
│       └── protections.rs        # persistent user protection list
├── tui/
│   ├── mod.rs                    # TUI composition
│   ├── terminal.rs               # raw mode/alternate screen RAII
│   ├── display.rs                # pure formatting shared by state and render
│   ├── runtime/
│   │   ├── mod.rs                # event loop/effect dispatch
│   │   ├── services.rs           # production and fake adapters
│   │   └── workers.rs            # scan/inventory/clean workers
│   ├── app/
│   │   ├── mod.rs                # single App state owner and root reducer
│   │   ├── events.rs             # UiEvent/Effect/WorkerEvent protocol
│   │   ├── input.rs              # keyboard/overlay transitions
│   │   ├── selection.rs          # target/inventory/filter projections
│   │   ├── worker.rs             # staged worker-result transitions
│   │   └── jobs.rs               # jobs, logs, confirmation, progress
│   ├── render/
│   │   ├── mod.rs                # render_app and root layout
│   │   ├── theme.rs              # styles and palette
│   │   ├── targets.rs            # dashboard, target list, details
│   │   ├── inventory.rs          # read-only inventory view
│   │   ├── jobs.rs               # jobs/logs view
│   │   ├── overlays.rs           # confirmation/progress/help modals
│   │   └── format.rs             # display sanitization/width/path helpers
│   └── test_support.rs           # cfg(test)-only shared fixtures
├── config.rs                     # tracing initialization
└── bin/process_fixture.rs        # ProcessRunner dynamic fixture
```

## Dependency Direction

```text
model                    process              filesystem
  │                         │                      │
  ├──────────┬──────────────┴──────────┬───────────┘
  ▼          ▼                         ▼
rules   cargo_metadata              inventory
  │          │
  ├──────┬───┴─────────────┐
  ▼      ▼                 ▼
plan    scan            execution
  └──────┴──────────┬──────┘
                   ▼
             application / tui
                   │
                   ▼
                 lib::run
```

`model` imports only standard-library and serialization concerns. `process`
and `filesystem` do not import scan, plan, execution, application, or TUI.
`rules` never imports scanner/provider implementations. `cargo_metadata` is a
neutral consumer of `process`, shared by scan and execution safety. Any source
dependency that points upward in this diagram requires an explicit design
review.

## Module Interfaces

### Crate entrypoint

The final external Rust interface is:

```rust
pub fn run() -> anyhow::Result<()>;
```

`main` delegates to it. CLI parsing, tracing initialization, command dispatch,
TUI launch, plan-file decoding, and output formatting remain private. No
compatibility re-exports are created for the current public modules.

### Model and plan trust

`model` owns serialized DTOs and internal typed cleanup values. It does not
know how a process is run, how a path is inspected, or how an action executes.

`plan` exposes validation and opaque validated-plan access needed by execution
and TUI confirmation. Canonical digesting and rule registry details stay
private. `rules` owns every rule ID, facts, catalogue entry, and trusted action
template, eliminating scanner/provider documentation back-references.

### Filesystem and process

`filesystem` hides platform-specific path identity and reparse implementation
behind the existing fail-closed operations used by scan, inventory, plan, and
execution. `process` hides capture threads and Windows Job Object/Unix process
group details behind `ProcessRunner`.

Cancellation and adapter traits remain only where real production and test
adapters exist. Platform-specific unsafe blocks stay local to the implementation
that documents their invariants.

### Scan and inventory

`scan` owns project/global sequencing, cumulative progress, merging, ranking,
dedupe, and reviewed rescan. Its external seam remains a small `Sweeper`
interface plus scan options/progress values. Project and global scanners are
private adapters behind that seam except where their focused tests need an
internal seam.

`inventory` stays separate because its report is read-only and must never gain
cleanup authority. It may reuse filesystem/process implementation but cannot
construct a cleanup plan or call execution.

### Execution

`execution` exposes `Executor` and typed request/report/progress values needed
by application and TUI. Authorization, live revalidation, command/trash
adapters, audit journal durability, and replay are private implementation.
Every side effect still requires `SafetyPolicy::authorize` to produce an
authorized action immediately before dispatch.

The protection-list interface hides config-path selection, canonicalization,
atomic persistence, and versioning. The application command handler asks it to
add, remove, or list paths; it does not own persistence rules.

### TUI

`App` remains the single state owner. Splitting its implementation does not
create competing state stores. `UiEvent -> App::update -> Vec<Effect>` remains
the reducer interface, and the runtime remains the only owner of threads,
channels, and effect dispatch.

Pure display formatting shared by reducer-owned logs/confirmation and ratatui
views lives in a neutral TUI sibling module. App code never imports render
implementation.

`render_app` remains the root render interface. View files accept immutable
state and never scan, execute, mutate `App`, or perform expensive I/O. Shared
format helpers normalize display text only and never write transformed paths
back into domain state.

## Data Flows

### Scan

```text
CLI/TUI -> ScanOptions -> Sweeper
       -> project/global adapters -> observed CleanTarget values
       -> merge + single ranking pass -> CleanupPlan
       -> untrusted projection + ScanHealth -> ScanReport / TUI snapshot
```

### Clean

```text
saved JSON -> UntrustedPlan -> plan validation + trusted rule resolution
           -> opaque ValidatedPlan + digest
           -> explicit selected TargetIds
           -> live SafetyPolicy authorization
           -> durable started audit
           -> command/trash adapter
           -> terminal audit + progress/report
```

### TUI

```text
keyboard/worker event -> App::update -> typed Effects
                     -> runtime worker adapters
                     -> typed WorkerEvents -> App::update
immutable App state  -> render_app -> terminal frame
```

## Compatibility and Migration

- Preserve all Clap command names, flags, defaults, help safety guarantees, and
  stdout/stderr separation.
- Preserve cleanup plan version 2, scan report version 1, inventory report
  version 1, field names, validation rules, and canonical digest behavior.
- Preserve audit JSONL event semantics and durability ordering.
- Preserve TUI keys, views, selection defaults, confirmation freezing,
  single-flight jobs, cancellation, tombstones, diagnostics, and narrow-layout
  behavior.
- Preserve Windows and Unix process-tree termination and no-follow path safety.
- Rust module paths may break. The release notes do not need a compatibility
  layer because the user selected a binary-oriented interface.

Migration proceeds child by child. Every child must compile and pass `just ci`
before the next child starts. Temporary crate-private re-exports are allowed
only when needed to keep an intermediate child buildable and must be removed by
the final child.

## Task Ownership and Ordering

1. `08-03-untangle-core-contracts-rules` establishes the acyclic core and rule
   ownership.
2. `08-03-modularize-backend-runtime-discovery` depends on child 1 and moves
   backend implementations behind the new interfaces.
3. `08-03-decompose-tui-internals` depends on child 2 so TUI imports move only
   after backend interfaces stabilize.
4. `08-03-narrow-entrypoint-architecture-docs` depends on children 1-3 and
   removes remaining public/internal compatibility exposure, updates docs, and
   performs final integration validation.

The parent owns requirements and final cross-child acceptance. It has no direct
product-code implementation.

## Tradeoffs and Rejected Alternatives

- Rejected: split every large file mechanically. This creates shallow modules
  and increases navigation without reducing interface complexity.
- Rejected: preserve all current public module paths. The user explicitly chose
  a smaller Rust interface, and compatibility re-exports would retain accidental
  architecture.
- Rejected: introduce generic repository/service layers. There is no database,
  remote service, or second adapter that justifies those seams.
- Rejected: place Cargo metadata back under scanner or safety. Both need it, so
  either owner recreates the current reversed dependency.
- Rejected: create multiple TUI state objects per view. This would weaken the
  reducer invariant and complicate cross-view job/selection behavior.
- Accepted: more module files and crate-private re-exports. The cost is extra
  navigation, paid back by acyclic ownership, localized unsafe code, and
  focused interfaces.

## Rollback

Each child is independently committed after its full gate. If a move changes
behavior or makes ownership less clear, revert that child's work commit rather
than layering compatibility wrappers on top. Task metadata/archive commits are
separate from work commits under the Trellis finish workflow.
