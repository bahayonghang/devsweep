# Technical design

## Boundaries and existing architecture

Keep CLI -> shared core and Tauri -> shared core. The desktop must not invoke the installed CLI as a second execution engine. Existing ScanOptions already expresses the two scopes. "CLI-cleanable" means one trusted action catalogue and authorization path across surfaces.

Use the existing ScanReport V1 and CleanupPlan V2 authority boundary. Both report and health DTOs reject unknown fields (crates/devsweep-core/src/model/scan.rs:9-27, :48-74); an arbitrary additive field is not automatically backward-compatible. Prefer a separate versioned read-only advisory DTO around a core summary projection, exposed through Tauri, leaving saved report/plan formats unchanged. Any unavoidable wire change must have an explicit old/new compatibility matrix, generated types and decoder tests before implementation.

## Scope-local state

One Clean-owned session controller holds two slots keyed by global/project, each with completed report identity, timestamp, coverage, and review selection. One active operation identity is shared with the existing coordinator.
Global request: include_global=true, include_projects=false, roots=[].
Project request: include_projects=true, include_global=false, roots=backend-validated explicit roots.
A busy scope disables the other start action with a reason; cancellation is explicit and joined before replacement. Fence every event by scope, scan id, sequence, and report revision.

Keep selection/plan/dry-run/digest/confirm bound to one originating report. Never concatenate reports or synthesize a combined executable plan. Editing project roots invalidates project observations and authority immediately, preserving unrelated global observations. Protection changes invalidate all affected authority. Threshold changes affect advisory projection only.

Reproduce Clean -> Settings/other mode -> Clean before implementing retention. Hoist the Clean controller's lifetime to a stable Clean-owned boundary without moving cleanup authority into AppShell. Do not keep hidden busy workers alive merely to preserve React components; route/close/unmount follow cancel-and-join policy.

## Global capability and selection

Reuse existing provider rules, discovery, live executable/version/config/path binding, and irreversible-action disclosures. uv remains uv.cache.clean; no implicit switch to prune. Tool absence, zero footprint, failed probe, unsupported version/config and inspect-only are independent from estimated size.

Project default selection becomes false in the canonical core policy, including old fully measured Rust targets. CLI explicit --select remains unchanged; TUI consumes the same safe defaults. Update ranking expectations/specs and existing frontend tests. A warning never changes this value.

## Project aggregation and mbx

Group by backend Scope::Project root. Define non-authoritative artifact summaries over build/test/tool-cache target kinds. Keep dependency/venv rows outside the build-artifact warning sum.
Deduplicate footprint aliases before sums. Current path/action dedupe is insufficient for different actions over the same storage; ownership summaries must identify overlaps and avoid double counting without hiding selectable actions.

mbx has two domains:
1. Shared action store: global inspect-only occupancy/availability, not per-project bytes and not automatic gc.
2. Workspace managed target/private learned state: project-owned candidate once attribution is proven.

B begins with a qualification spike for supported mbx versions and stable public report/config contracts. Do not parse a display table or unstable internal session file into authority. Generic Cargo continues refusing reparse targets. A specialized adapter may inspect only when the workspace/link/configured managed root/current identity proof is complete.
A proposed closed mbx workspace-clean intent must enumerate the complete mutation set for the pinned tool version: the workspace target link entry itself, managed target contents, private learned state, and any affected metadata. Rebind each affected path/identity, workspace, tool, relevant config, protections and live busy/lease state at preview/execute. Preview/confirmation must explicitly disclose structural link removal as well as capacity. Use separated program/argv. Exclude --under, cache remove, gc, and direct directory deletion. Corroborate actual behavior through a version-matched source/public-contract audit; mocked execution alone is insufficient. If that evidence is unavailable, a fully isolated disposable-workspace conformance run requires separate authorization and cannot use user cache storage.
If the public interface cannot support this proof, retain truthful inspection but leave executable mbx acceptance open. Do not weaken no-follow policy, introduce an unbounded command, or silently drop the requirement.

## Capacity advisory

T default = 5 * 1024^3 = 5,368,709,120 bytes, inclusive.
Input: completed project report, verified owner/footprint summaries, configured threshold.
Output: owner, included kinds/unique target references, known logical bytes, confidence, assessment, threshold and measurement identity.
- Complete value >=T: oversized.
- Complete value <T: below threshold.
- Partial lower bound >=T: oversized, "at least".
- Partial lower bound <T or unknown overlap/size: incomplete, never below threshold.
- Shared/unattributed mbx bytes: separate, excluded from per-project sum.
Hardlinks and filesystem allocation mean logical occupancy is not guaranteed physical reclaim.

Use a focused core-owned clean advisory preference document (proposed clean-advisories-v1.json beside existing settings); do not alter the roots V1 or desktop appearance V2 store for an unrelated setting. Store threshold bytes, expose a positive whole-GiB UI value with overflow-safe conversion. Missing store uses default without write. Corrupt/future/unknown data is preserved; invalid save fails visibly and previous committed value remains. A session-only value may be used with a visible persistence error.
The projection is pure and read-only. Setting changes can re-evaluate the displayed completed report without another scan. No timer or watcher.

## Visual and identity design

design-system/DESIGN.md is the task's authoritative proposal, generated from style:shadcn and customized. Promote to desktop/DESIGN.md during D; preserve historical root design.md.
Use existing plain CSS/custom properties. New palette roles are Clean-local aliases/overrides beneath the Clean workbench root, inheriting the saved theme/font/scale selection. Do not replace shared root appearance colors. Two asymmetric sections at wide widths, stacked at compact widths; real status/rows replace the Clean Earth hero. Preserve mode navigation and other planet renderers. Other modes and Settings require before/after palette/geometry comparison; the intentional shared logo is the only cross-mode visual replacement.
Record explicit spec amendments for Clean's stage-first/no-visible-header/mandatory-Earth rules in component-guidelines.md and desktop index. Existing accessible theme/font/scale choices remain valid.

E owns the webview/master/native icon files, compact character variant and provenance. D owns semantic Glyph usage, controls, layout, locale copy and warning presentation. Coordinate any shared shell import change; do not edit the same asset in parallel.

## Compatibility, rollout and rollback

Preserve CLI command grammar/default scope and saved plans. Core selection defaults intentionally become more conservative for project outputs. Unsupported provider versions remain visible and blocked. Existing roots/preferences remain readable.
Land core contracts/qualification before consumers; regenerate DTOs from Rust, never hand-edit types.gen.ts. Validate old stored bytes and native/CLI parity.
Rollback by reverting a bounded child change; do not restore a warning model by deleting stores or changing user cache paths. Keep generated assets with reproducible conversion and prompt provenance.
The pre-existing desktop/src-tauri/Cargo.toml modification belongs to the user and must remain untouched.

## Technical risks and proof obligations

- mbx may lack a stable per-workspace interface sufficient for DevSweep's preview digest. This is a completion gate, not a harmless UI omission.
- Route changes may currently destroy state. Source evidence is strong, runtime reproduction remains NOT_RUN.
- Two report slots increase retained memory; preserve current 10k selection/render tests and measure production p95.
- Input selector mismatch is confirmed; screenshot font cause remains unknown.
- Generated mascot v1 is a concept with too much detail for tiny sizes; no production-brand acceptance is claimed.

