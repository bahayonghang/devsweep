# Functional acceptance summary — 2026-10-13

Seven of eight functional acceptance criteria are satisfied by the evidence
below. **F-AC3 remains open** for one combined native GUI scope case. This is a
criterion-level documentation checkpoint, not task completion, release approval,
archival or authorization to clean user data. This update used existing evidence
and static reads only; no application, build or test ran during the reserved
native measurement slot.

## Implemented result and final identity

The shared scanner now carries root/provider coverage through real report/v2
plan conversion, trusted reconstruction and preview. Go and configured cache
roots round-trip, nested Python follows the nearest supported marker, and
independent provider failures preserve healthy findings. The approved generated
project outputs and narrow Windows cache/temp families have closed ownership
proofs and explicit executable or inspect-only outcomes.

Desktop scope starts with an explicit visible choice, supports persisted root
selection, and keeps scope separate from cleanup protection. CLI and desktop
retain incomplete/empty distinctions, reviewable paths and size confidence,
selection/preview invalidation, reviewed size rescan, and distinct owner-command
versus Recycle Bin outcomes. Live provider tool/configuration/footprint binding,
marker/owner/protection checks, explicit confirmation and audit remain core-owned.
Tauri calls the same core services in-process.

The final checked source is packaged as C2 native SHA-256
`5ffaefe242808e9a2074880667209a0ece6facef1b15ab5f782adf36cc2f2a59` and CLI
`a5881ebf98eed803581619db2cf0cb967093cf0151887c67b274511047701277`.
C2's changes after C1 were desktop-only, including presentation visibility and
the async worker cap; the CLI is byte-identical. Corrected B3 remains the
immutable pre-optimization reference. Earlier B/B2 prechecks and failed attempts
remain historical evidence, not substitutes for B3/C2.

## F-AC1 through F-AC8

