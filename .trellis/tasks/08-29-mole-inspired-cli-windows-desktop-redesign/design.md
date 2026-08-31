# Design - Five-Mode Windows Product Programme

## 1. Clean-room design boundary

Mole commit `f92133a4d6277574177e0b1284742072fb3b5bdc` and the five supplied
screenshots are reference evidence only. Implementation begins from DevSweep's
existing domain contracts. No Mole source, fixtures, rules, strings, layouts,
logo, or planetary assets enter production or tests. Every reference decision
is recorded as accept, adapt, or reject in `research/mole-reference-audit.md`.

## 2. Product architecture

```text
CLI / TUI / Tauri desktop
          |
          +-- shared locale and presentation DTO boundary
          |
          +-- Clean ------ CleanupPlan / preview digest / executor
          +-- Software --- SoftwarePlan / uninstall digest / executor
          +-- Optimize --- MaintenancePlan / operation digest / executor
          +-- Analyze ---- read-only analysis snapshot
          +-- Status ----- read-only metric snapshot/live stream
          |
          +-- Protection / Rules / History supporting services
```

The five modes share presentation primitives and a single operation coordinator,
but never share executable plan types or authorizers. `Clean`, `Software`, and
`Optimize` each use a domain-specific untrusted selection, live revalidation,
preview digest, explicit confirmation, executor, and audit representation.
`Analyze` and `Status` are read-only and can never produce executable actions.

The coordinator permits one heavy or mutating workflow at a time. Scan,
Analyze, Software inventory/execution, Optimize execution, and live Status are
mutually exclusive. Mode changes request cancellation and join owned work before
starting another workflow. A skipped Status tick never starts parallel work.

## 3. Public command direction

No subcommand opens the interactive TUI. The new roots are `clean`, `software`,
`optimize`, `analyze`, `status`, and read-only `history`; bare `devsweep` opens
the TUI. The old roots are removed in one breaking change. The CLI child freezes
the exact grammar and versioned schemas before downstream modes implement it.
Human text is bilingual; machine fields, enum values, error codes, digests, and
audit identities are invariant.

## 4. Original Windows visual direction

The approved direction is structural depth:

- centered five-mode navigation adapted to native Windows title-bar and window
  controls;
- dense sortable/grouped workbench rows with progressive detail disclosure;
- fixed or sticky selection/action summary that never hides safety state;
- read-only Analyze hierarchy with list plus zero-dependency treemap;
- explicit preparing/running/canceling/partial/completed/failed stages;
- original deep gray-green tokens, restrained mode accents, Segoe UI Variable
  and system fallbacks, visible focus, keyboard parity, reduced motion;
- original generated DevSweep icon and optional CSS-native sweep/orbit motifs,
  but no photographic planet hero, fake macOS chrome, glass, or copied geometry.

The gray-green palette and optional motifs remain gated on the desktop-shell
task's R6/AC5 spec-first update. That update must replace the conflicting surface
palette clause, distinguish non-informational CSS-native motion from prohibited
hero/illustration treatment, and preserve reduced-motion and Clean safety rules
before any mode consumes the visual direction.

Desktop widths must cover 390, 800, 1024, and 1440 CSS pixels without hiding
authority, availability, or confirmation information. Native evidence covers
100%, 125%, 150%, and 200% Windows scaling in both languages.

## 5. Delivery graph and gates

1. Freeze the CLI/localization contract and finish the bounded-sizing/icon
   foundation checkpoints.
2. Build the shared desktop/TUI shell and operation coordinator.
3. Deliver Clean and Analyze.
4. Deliver Software inventory/plan before its executor, then its presentation.
5. Deliver Optimize catalogue/executor before its presentation.
6. Deliver Status collector/CLI before its live presentation.
7. Deliver Protection/Rules/History only after the Clean, Software, and Optimize
   audit schemas exist.
8. Run final cross-mode native integration, documentation, resource, safety, and
   bilingual evidence gates.

Within each domain, core contracts precede IPC and presentation. Parent tasks
with children are coordination gates and do not duplicate child file ownership.
The final integration child may repair cross-mode glue but must return
domain-specific defects to their owner.

## 6. Dependency policy

The baseline uses the existing Rust/TypeScript stack, expands `windows-sys`
features only for named Win32 APIs, and implements treemap and message catalogues
without a new frontend runtime dependency. Current-user MSIX uses a direct,
Windows-only `windows = =0.56.0` dependency with only `ApplicationModel`,
`Foundation`, `Foundation_Collections`, `Management_Deployment`, and
`Win32_System_WinRT`. This exact version is already locked transitively through
`trash`; the direct declaration adds WinRT projection features but no second
`windows` version. It is Microsoft `MIT OR Apache-2.0`, declares Rust 1.62, and
its cached crate archive is 10,807,828 bytes; the project remains Rust 1.88.
The Software child owns the direct declaration, feature proof, release-size
delta, dedicated MTA worker, current-user SID enumeration, and
`RemovePackageAsync` adapter. PowerShell, winget, vendor strings, generated raw
COM bindings, and silently dropping MSIX are rejected alternatives. The package
was separately approved for this exact Windows-target version/feature set during
planning. No other dependency is authorized, and adding the declaration still
waits for approval of the final planning summary and the Software leaf start.

## 7. Evidence and rollback

Every child records focused automated gates and direct native evidence. Native
Windows/UAC/antivirus/layout/resource observations cannot be replaced by unit
tests. Failed children roll back within their domain; no partial mode is exposed
in navigation. Final resource acceptance uses the numeric protocol owned by
`08-29-five-mode-native-integration`; qualitative "no material regression" is
not a gate. Native side effects use only the separately approved bounded
evidence protocol: task-owned temporary trash fixtures, exact-target-confirmed
disposable current-user MSIX removal, fixed DNS flush, fixed Settings-page launch
without settings changes, and user-operated display scaling. The parent remains
planning until recursive Trellis validation and an independent plan audit pass.
It is not implementation authority.
