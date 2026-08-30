# Implement - Parent Task Orchestration

This parent is not an implementation target. Do not run `task.py start` on it.

## 1. Approval gate

Present the latest parent and both child planning summaries. Continue only after
a subsequent user message explicitly approves that exact plan for
implementation.

## 2. Child execution

Recommended order:

1. after explicit approval, resume the existing `in_progress` sizing checkpoint
   without resetting its current diff or treating prior partial work as verified;
2. finish only its outstanding R4 throughput measurements, Measurement Record,
   full checks, evidence review, authorized product commit, and immediate
   leaf archive through `task.py archive`;
3. `task.py start 08-29-generated-app-icon-integration`
4. implement, check, record evidence, commit only that child's product changes,
   and immediately archive the leaf through `task.py archive`.

The children are technically independent at the source-file level and may be
delegated with exclusive ownership, but all sizing baseline/candidate warm-ups
and five-pair A/B measurements must run alone. Pause sibling builds, tests,
packaging, icon generation, and other CPU/disk-heavy work for that entire
measurement window. The sizing child is recommended first because it resolves
the reported resource pain.

## 3. Parent integration review

After both children pass and have been archived leaf-first:

1. inspect the complete tree diff and commit history;
2. verify every child AC and evidence record from the corresponding archived
   artifacts under `.trellis/tasks/archive/`;
3. rerun or confirm combined `git diff --check`, desktop gates, and `just ci`;
4. confirm no runtime path references task assets;
5. preserve/identify every pre-existing dirty path outside this task tree's
   declared change lists; and
6. report Windows direct evidence separately from macOS/Linux `UNVERIFIED`
   native appearance.

Persist this read-only review at
`.trellis/tasks/08-29-scan-resource-bounds-app-icon/acceptance.md`. The file must
record the reviewed child product commit SHAs and archived task paths; map every
parent AC clause to evidence; list each command, exit code, and evidence/log
path; identify any manual/native `UNVERIFIED`; and end with exactly one overall
`PASS` or `FAIL`. Overall `PASS` is allowed only when no completion-required
evidence is `UNVERIFIED`.

## 4. Parent closeout

After the integration review records `PASS`, keep this coordination parent
active so final integration can consume the stable `acceptance.md`. Archive it
only after `08-29-five-mode-native-integration` passes, in the approved
hierarchy. Do not push, publish, or release unless the user separately asks.
