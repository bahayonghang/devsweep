# Repository audit and asset inventory

Inspected 2026-10-15 at `dev / 25437d102ab7355198b3ef2647c2cbced740b3b2`.
This is source/document inspection, not a new product test or benchmark.

## Current capabilities and seams

| Area | Current evidence | Consequence for rewrite |
| --- | --- | --- |
| CLI roots | crates/devsweep-cli/src/application/cli.rs:37; SoftwareCommand:177, AnalyzeCommand:241, StatusCommand:256 | Extend existing command families; preserve existing grammar and TUI entry. |
| Shared Clean services | crates/devsweep-core/src/services.rs:43 and CleanModeAdapter in that file | Reuse the trusted scan/execution boundary; do not build a second engine. |
| Desktop-only routes | desktop/src-tauri/src/lib.rs:29 and software.rs:361,383,390,405,433 | Existing update checks/startup/leftover APIs need CLI exposure. |
| Software identity | crates/devsweep-core/src/software/model.rs:11,53 | ARP/MSI/current-user MSIX inventory does not imply execution support for all sources. |
| Uninstall | crates/devsweep-core/src/software/execution/mod.rs:354; CLI docs at cli.rs:184 | Current actual execution is current-user MSIX. Win32/MSI/Scoop are extension work. |
| Updates | crates/devsweep-core/src/software/updates.rs:80 | Read-only winget check; installing offered updates is new authority. |
| Startup | crates/devsweep-core/src/software/startup.rs:272,278 | Current-user StartupApproved changes exist with explicit bool confirmation; machine entries read-only. New CLI strict plan wrapper must be designed. |
| Leftovers | crates/devsweep-core/src/software/leftovers.rs:385,400,416 | Discovery, selection-plan preview and recycle already exist; reuse ownership rules. |
| Optimize | crates/devsweep-core/src/optimize/catalogue.rs:13-27 | DNS flush, three Settings links, four guidance entries; not eight implemented repairs. |
| Analyze | crates/devsweep-core/src/analysis/mod.rs:9; actions.rs:203,211,221,236 | Bounded walker and reveal/recycle exist; CLI has only scan. |
| Status | crates/devsweep-core/src/status/mod.rs:179,335,369 | Native snapshots and process list exist. VRAM/fan/SMART/physical disk activity are explicitly unsupported in V1; GPU/thermal vary by probe availability. |
| Windows caches | crates/devsweep-core/src/rules/windows_caches.rs:19; rules/definitions.rs:5 | Six families: Chrome, Edge, Firefox, VS Code, Cursor, stale user temp. Existing developer providers and artifact rules are separate and remain valuable. |
| Runtime | .trellis/spec/backend/bounded-delivery.md and runtime-publication.md | Preserve lossless Status vs replaceable scan observations and operation-specific teardown. |
| Build | Cargo.toml:1-12 | Core/CLI/Tauri workspace, Rust 1.88.0, edition 2024. GPUI must not silently raise CLI MSRV. |

Line anchors describe this checkout and should be refreshed before code changes.

## Reusable assets and behavior

| Source | Reuse | Boundary |
| --- | --- | --- |
| desktop/src/assets/devsweep-icon-master.png | Original compact beaver master | Canonical application identity; do not replace with Mole animal. |
| desktop/branding/devsweep-beaver-full.png and prompts/ | Original full mark and provenance | Keep original/derived linkage. |
| desktop/src-tauri/icons/icon.ico and PNG family | Window/taskbar/tray/package inputs | Verify alpha and native small-size rendering again in GPUI. |
| desktop/scripts/prepare-brand-icons.mjs | Existing deterministic export recipe | Uses Node 22/Tauri CLI 2.11.4 at generation time. New GPUI runtime/build must not depend on a running Tauri frontend; reuse exports or replace export tooling later. |
| desktop/public/earth/still.png and natural-earth.png | Optional still/local texture | Natural Earth license/hashes in THIRD_PARTY_NOTICES.txt; no network runtime. |
| desktop/public/THIRD_PARTY_NOTICES.txt | Natural Earth and palette attribution | Preserve in new bundle. |
| desktop/src/stage/ and shared semantic icons | Behavior/visual reference | Do not port TS/WebGL implementations into a supposedly Rust-only UI. |
| resources/i18n and shared stores | Language, units, preferences/history | Reuse authoritative contracts; isolate any needed presentation-code extraction. |

User screenshots are saved only as design-reference evidence under
`reference-images/`; none is a distributable application asset.

## Recent workflow decisions worth retaining

The current 10-14 Clean workbench separates global/project scans, retains reports
through Settings/navigation, uses an original animal identity and removes a
mandatory large Earth. See archived tasks
`10-14-desktop-cleanup-workbench/prd.md` and
`10-14-devsweep-animal-identity/prd.md`. Their native screenshot acceptance has
unchecked/NOT_RUN entries even though they are archived. Do not treat archive state
as proof of native acceptance.

The reference screenshots support five-mode workflow design. They do not require
reversing the useful two-scope Clean workflow or reintroducing a costly hero.

## Historical performance evidence

The checked-in
`.trellis/tasks/archive/2026-10/10-13-cli-tauri-runtime-performance/evidence/c4-final-measurement.md`
records equal-envelope fixture improvements for Clean. It explicitly says the
formal Software series was not repeated: C3 p95 **5211.467 ms** still exceeded its
**5000 ms** limit. This is historical scoped evidence, not a fresh measurement of
the current checkout. New comparable baseline and candidate measurements are
required; no claim that GPUI already fixes it follows from this audit.
