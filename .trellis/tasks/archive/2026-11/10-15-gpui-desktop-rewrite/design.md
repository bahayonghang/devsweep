# Architecture and migration proposal

Status: approved Windows x64-first design, 2026-10-15.

Platform boundary: the first native desktop targets Windows x64. Preserve existing
Windows/Linux/macOS CLI behavior and CI; isolate new Windows APIs behind explicit
capability contracts. Other native desktop targets are later milestones.

## Dependency direction

```text
CLI commands ----------\
TUI adapters -----------+--> shared devsweep-core domain APIs
GPUI Rust views --------/       | observation / plan / live preview
   | native coordinator         | authorize / audit / dispatch / read-back
   | presentation only          +--> OS APIs / finite installed-provider adapters
   +--> native window, dialogs, tray, accessibility
```

GPUI links Rust domain APIs directly. It does not shell out to DevSweep and parse
its text, embed the old web frontend, or duplicate scanners. CLI-first describes
capability ownership and delivery order, not a mandatory subprocess transport.
Keep the existing core crate unless a proven dependency boundary needs extraction.

## Runtime ownership

Blocking filesystem, registry, provider and inventory work stays off the UI thread.
Reuse the current cancellation and process ownership abstractions. Each domain
operation has an identity, phase and terminal result; UI generations reject stale
completion before changing accepted reports or enabling actions.

Use bounded delivery. Replaceable scan progress may be coalesced within one
operation/phase; lifecycle and final events remain ordered. The existing Status
CLI NDJSON stream remains lossless with its 16 ordinary + 2 reserved-record bound.
UI view updates may use latest projections after consuming records; they cannot
alter the public stream. Virtualize rows and compute expensive projections once
per relevant snapshot rather than per frame.

A shared operation coordinator serializes conflicting heavy/mutating work and
allows explicit read-only subscriptions without duplicate collectors. Read-only
cancellation joins. Close/navigation cannot silently cancel or kill already
dispatched uninstall/maintenance; follow that operation's exact policy.

## Authority boundary

Domain plans contain typed intents and observed identities. The core reconstructs
allowed actions from current evidence and checks the live digest. UI selection,
saved JSON, registry strings and provider output are untrusted observations.

C02 supplies only the concrete elevated operations required by approved catalogues.
A native helper is short-lived, caller-bound and operation-bound, with no arbitrary
command endpoint or persistent system service. Validate again after privilege
handoff. Durable audit precedes side effects. UAC decline and interrupted/unknown
outcomes are not success and never cause an automatic retry.

No permanent file deletion is introduced. Native uninstall and explicit OS-owned
maintenance have their own irreversible/reboot disclosures; Recycle Bin outcomes
do not assert physically freed space. Registry changes require finite ownership,
backup and conditional restore. A generic undo promise is forbidden.

## Build and dependency policy

Proposed new source location: `desktop-gpui/`, with native binary
`devsweep-gpui` during coexistence. C09 owns the final workspace/package arrangement.
The inspected core/CLI MSRV is 1.88.0. The user now prioritizes latest compatible
libraries, including GPUI. C16 evaluates a current stable toolchain and any declared
MSRV migration; update CI/documentation together and verify compatibility. Choose
shared or separate native workspace layout from evidence, not an arbitrary old
compiler constraint.

Prefer latest compatible stable releases and a coherent latest GPUI/platform
revision proven by C09 when needed; pin the evaluated result reproducibly. C16
records all direct-library latest/selected/source/date/exception decisions.
Do not mix main-branch README APIs with a same-numbered crates.io release.
Transitive native libraries/Windows SDK are compatible with Rust application code;
the goal does not require every OS component or external package manager to be Rust.
No new default production dependencies are added in this planning turn.

## Presentation design

Amended 2026-10-16 from `research/mole-014-rea.md` and the Clean screenshot.
The Tauri PureMac sidebar remains the current `desktop/` shell. This section
is only the GPUI shell.

