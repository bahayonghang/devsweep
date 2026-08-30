# Design - Parent Integration Boundary

## 1. Task Tree

```text
08-29-scan-resource-bounds-app-icon (parent, planning/integration only)
├── 08-29-bound-sizing-concurrency
│   └── core sizing policy + tests + performance evidence
└── 08-29-generated-app-icon-integration
    └── product master + Tauri icons + native visual evidence + shell handoff
```

The parent has no direct implementation files. Its artifacts are the source
requirements, child map, approval boundary, and combined review contract.

## 2. Ownership and Overlap

The sizing child owns only core sizing code/tests and its measurement record.
The icon child owns only the stable master, generated native icon inventory, and
native small-size evidence. The new desktop-shell child owns React/TUI placement,
accessible naming, and responsive shell tests. Their product file sets do not
overlap.

Both children run `just ci`, inspect the same unrelated worktree dirt, and
report evidence to this parent. Those shared checks are coordination points, not
shared file ownership. Source edits may be delegated independently, but the
sizing child's warm-ups and five-pair A/B timing window is repository-exclusive:
no sibling build, test, package, icon generation, or other CPU/disk-heavy work
may run from baseline warm-up through the final candidate sample.

## 3. Approval and Lifecycle

- The latest parent plus both child summaries must be presented together.
- A later explicit user response approves implementation; task-creation consent
  does not.
- The sizing child is already `in_progress` but paused/partially unverified. Its
  existing `sizing.rs` pool, serial-mode, budgeting, cancellation, and test diff
  is the resume checkpoint; R4 throughput samples, Measurement Record, full
  validation, and final acceptance remain outstanding and `UNVERIFIED`.
- Resumption must continue that checkpoint without resetting or redoing its
  existing product diff. Starting a new child applies only to the icon task.
- Start the child that owns the next deliverable. Do not run `task.py start` on
  this parent.
- After each child passes its focused validation and independent check, commit
  only that child's product changes and immediately archive that leaf through
  `task.py archive`. This is the approved leaf-first commit-then-archive cadence.
- After both children are archived, the parent performs the read-only combined
  diff, evidence, and scope review from commit history plus the moved artifacts
  under `.trellis/tasks/archive/`. Record the result in this active task's
  `acceptance.md` using the format defined by `implement.md`. A PASS does not
  archive this parent: keep it active until `08-29-five-mode-native-integration`
  passes, then archive it in the approved hierarchy.

## 4. Cross-Child Integration Review

The final parent review verifies:

1. the scan resource claim is limited to the hard worker ceiling and recorded
   same-host timing unless separate whole-system metrics exist;
2. the icon claim is limited to direct Windows evidence and decoded generated
   files on other platforms;
3. neither child added dependencies, persisted contracts, cleanup authority, or
   task-directory runtime paths;
4. the combined diff still excludes every pre-existing dirty path outside the
   task tree's declared change lists; and
5. child validation remains green together.

## 5. Rollback

Each child has an independent rollback. A failure in one child does not require
reverting the other. The parent has no data migration and no product code to
roll back.
