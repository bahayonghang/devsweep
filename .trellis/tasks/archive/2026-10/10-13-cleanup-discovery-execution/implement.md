# Shared functionality implementation plan

## Entry and ownership

- [ ] Parent plan approved; activate this child, not the parent, for code work.
- [ ] Runtime child preserves the baseline binary and golden fixture before changes.
- [ ] Agree coverage DTO, progress ownership and shared-file handoff with runtime child. UI child independently owns only Planet/stage rendering.

## Ordered work and verification

1. [ ] Add failing production roundtrip fixtures for Go, configured Gradle/NuGet/Go, space/Unicode paths, nested Python caches, and mixed valid providers. Verify the failures represent current code, not a mocked bridge.
2. [ ] Freeze the provider scope/binding schema, canonical digest fixture and legacy-v2 rescan behavior from `design.md`; then repair typed intent registration and shared trusted root resolution across scanner, registry and execution safety. Add same-path config-content/tool replacement, marker drift and tampered-path negatives; preserve explicit execution authority.
3. [ ] Improve provider parsing and fallback, retaining timeout/permission/truncation/absence results. Freeze core coverage DTOs and migrate CLI/Tauri fixtures/decoders together.
4. [ ] Implement visible desktop selected-root add/remove/deduplication, suggested existing locations and narrow persistence. Preserve CLI invocation semantics and expose effective scope. Verify unrelated CWD, multiple roots/volumes, invalid roots and removal of a nested selection still covered by its parent. Do not add exclusion syntax or conflate scope with cleanup protection.
5. [ ] Implement new catalogue wave B, one family at a time. For each, read official tool docs, record exact supported version/config/footprint, and run the per-rule matrix before enabling it.
6. [ ] Implement Windows browser/editor/temp wave C behind closed built-in rules with conservative selection and active-owner guards. Verify protected sentinels and native path/owner behavior. Never enable whole app/profile/temp roots.
7. [ ] Add useful bilingual CLI scan/preview/outcome rendering and truthful JSON/exit classifications. Keep named outputs create-new and machine strings locale-neutral. Regenerate parser-owned CLI docs if grammar changes.
8. [ ] Update Tauri empty/partial review, capability explanations, command/trash results and reviewed-size rescan. Preserve reducer correlation, selected-estimate stage, protection and stale-digest rules.
9. [ ] Run CLI/Tauri normalized parity over one golden fixture set and hand stable DTOs/shared files to runtime optimization. Publish immutable functionally corrected release builds and golden observations/plans as runtime reference B before performance edits. Refresh B whenever functional semantics change; retain original baseline A. Re-run coverage parity after performance changes.
10. [ ] Record native packaged Tauri and CLI smoke evidence using isolated disposable fixtures. Publish rule coverage/status matrix and unresolved failures to parent; do not mark an unverified row complete.

## Validation

Current integrated acceptance: [functional acceptance summary](evidence/acceptance-summary.md)
maps the final C2 gates and evidence to F-AC1–F-AC8. Seven criteria pass; F-AC3
retains the combined native GUI denied-root/healthy-project/global-survivor gap.
The implementation checklist and earlier checkpoint below are historical work
records, not a substitute for that criterion-level ledger. Task status,
archival, journal and commit decisions remain unchanged.

Desktop implementation checkpoint (2026-10-13): steps 4 and 8 are implemented
and focused adapter/store/frontend checks pass. See
[desktop functional evidence](evidence/desktop-functional.md) for exact commands,
the eight-case root-control visual check, native test anomaly and remaining
packaged/parity acceptance. These steps remain unchecked until integrated
native requirements are verified; this is not task completion.

At repo root:

```powershell
just ci
```

At `desktop/` with Node 22:

```powershell
mise exec node@22 -- npm run types:generate
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
```

Check that a second type generation is unchanged, and Rust wire-parity tests accept every real fixture. Use focused failing/regression tests while implementing; run the full gate at handoff. CLI integration must include real saved observation -> selected plan -> preview digest; dispatch verification uses fake runners or explicitly disposable task-owned fixtures.

## Acceptance evidence and stop points

- Preserve per-rule sources, exact fixture roots, IDs and retained-data sentinel checks.
- Record absence, unsupported, partial and execution failures separately; every matrix row remains accounted for.
- If tool semantics cannot bind a cleanup footprint safely, keep inspect-only with the reason and leave that executable criterion open for review.
- No production dependency, privilege escalation, new permanent-delete policy, user-data cleanup or broad task expansion is implied by this plan.
- Do not solve plan-validation failures by dropping trust checks or accepting frontend-supplied actions.