| Criterion | Disposition | Actual evidence and boundary |
| --- | --- | --- |
| F-AC1 — existing production rule chain | **Pass** | [Rule matrix](functional-rule-matrix.md) maps existing project, command and known-global rules to real scan → persisted v2 → registry → preview tests, with recording runners. [Review regressions](functional-review-regressions.md) add tox-only/nearer-tox and malformed Windows-root cases. Go plus unrelated candidates, configured Gradle/NuGet/Go, and nested Python are included. The production pipeline does not use a test-only validated-plan constructor. |
| F-AC2 — provider parsing and typed failures | **Pass** | The scan/process suites in the [final C2 gate](../../10-13-cli-tauri-runtime-performance/evidence/independent-c2-check.md#verification) include CRLF/space/Unicode/`warning`/`error` paths, ambiguous stdout, benign stderr, nonzero/truncated output, deadlines, absent tools and Python fallback. [F5/F6 regressions](functional-review-regressions.md) preserve typed uv/Bun/Go-build outcomes and refuse unbound Yarn delegation. Invalid output does not create executable authority. |
| F-AC3 — explicit GUI roots and independent failures | **Open: combined GUI scope case** | [Packaged C2 Clean](native-clean-acceptance.md) proves unrelated CWD, two physical drives, Unicode/spaces, root deduplication, nested-selection removal, persistence and invalid/empty recovery. The later actual ACL test proves GUI denied-root plus healthy-project survival and matching project-only CLI output. A separate isolated CLI all-scope scan preserves the healthy project and a global Maven target. Those two scopes do not prove the same combined GUI observation; see the exact remaining case below. |
| F-AC4 — truthful incomplete/empty/outcome presentation | **Pass** | [C2 native reports](native-clean-acceptance.md) match CLI completely for healthy, mixed missing-root, failed zero-target and genuinely empty cases; actual denied-root parity also returns partial exit 5. [Desktop coverage/results tests](desktop-functional.md) and final generated-wire/decoder checks retain zero-target provider-failure diagnostics and all-skipped reasons. Owner-command success never implies bytes were moved or freed. Execution-result UI tests use controlled reports/fake executors. |
| F-AC5 — required provider waves and retained data | **Pass** | Every required executable family in the [rule matrix](functional-rule-matrix.md) has production scan/v2/validation/preview and recording-runner coverage; retained families have explicit inspect-only/disabled-action checks. [Provider authority](provider-authority-validation.md), [Windows production pipeline](windows-cache-pipeline.md), [native helper checks](windows-cache-validation.md), and [review regressions](functional-review-regressions.md) cover source, credential/profile/offline/editor/agent/model sentinels, protection/reparse refusal, and live drift. Activity seams and native helper proof are distinct as described below. |
| F-AC6 — useful results and stale-preview invalidation | **Pass** | [Desktop functional evidence](desktop-functional.md) records mixed command/trash, command-only, all-skipped, inspect-only and reviewed-rescan behavior. Final Clean component/reducer/scope tests cover no-default-selection recovery, size-confidence display, failed preview/result states and selection/root/protection invalidation. Core freshness and size tests cover recent-only and unknown-size observations; recording executor outcomes distinguish failures and unknown results. [F7 regressions](functional-review-regressions.md) preserve busy/unknown inspect-only reasons after rescan without granting new authority. These are core/component/adapter proofs, not native real-cleanup claims. |
| F-AC7 — authority, drift and shared adapters | **Pass** | [Provider binding tests](provider-authority-validation.md) and the [closed independent F1–F7 review](independent-check.md#follow-up-review-and-f6-correction--2026-10-13) cover same-path shim/script/interpreter/config changes, Yarn Classic sources and refusal before provider calls. Rule/executor tests cover tampering, owner/marker/protection drift and stale digests. Missing observed provider footprints require rescan at the shared binding boundary; no legacy observation gains authority. CLI/Tauri core services, closed generated wire/decoder tests and adapter tests remain shared; no shell authority or second scanner was added. Native Unix execution is not claimed. |
| F-AC8 — integrated gates and packaged Windows smoke | **Pass** | [Final C2 independent gate](../../10-13-cli-tauri-runtime-performance/evidence/independent-c2-check.md#verification): canonical `just ci` passes format/check/tests/Clippy, **744 tests**, five pre-existing ignored entries. Node **22.23.2 / npm 10.9.8** passes lint/typecheck, **438 Vitest + 5 Node tests**, production build and generated checks. The Rust gate includes exact English/Chinese CLI-reference-to-parser parity and wire fixtures. [Packaged C2 smoke](native-clean-acceptance.md) and [native command-shim fixtures](provider-authority-validation.md) pass with isolated read-only/fake-action inputs; no user-cache cleanup was used. |

The final raw quality outputs are `dist/just-ci-c2-independent-review.txt` and
`dist/frontend-c2-visibility-gate.txt`. Earlier focused counts in linked notes
identify their checkpoints and must not be summed as distinct tests or presented
as the final totals. The retained >500 kB frontend chunk warning is not a failed
build. The prior native test crashes and settings-pointer retry are preserved
in their original notes; their causes were not rewritten as proven product bugs.

## Exact remaining F-AC3 case

The same GUI scan has not yet been shown to include all three of:

1. A task-owned root whose directory enumeration is actually ACL-denied.
2. A healthy explicit project root that remains visible in review.
3. A healthy isolated global provider target that remains in that same report.

The successful GUI request explicitly used `include_projects=true` and
`include_global=false`. The all-scope CLI request used a separate fully isolated
home/provider environment and retained Maven alongside the project and denied
root. Native bootstrap retaining the real USERPROFILE is not equivalent to that
CLI isolation. Their reports cannot be combined into one GUI pass.
The saved fixture environment is recorded before the measurement launcher
restores its parent's HOME/USERPROFILE, so it is not a GUI effective-environment
capture. Both earlier isolated launch scripts explicitly select the installed
WebView runtime; the second manifest omits that setting. Their missing CDP
endpoints have no established cause, and do not prove a USERPROFILE requirement.
The later [isolated C3 bootstrap diagnostic](native-isolated-bootstrap-c3.json)
and [dual-stack follow-up](native-isolated-bootstrap-c3-02.json) rule out
elevation and missing debug arguments for those specific launches:
host/app/WebView tokens were non-elevated, and browser/renderer command lines
had the requested port and isolated user-data folder. The follow-up observed
no listener in either address-family table across 120 samples over 29.916 s.
Visible page contents and the reason for no observed listener remain unverified.
Both runs closed normally without Scan, so neither closes the combined GUI
criterion.

[ACL evidence](native-clean-acl-c2.json) and the
[independent acceptance audit](../../10-13-cli-tauri-runtime-performance/evidence/independent-c2-check.md#acl-and-repaired-provider-evidence-follow-up)
record this boundary. The ACL was confined to a disposable task fixture; its
exact original DACL and all four sentinel hashes were restored. The wrong-scope
earlier driver attempt and bootstrap failures remain excluded from acceptance.
No preview, execute, cleanup or maintenance ran. F-AC3 stays unchecked; no new
product defect is inferred from the missing combined observation.

## Evidence levels supporting parent P-AC2

P-AC2 does not add a requirement to run every provider in a packaged GUI. Its
golden categories are supported by these distinct, complementary layers:

| Layer | What it proves | What it does not prove |
| --- | --- | --- |
| Shared core/rule and production-pipeline tests | Rule/ownership facts, real conversion/validation/preview, negative sentinels, reparse/protection, cancellation/budget outcomes, and terminal/interactive policy equality | Every live installed tool/configuration, or every platform's native behavior |
| Deterministic CLI/Tauri adapter and generated Rust/TypeScript wire/decoder tests | Shared services, result/health representation, selection/digest/cancellation correlation, closed decoding | Environmental provider discovery merely from schema agreement |
| Packaged native/CLI equal-input reports | Every-field report equality for two-drive explicit roots, healthy/missing/empty scopes, and actual denied-root plus healthy project | The unperformed native all-scope global-survivor case |
| [Six repaired-provider B3/C2 CLI golden pairs](../../10-13-cli-tauri-runtime-performance/evidence/repaired-provider-golden-parity.md) | Complete JSON, stdout-byte and exit equality after optimization for Go, configured known roots, npm, nested Python, partial roots and Yarn refusal; immutable fixtures and independent expected outcomes | A CLI-to-Tauri provider run, real package-manager configuration certification, or performance improvement |

The repaired-provider pairs use the same task-owned native query executable
identity for both builds. All twelve CLI scans exit naturally; fixture hashes,
timestamps and retained data remain unchanged. The path-only expected-value
checker correction was reassessed offline while preserving the original driver
and failed assertions; neither complete envelope was normalized.

## Platform and safety limits

- Native execution evidence is Windows. Unix Node shebang/script branches and
  configuration paths were implemented/reviewed but not executed on this host;
  platform CI remains unverified. New generated-output activity proof returns
  inspect-only when activity cannot be established. Windows app/temp families
  explicitly report unsupported elsewhere.
- Windows owner-state pipeline substitution is a scoped test seam, separate
  from actual ToolHelp/SID/no-sharing-handle helper tests. F1 root-reparse and
  cloud no-follow cases use the injected reparse probe; they are not claims of
  creating a native junction, symbolic link or Cloud Files provider.
- Browser/editor/temp support is limited to the documented default layouts and
  exact safe cache children. Custom/portable/delegated/ambiguous layouts remain
  unsupported or inspect-only. Approved version ranges are finite; fixture
  versions do not establish that every historical provider binary was installed.
- Command execution fixtures only record approved argv; trash runners record
  paths. No real user cache, Cargo home, source/secret/profile/session/model data,
  software uninstall or system maintenance was used to satisfy an acceptance
  condition. No permanent-delete policy or automatic application termination
  was added.
- Runtime resource/latency gates and Earth-specific acceptance remain with
  their owning children. This summary does not promote their component or
  preparation results to full programme acceptance.
