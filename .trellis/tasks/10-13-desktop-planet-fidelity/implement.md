# Earth implementation plan

## Entry gate

- [x] Parent's latest plan approved; this child becomes the explicit implementation target via `task.py start`.
- [x] Read curated manifests and accepted asset decision; coordinate shared performance evidence with runtime child.
- [x] Save pre-change production/native build identity and baseline. Do not substitute historical September screenshots for current evidence.

## Ordered work

1. [ ] Confirm texture source and ship-size budget; acquire/process the local derivative and fallback with source/checksum/notices. Verify packaging and offline decoding.
2. [x] Main session updates only the superseded Clean visual-spec/provenance clauses before renderer edits.
3. [x] Add the focused Canvas2D texture candidate, bounded source/backing sizes, reusable buffers and balanced filtering/light. Preserve other modes and current preference semantics.
4. [x] Fix resize/DPR still-frame handling, hidden-first-mount work, stale decode disposal and context/asset fallback. Add tests for these observable failures.
5. [x] Add a production Planet benchmark over the real component; record the Canvas comparison. If necessary, investigate one dependency-free GPU alternative and record why it is warranted before integration.
6. [ ] Capture EN/zh-CN, viewport, scaling, theme, forced-colour and reduced-motion evidence. Preserve the user's configured font.
7. [ ] Run integrated scan-replay responsiveness and native process-tree measurements with the runtime child. Write pass/fail/unavailable results with raw evidence links.
8. [ ] Hand final asset/renderer decision, screenshots and budgets to parent; check no unrelated UI or cleanup-contract changes.

## Validation commands

Implementation checkpoint: [renderer measurements](./evidence/renderer-measurements.md).
26 focused tests and stage/benchmark lint pass. Asset notices and local payload
are complete. Remaining unchecked work includes final native packaging/offline,
visual matrix, native lifecycle/process-tree evidence, and integrated quality
gates. The runtime child owns that integration measurement handoff.

From `desktop/`, with Node 22:

```powershell
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
```

Add a dedicated benchmark script during implementation and record its exact invocation in the evidence. If shared/Tauri contracts change, regenerate fixture types and run root `just ci`; the parent integration always runs the full canonical gate.

## Risk and review checkpoints

- Verify source rights and all packaged asset paths before visual review.
- Freeze baseline, host and budgets before comparing renderers; do not lower limits after a failing result.
- Keep fallback failures independent of operation buttons, keyboard navigation, and cleanup state.
- Native evidence must come from a Tauri-built application with bundled UI, not a bare Cargo executable or Vite-only page.
- A package addition, global redesign or new cleanup behavior requires a plan revision.
