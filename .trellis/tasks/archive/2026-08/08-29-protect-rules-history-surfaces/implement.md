# Implementation Plan - Protection, Rules, and History

Implementation remains gated on explicit approval. Before any edit, verify that
the Clean, Software, and Optimize audit schemas and shell registry are accepted;
otherwise keep this task paused in phase 7.

## Steps

1. Freeze supported audit schema versions and redaction fixtures from the three
   domain owners; reject executable argv and plan payloads at the DTO boundary,
   and prove the reader opens only the three fixed V1 stores without legacy path
   discovery/import.
2. Implement the sidecar lock over the complete reload/validate/compare/
   temp-flush/atomic-replace transaction. Preserve corrupt/unknown bytes and
   fail closed; add two-process lost-update/conflict fixtures. Emit requested/
   committed/failed protection mutations through the accepted Clean V1 writer
   with only canonical-identity hashes, then add immediate Clean enforcement.
3. Implement the read-only rules projection and non-replayable history readers.
4. Implement the already frozen CLI commands and
   `presentation/{protect,rules,history}.rs` human renderers before adding
   TUI/Desktop adapters.
5. Add supporting destinations under Clean/help/overflow without changing the
   five-mode primary navigation.
6. Run focused persistence, schema, redaction, cross-surface, native, and full
   repository gates.

## Verification

- `rtk cargo test -p devsweep-core protection`
- `rtk cargo test -p devsweep-core history`
- `rtk cargo test -p devsweep-cli protect`
- `rtk cargo test -p devsweep-cli rules`
- `rtk cargo test -p devsweep-cli history`
- `rtk npm --prefix desktop run test -- --run support`
- `rtk npm --prefix desktop run lint`
- `rtk npm --prefix desktop run typecheck`
- `rtk npm --prefix desktop run build`
- `rtk just ci`
- Manual native gate: English/Chinese, keyboard-only, screen reader, high
  contrast, narrow/wide layouts, concurrent protection writers, corrupt/newer
  stores, mixed audit versions, redaction inspection, and proof that no history
  row can execute or replay an action.

## Stop and rollback points

- Stop if any domain audit schema is still provisional, if unknown versions are
  silently accepted, if corrupt protection bytes would be cleared/deleted, or if
  UI/CLI output can reconstruct executable argv/raw protected paths.
- Roll back adapters and readers without deleting or downgrading protection or
  audit files. Preserve unknown/newer documents byte-for-byte.
