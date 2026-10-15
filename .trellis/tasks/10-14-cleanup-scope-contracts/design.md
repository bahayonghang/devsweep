# A: technical design

## Design and evidence

- Source anchors: scan/mod.rs:203 and :455/:503; application/cli.rs:79; commands/clean.rs:47; desktop useScanScope.ts:8; ScanPage.tsx:104.
- Do not build a second scanner. Scope comes from the existing ScanOptions. Coverage absence is not an empty healthy result.
- Own shared policy and capability contracts. If B/C needs new read-only DTOs, freeze their schema with those owners first.
- Correct the related ranking spec drift about unknown mtime while preserving actual conservative behavior.

## Owned boundary

Core scan/ranking/model/provider catalogue, CLI clean adapter/contract tests, necessary TUI selection parity, backend scan-coverage/provider-authority/quality specs.

## Compatibility

Preserve the parent design's saved-report/plan, appearance and explicit-root contracts. New wire fields require generated Rust/TypeScript/decoder parity and old/new fixtures. No UI-derived cleanup authority.

## Dependencies and handoff

None. Parent planning package must be reviewed before implementation.

## Rollback

Revert this bounded contract/policy change without rewriting saved observations, plans, or user settings.

