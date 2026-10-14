# Runtime implementation plan

## Entry gate

- [x] Parent review approved; activate this child for its implementation phases.
- [x] Coordinate immutable baseline and golden fixture capture before other children change product code.
- [x] Read current and inherited budgets; identify user-owned processes and exclude them from tooling.

## Ordered work

1. [x] Adapt the resource harness to an isolated read-only/task-owned mode. Add process-tree and I/O sampling, missing-sample handling, build identity and coverage manifests. Verify no maintenance/cleanup/user-process stop occurs. See [baseline A checkpoint](evidence/baseline-a.md).
2. [ ] Build/preserve original release CLI and Tauri baseline A; execute safe workloads, capture phase/resource data, and remeasure old Software/Optimize/idle failures. Freeze exact host/fixture/method before comparisons.
3. [ ] Receive functional child's immutable corrected build B and golden results with stable event/coverage DTOs before shared runtime edits. Add many-target/wide-enumeration regression fixtures, then remove unused CLI snapshots and move expensive progress projection behind publication decisions. Verify B-to-C semantic parity, refreshing B after any later functional changes.
4. [ ] Address measured enumeration buffering/cancellation and redundant probes without changing fresh child sizing budgets or live authorization. Report any intentional global-budget proposal separately instead of silently reducing coverage.
5. [ ] Add TUI burst/input/idle redraw and Status slow-consumer/broken-pipe/cancel tests. Implement bounded/cancelable handoff and redraw fairness with lossless lifecycle delivery.
6. [ ] Profile real React preview/review at 100/1k/10k targets. Apply lazy detail rendering/stable derivations first; add accessible paging/windowing only if measured need remains. Preserve selection/authority and closed decoding.
7. [ ] Measure visible HUD, Status, combined sampling, hidden HUD, minimized main work and repeated cycles. Remove confirmed redundant sampling/wakeups without changing opt-in monitoring intent.
8. [ ] Fix any remeasured inherited Software/Optimize/Analyze/idle gate failures using phase evidence. Do not treat old task archival as a waiver.
9. [ ] Integrate UI and functional branches/changes through agreed file handoffs. Repeat paired B-to-C equal-coverage runs, A-to-C unchanged healthy workloads and separately publish added-coverage cost. Complete operation-specific native cancellation/close evidence.
10. [ ] Run quality gates and hand raw measurements, failure/unavailable list, coverage parity and cause/fix/effect record to parent.

Step 2 checkpoint: both immutable A release binaries, original frontend/source,
and deterministic fixtures are preserved; native packaged page and passive
normal close are proven. Initial Software timing fails its inherited bound.
The full idle/phase/operation matrix remains open; preparation samples do not
complete step 2 or runtime acceptance.

## Build and validation commands

At repository root:

```powershell
cargo build --locked --release -p devsweep-cli --bin devsweep
just ci
```

At `desktop/`:

```powershell
mise exec node@22 -- npm run types:generate
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
mise exec node@22 -- npm run tauri -- build --no-bundle
```

Record exact new safe-harness/benchmark invocations after they exist; do not invent a passing run or invoke the old side-effectful harness. Reuse current production Analyze and native tooling only after checking their scope. The parent integration runs the full root/frontend gates; focused implementation iterations use meaningful targeted tests.

## Review checkpoints

- Baseline before optimization; same effective roots/rules and raw outcome parity afterward.
- No dropping final/lifecycle/NDJSON records or removing decoder checks for throughput.
- No blocked producer-send/join deadlock in cancellation paths.
- No change to wait-only cleanup execution or durable audit writes.
- No new runtime dependency or unreviewed renderer/global sampler framework.
- Native app is actually the expected packaged release build; process-tree statistics include WebView2, while inherited host-PID limits remain clearly separate.