Visual thesis: a dark navy field, a light selected pill, and one licensed Earth
still. The selected control is a solid pill. Do not use gradient text, glass
cards, or the reference poem, planet textures, or silhouette.

- Window: custom title bar, caption buttons at the top right, no document title
  in the content.
- Navigation: one horizontally centered capsule. Beaver button, then five mode
  tabs in this order: Clean, Software, Optimize, Analyze, Status. The selected
  tab is a light pill. The beaver menu opens Settings and the existing
  supporting destinations. Keep Screen On and Clean Screen stay C13 sessions.
  Do not add license, activation, or a Mole self-update flow.
- Clean states: Landing, Scanning, Found, AllClean, Cleaning, Completed, Failed.
  Landing centers the Earth still, one existing DevSweep sentence, and one
  primary button. That button opens the global-cache / project-artifact choice
  and does not start a scan by itself. Found is the review. Cleaning and
  Completed show per-group results. Recycle is the only clean execution path.
- Software states use three segments: Uninstall, Updates, Startup. Leftover
  review is a later step, not a fourth segment.
- Optimize states: Landing, Running, Completed, Failed. Landing opens grouped
  tasks (startup, network and search, maintenance). Each task shows precheck,
  privilege, skip reason, and last result. The run button creates a review for
  the selected operation ids. Admin request is a visible activity, not a silent
  elevation.
- Analyze states: Landing, Scanning, Detail, Failed. Landing chooses a drive or
  root. Detail keeps treemap and accessible list together.
- Status states: Landing, then Live. Live shows capability-aware cards and a
  process table. Missing sensors stay unavailable.
- Optional tray panel shares collectors. Screen-on and screen-cover sessions
  are visible, user initiated, and have explicit end/escape behavior.
- Use the existing beaver identity and local licensed Earth still. Reduced
  motion and idle/hidden windows paint the still once and do no recurring
  decorative work.

Keep original EN/zh-CN catalogue and preference semantics. If pure presentation
catalogue helpers currently live in CLI, extract only those needed by both consumers
rather than depending on the CLI executable or cloning strings.

## AI data and provider boundaries

C15 adds a Clean subview backed by CLI operations, not a sixth required top-level
mode. Its provider adapters separate caches, old versions, sessions, worktrees and
installed tools. Cache selection cannot acquire session/worktree authority. Opt-in
session changes require validated recovery and provider-index consistency; worktree
changes require complete ownership/Git-state/active-use proof. A provider that lacks
a safe reversible operation stays inspect-only with a tracked capability gap.

## Assets and stores

Source inventory is in research/repository-audit.md. C10 owns relocation/embedding,
hashes and all consumers. Reuse the canonical icon master and exported ICO/PNG;
maintain Natural Earth and palette notices. No screenshot or Mole asset is shipped.

Preferences and history stay domain-owned. Reads do not migrate/write; explicit save
uses existing versioned, locked atomic stores. Preserve old-version rollback data.
Concurrent Tauri/GPUI mutation is serialized by domain locks. Unsupported future
versions fail clearly rather than falling back to stale data.

## Coexistence and rollback

Keep `desktop/` buildable as a comparison and rollback until C14 accepts the new
package. Add explicit GPUI dev/build/check targets; do not silently change
`just tdev` or remove workspace members during early work. Shared core changes must
pass CLI and still-supported Tauri contracts. A later promotion switches defaults
only after native acceptance and a reviewable migration record. Retirement is a
separate user-authorized change; historical evidence remains in the repository.

## Test levels

Core/provider fixture tests prove semantics and rejection, GPUI tests prove state
and actions, native UIA/input proves platform interaction, and release process-tree
measurements prove performance/lifecycle. None substitutes for another. Native
destructive tests use disposable state/apps; fixture outcomes are never advertised
as successful real user cleanup.

## Unresolved technical gates

See research/gpui-windows-feasibility.md (C09), child C02 design (elevation),
and research/performance-protocol.md (measurement). The GPUI source layout,
pin and helper mechanism are proposals until these bounded gates produce evidence.
