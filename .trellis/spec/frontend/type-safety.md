# Type Safety

> Type safety patterns in this project.

---

## Overview

TUI code is Rust code and should use the same typed domain model as the backend.
Do not create UI-only string schemas for cleanup targets, actions, risks, or
evidence.

---

## Type Organization

- Shared cleanup types live in `src/model/`; JSON-facing `UntrustedPlan` and
  `CleanupIntent` are distinct from internal `CleanupPlan`/`CleanAction`.
- CLI argument types are private to `src/application/cli.rs`.
- TUI-only state types live under `src/tui/app/`; the event protocol is owned by
  `app/events.rs`, while `App` remains in `app/mod.rs` and is shared read-only
  with render code.
- Runtime service traits and adapters stay private under `src/tui/runtime/`.
  Render-only helpers and types stay private under `src/tui/render/`.
- Presentation shared by app and render lives in typed, pure
  `src/tui/display.rs`; app modules must not import render modules to reuse text
  formatting.

Example shared type:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanTarget {
    pub id: TargetId,
    pub risk: RiskLevel,
    pub evidence: Vec<Evidence>,
    pub action: CleanAction,
}
```

---

## Validation

The TUI should receive already-typed data. JSON decoding belongs at file/CLI
boundaries, not in widget code. If future UI code loads a plan file, decode it
into `UntrustedPlan`, validate it through `plan::validate_plan`, and only then
use the resulting typed projection; never deserialize executable action details
into UI state.

---

## Common Patterns

- Use enums for closed sets such as `RiskLevel`, `Ecosystem`, and
  `CleanAction`.
- Use `PathBuf` for paths, not `String`.
- Use `TargetId` instead of raw strings when storing selected targets.
- Use explicit `UiEvent`, `WorkerEvent`, and `Effect` enums for TUI event and
  worker boundaries.

---

## Forbidden Patterns

- No `serde_json::Value` in render code.
- No stringly typed action dispatch such as `"move_to_trash"` when
  `CleanAction` exists.
- No unchecked indexing into target lists for selected state; store a stable
  target id or validate the index before use.
