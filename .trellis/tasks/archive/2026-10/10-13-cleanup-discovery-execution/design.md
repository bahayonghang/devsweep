# Shared discovery and cleanup design

## Data flow and ownership

```text
CLI args / desktop explicit scope
  -> core validated effective scope + trusted provider context
  -> read-only project and provider discovery
  -> candidates + root/provider coverage + capacity confidence
  -> real v2 intent conversion and trusted plan validation
  -> explicit selected ids + read-only preview / eligibility
  -> current digest + confirmation
  -> live resolver/identity/owner/protection recheck + audit
  -> existing command or Recycle Bin executor
  -> per-target outcomes + optional explicit rescan
```

Tauri continues to use core services in-process. React receives closed DTOs and never normalizes an action path, resolves authority or generates argv. CLI wraps the same facts in human/JSON presentation. Progress observations remain authority-free and cannot become a completed plan.

## Repair the existing trust boundary first

Unify provider root resolution in core so discovery, persisted intent reconstruction and live safety use the same closed rule-specific policy. Configuration input is trusted context only after validation: absolute path, permitted root/footprint, no protected overlap, no reparse escape. A rule id plus caller path is insufficient.

Register the Go module-cache command using a typed intent and documented supported arguments, rather than letting unregistered discovery break report conversion. Bind its configured footprint and recheck it on execute; never let `go` found on PATH alone suppress an inspect/diagnostic fallback after failed discovery. Preserve intentional command irreversibility and selection policy.

For Python, specify supported nesting per rule and retain the nearest valid project marker and relative artifact evidence. Do not turn exact-child rules into arbitrary-descendant authorization. For providers, parse their documented structured or single-path output; use exit/truncation/status/stderr separately instead of discarding lines containing spaces or keywords.

Invalid external plans remain rejected as a whole. Scanner-side recoverable provider failures become explicit diagnostics/coverage entries before plan creation, while verified unrelated candidates survive. Do not weaken Tauri validation to hide scanner bugs.

### Provider scope and preview binding

Use one core-owned binding for CLI saved v2 plans and Tauri reviewed in-memory v2 payloads. Persist only the untrusted observed scope needed to reconstruct a provider: rule/provider id, observed absolute cache footprint and the rule-specific configuration source. Observation fields do not grant authority, and a submitted executable path or action is never trusted. Add scope fields with explicit v2 compatibility fixtures; a legacy report missing evidence required by a configured provider must be rescanned. Existing fully reconstructible local rules retain their current behavior.

At preview, resolve through the trusted provider policy and build a canonical binding containing the rule/provider id, validated absolute footprint, resolved executable/interpreter canonical path and content identity, supported version/capability, and a fingerprint of the allowlisted effective configuration that controls that footprint. Exclude credentials and unrelated environment/config values from both the binding and logs. Bind this derived snapshot into the preview digest alongside the selected manifest. Freeze its canonical serialization and golden digest fixtures before changing adapters.

Execute reacquires these facts, compares the binding before side effects, and invokes the bound resolved program with separate argv. A changed footprint relative to the observation requires rescan; a changed tool/configuration binding after preview requires a fresh preview and digest (and rescan if scope changes). Same-name/same-path executable replacement and same-path configuration-content drift must be detected. Do not cache a preview identity as live execution authority. Existing path/marker/owner/protection checks and audit ordering still apply.

## Effective roots and persistence

Desktop replaces the invisible `.` default with visible root selection, existing-directory suggestions, and an explicit global-cache scope. Normalize/dedupe selected roots in core; report their absolute effective values. Support add/remove of selections and explain empty/invalid selections before starting. Removing a nested root does not exclude it when a remaining parent still covers it. This task adds no glob, negative-root or descendant-exclusion feature. Existing durable cleanup protection affects eligibility and does not promise omission from read-only observation. Remember selected roots with the existing app-data persistence conventions, using a narrowly versioned scan preference contract if no suitable store exists. Do not overload appearance preferences or merge stored GUI roots into CLI default semantics.

An explicit CLI `--root` remains authoritative for that invocation. A future shared config flag must be separately specified rather than silently changing existing scripts. Exact known AI worktree containers may be suggested, but only recognized artifacts inside them can become candidates. No whole-worktree deletion.

## Coverage and compatibility

Add a core-owned report structure for effective scope and per-root/provider results. Use existing diagnostic stage/provider and completeness primitives where possible. Freeze enum names in a real Rust/JSON fixture before changing adapters. Distinguish attempted-empty, absent, unsupported, skipped/protected/busy, failed/timeout/truncated, canceled and budget-limited; count warning/partial effects deliberately. An absent optional tool is not a process failure, while failure to inspect a selected root makes coverage incomplete.

Keep plan v2 unless an authority change truly needs a new persisted version. Additive report fields need explicit older-observation defaults/compatibility tests; closed desktop decoders and generated samples update together. Unknown required authority remains rejected. If a schema migration is needed, document/read old reports safely without upgrading them to new authority.

Bind changed selection, roots, protection, provider configuration or reviewed-size observations to stale-preview invalidation. Reuse core's reviewed rescan with a typed Tauri bridge and production fixture. Global phase failures must not silently disappear; zero-target reports still carry coverage and diagnostics.

## New providers

Implement `coverage-matrix.md` in waves through the existing rule catalogue and trusted registry. Do not create a plugin framework. Use platform-known directories and exact supported application/tool layouts; read current official tool documentation before freezing commands, supported versions or config precedence.

New broad build-output rules require generated-content/marker evidence, no tracked source/nested repo/secret ownership, and a defined activity policy. Browser/editor rules may cover regenerable cache children only and are initially unselected. User temporary cleanup lists individual entries older than the proposed seven-day threshold; unavailable activity evidence or active locks fail closed. Scanner reads may size these items, but only the existing executor can move them.

## Presentation and outcomes

Reuse existing CLI catalogue/table patterns and React review components. Show target id for CLI selection, full accessible/copyable paths, observed occupancy, action/risk, rebuild cost, size confidence and capability. A persistent scope/coverage summary stays visible for empty/partial scans. Existing selected-estimate stage behavior remains.

Separate observed occupancy, selectable estimate, action-specific reclaim estimate (only if defensible), moved-to-Recycle-Bin bytes, and owner-command outcomes. Full pnpm-store size is not a prune promise. Command-only success shows its successful action and an optional explicit rescan rather than implying zero cleanup. Do not infer freed disk space from command exit 0 or trash movement.

Read-only eligibility checks may make upcoming refusals visible, but the executor still performs live authorization. An item can become busy or protected after preview; show the actual refusal and recovery action without auto-closing apps.

## Testing and rollback

The principal new test seam is real scanner output through real v2 conversion/validation/preview, with fake external tools and execution runners. Include mixed valid/invalid-provider contexts and sentinel user data. Isolate app data, tool environment and temporary roots; never execute reference scripts or test cleanup against user caches.

Land repair, scope/coverage DTOs, each provider wave and presentation in separable commits during implementation. Disable/revert a problematic new provider without reverting current-rule repairs or accepting an invalid plan. Runtime child edits shared scan/progress modules only after this child's DTO/semantic changes settle or by explicit file handoff.
