# Cleanup home: global caches, project artifacts, and original identity

## Goal and user value

Let a developer answer two separate questions immediately: which tool-owned global caches can be reviewed for cleanup, and which selected projects have large rebuildable artifacts worth cleaning on demand. Deliver a clear desktop workbench and an original small-animal identity.

## Authorization and status

The initial request authorized deep analysis and a Trellis parent/child tree. Planning documents, research, a catalogue-derived visual specification, and a generated mascot concept were delivered. On 2026-10-14 the user explicitly approved implementation in the planned task order. Product work and verification are now authorized; real user-cache cleanup, commit, archive, installer replacement, and publication remain outside this authorization.
The user subsequently confirmed a configurable **5 GiB** per-project warning, evaluated only after user-initiated scans.

## Confirmed pre-edit findings

- Core/CLI already support independent project/global scopes. The desktop defaults both flags on with no roots and disables its single action: desktop/src/modes/clean/useScanScope.ts:8; desktop/src/pages/ScanPage.tsx:104; desktop/src-tauri/src/commands.rs:185.
- uv.cache.clean already exists with live provider authorization and explicit selection: crates/devsweep-core/src/rules/definitions.rs:242. Cargo home is inspect-only. Docker is outside current behavior.
- Project ownership and target kinds are backend facts. Current completed results group by kind/scope rather than project: desktop/src/modes/clean/impact.ts:55.
- mbx has no current integration. Installed help distinguishes workspace-managed output/private state from shared objects. Its per-workspace executable authority still needs qualification; see research/scan-contracts.md.
- The path input misses the current CSS selector because it lacks explicit type=text: desktop/src/modes/clean/ScanRoots.tsx:18; desktop/src/styles.css:91.
- Screenshot font settings are unverified. Clean route retention has a source-level discrepancy requiring reproduction: desktop/src/app-shell/AppShell.tsx:474; desktop/src/modes/clean/CleanWorkbench.tsx:51.
- Current checkout is dev at e69a4ee. The only pre-existing tracked modification is desktop/src-tauri/Cargo.toml. Previous cleanup/Earth/performance tasks are archived, not a current verification result.

## Requirements

| ID | Observable requirement | Owner |
| --- | --- | --- |
| R1 | Global caches and project artifacts have independent scan actions, results, status, and readiness. No project folder is needed for global scans. | A, D |
| R2 | Show existing global provider capabilities and allow supported items through the same DevSweep plan/preview/confirm workflow available in the CLI. Missing, unsupported, empty, failed, and inspect-only states are distinct. | A, D |
| R3 | Inspect only explicitly selected project roots. Group Node builds, Cargo outputs, and mbx workspace-owned artifacts by verified project identity; show dependencies separately. Preserve existing safe rule coverage. | B, D |
| R4 | Project cleanup is opt-in. Warn at aggregate >=5,368,709,120 bytes with a configurable threshold. Warning does not select or clean anything. | A, C, D |
| R5 | Do not count overlapping/shared storage twice. Unknown capacity stays unknown; partial measurements retain lower-bound qualification. Occupancy is not a promise of freed space. | B, C, D |
| R6 | Apply the chosen style:shadcn utility language, two informative scope sections, native CJK typography fallback, coherent controls/icons, and no large Earth hero on Clean home. Preserve appearance choices and other modes. | D |
| R7 | Use image generation for an original small-animal logo, with a reviewed compact version for small icons and a consistent packaged asset family. Mole is a product reference. | E |
| R8 | Verify contracts, workflow safety, warnings, route retention, native visuals, accessibility, packaging and existing performance gates with current evidence. | F |

## Product decisions for review

- Two logical scopes, one shared heavy-operation coordinator; separate entry points do not request concurrent scans.
- Both sections are available on first visit; global scan is immediately available, project scan asks for explicit roots. No scan starts merely from opening the page.
- Scan results are retained per scope for the current app session and across route changes. Old results have a timestamp/stale state; no new persistent report database.
- Project build, test, and tool-cache artifacts contribute to the warning. Dependency directories and virtual environments remain visible and individually reviewable but outside this build-artifact warning subtotal. Label that boundary explicitly. Source files are never included.
- Shared mbx action-store occupancy is shown separately and excluded from per-project warning totals. Workspace output is counted once even if Cargo and mbx both discover it.
- Project targets start unselected through the shared policy; the warning and selection are independent. Existing global provider risk/default-selection rules remain unchanged.
- Alerts are inline after scans. No scheduler, background watcher, OS notification service, auto-clean, percentage-of-disk policy, or new cache-prune mode.
- The selected design and original beaver were approved with the implementation package and have been implemented; native consumer acceptance remains separately tracked.

## Acceptance criteria

- [x] AC1: From an empty root store, global action sends global=true/projects=false/roots=[] and returns provider coverage; project action cannot traverse an implicit CWD/home/drive.
- [x] AC2: Project action sends projects=true/global=false with validated roots; independent results survive the other scope's empty/error/cancel response and route/Settings return.
- [ ] AC3: Each existing supported global item reaches CLI and native preview through the same trusted registry; unsupported or inspect-only items cannot be selected by row, select-all, or reducer.
- [ ] AC4: Node/Rust ownership evidence, external targets, nested roots and Cargo/mbx duplicate footprints are verified with isolated fixtures. A shared cache never belongs to every project.
- [ ] AC5: A supported mbx workspace-clean action is implemented only with proven stable ownership and live footprint binding. If that cannot be established, B stays incomplete for executable cleanup and the parent cannot claim the requested mbx cleanup is delivered.
- [x] AC6: Threshold tests cover T-1/T/T+1, sum of smaller outputs, overlap, partial-above/below, unknown, changed setting and corrupt/future stores. Oversized project candidates remain unselected.
- [ ] AC7: Both scoped cleanup flows retain current plan, preview digest, selection invalidation, second confirmation and outcome semantics; no frontend-created authority.
- [ ] AC8: Compare fixture screenshots and actual native app at required sizes/locales/themes. Verify the input style, current/saved fonts, focus, long paths, reduced motion and high contrast.
- [ ] AC9: Original mascot alpha/edges, compact-size silhouettes, all consumed bundle/tray/webview assets and provenance pass review.
- [ ] AC10: Desktop gates and just ci pass on the implemented revision; current native and full-report performance evidence is recorded. Missing evidence is NOT_RUN/UNVERIFIED.

## Scope exclusions

No real user-cache cleanup in research/verification; no uninstall/optimization execution; no broad machine scan; no whole-shell/five-mode redesign; no manual deletion of global cache internals; no automatic traversal of mbx links; no new runtime UI dependency; no commits, archive, signing, publishing or installation implied by planning.

## Task map and review state

See task-map.md for six children and explicit dependencies. Research is persisted under research/. Technical contracts and implementation sequence are in design.md and implement.md. The catalogue-derived design is design-system/DESIGN.md; logo prompt/assets are under assets/.
Blocking product questions: none. Technical gate: mbx authority qualification in B. Implementation was approved on 2026-10-14 and begins with the parent-owned baseline preflight.

