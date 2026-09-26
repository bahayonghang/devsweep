# Implementation: Settings And Runtime Preferences

## Entry And Sequence

- [x] The user approved the parent summary and implementation of both children.
- [x] Load this child's PRD/design/manifests and parent research.
- [x] Window-control integration and focused tests are complete. The final product review covers both children; native acceptance remains open.

## Ordered Work

1. [x] Update affected theme/font, language-store separation, HUD read, and IPC spec clauses. Keep fixed safety and performance contracts explicit.
2. [x] Add the core-owned closed desktop preference DTO, defaults, typed patches/group resets, and versioned atomic store. Test missing/invalid/newer files, failed writes, exact language-byte preservation, and concurrent unrelated patches. See evidence/backend-report.md.
3. [x] Add Tauri get/update coordination and ordered committed snapshots. Update main/HUD invoke allowlists, real fixtures, wire parity, decoder, and generated types through the existing generator.
4. [x] Build the dedicated Settings page under the existing supporting route. Add grouped fields, localized labels, previews, pending/error/apply timing, and separate group reset actions. Reuse the language bridge.
5. [x] Apply shared main/HUD dark/light/system themes and local font/scale tokens. Update only affected literal styles and appearance assertions. Component tests cover media changes, initialization, save failures, and stale responses. Native appearance evidence remains under step 10.
6. [x] Wire effective motion and 15/30 FPS to the actual Planet loop and decorative CSS. Preserve visibility cleanup and fixed renderer limits.
7. [x] Wire committed Status interval and returned rows to snapshot/live calls and the existing interval control. Verify commit-before-restart and cancel/join ordering.
8. [x] Wire the HUD interval at the next show. Preserve one sampler, hide/exit join, read-only HUD IPC, and immediate appearance updates.
9. [ ] Run focused tests as each owning layer changes. Frontend gates, Tauri tests, and workspace Clippy passed. The required just ci gate failed before core unit tests started because the executable was missing; cause unknown. See evidence/check-report.md.
10. [ ] Review all modes, supporting pages, dialogs, titlebar, Settings, and HUD in both locales under dark/light/system, font scale, reduced motion, and forced colors. Record native and fixture evidence separately.
11. [x] Hand evidence and the complete configuration matrix to the parent. The parent integration report links the backend/frontend/check reports and the design matrix. Previous performance/native task outcomes remain unchanged.

## Validation Commands

From desktop:

- mise exec node@22 -- npm run types:generate
- mise exec node@22 -- npm run lint
- mise exec node@22 -- npm run typecheck
- mise exec node@22 -- npm run test
- mise exec node@22 -- npm run build

From the repository root:

- just ci

Generated files must match fixtures and Rust declarations. Focused tests must cover meaningful behavior, including both Status call sites, event order, actual sampler/frame cadence inputs, invalid stored bytes, group reset scope, and old language compatibility. Do not add tests that only repeat a constant without exercising a consumer.

## Native And Visual Evidence

Continuation on 2026-09-26 follows the user's explicit instruction to use no
image features or screenshot verification. Text browser results are recorded
in ../09-25-desktop-window-settings/evidence/text-browser-20260926.md.
Unperformed native/forced-colors/visual checks remain open.

Use the existing 390/800/1024/1440 CSS viewport protocol and labelled native scale evidence. Keep the current actual Windows display scale. Cover each theme and font preset across both languages; use 100 and 125 percent text scale for the boundary layout checks and verify the 110 percent token path.

Native checks include system-theme changes, current HUD appearance, hidden/new HUD initialization, changed HUD cadence after reopening, Status start/stop/restart, and combined titlebar behavior. No real cleanup, software removal, startup mutation, or system optimization is required for this task.

## Rollback And Review

Rollback new desktop preferences and consumers together; retain the new user file and preserve the old language store. No uninstall, file deletion, database migration, production dependency, or installer run is part of the task.

The final check reviews only this task's product changes and combined titlebar/settings behavior. A general coordinator defect or old performance gate failure belongs to the existing owning task and must be reported without changing its threshold.

## Context Loading Note

The full backend quality guide exceeds the configured 32 KiB per-file injection limit. The manifests load the backend index and scoped research instead. Before affected Rust edits, read the relevant quality-guide sections directly in bounded chunks. Do not treat a truncated injected document as the full contract.
