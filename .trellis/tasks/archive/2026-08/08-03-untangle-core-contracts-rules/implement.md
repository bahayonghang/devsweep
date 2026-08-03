# Core Contracts And Rules Implementation Plan

## Prerequisite

- Parent planning summary is approved.
- Start this child, not the parent.
- Confirm the baseline worktree contains only authorized task artifacts.

## Ordered Checklist

- [ ] Record targeted baseline results for model, rules, plan validation, Cargo
      metadata, scanner catalogue, and registry behavior.
- [ ] Create the model module directory and move plan versus scan/report types
      without changing derives, serde attributes, constants, or behavior.
- [ ] Move `ProcessResult`/`ProcessStatus` diagnostic mapping out of model and
      update Cargo metadata callers/tests.
- [ ] Extract neutral `cargo_metadata` implementation from safety; update
      scanner caching and safety live revalidation imports.
- [ ] Create the rules directory; move every scanner/provider `RuleDoc` and
      declarative rule table under one owner.
- [ ] Move trusted action registry implementation under rules and expose only
      the crate-private resolver required by plan validation.
- [ ] Create the plan directory; keep validation at its narrow interface and
      move canonical digest implementation private.
- [ ] Remove obsolete modules/imports and shrink visibility introduced only by
      the old flat layout.
- [ ] Update affected backend Trellis specs for model, plan, rules, registry,
      and Cargo metadata ownership.
- [ ] Run targeted tests, then `just ci`.
- [ ] Inspect the full diff for serde/action/digest/safety changes; commit only
      this child, archive it, and record the session before starting child 2.

## Targeted Validation

```powershell
cargo test --locked model
cargo test --locked rules
cargo test --locked plan
cargo test --locked scanner::tests::truncated_cargo_metadata
cargo test --locked safety::tests::cargo_metadata
just ci
```

If module paths change test filters, use the new exact module names and record
them in the child journal. The full `just ci` result is authoritative.

## Review Checklist

- `rg` confirms model has no process import.
- `rg` confirms rules has no scanner/provider import.
- One owner exists for every `*_RULE_DOC` and rule ID.
- Cargo metadata process/parser code has one production owner.
- Plan JSON snapshots/round trips and digest tests are unchanged in meaning.
- No public compatibility re-export was added without a dated removal item in
  the parent final child.
