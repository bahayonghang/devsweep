Active task: .trellis/tasks/08-29-software-inventory-plan

CONTINUATION — complete implement.md Step 3. You are the trellis-implement sub-agent again (same repo, Windows, Git Bash shell, branch `dev`); all standing constraints apply (no new dependencies beyond the approved binding, no commit/push, protected files untouched, no parser edits, no sub-agents).

## Resolving the Step-3 stop
Your stop was correct to raise, but the boundary is resolvable within this task's own design: design.md states "The CLI contract task first creates `application/commands/mod.rs` and the empty `application/commands/software/mod.rs` boundary; this task fills only `software/inventory.rs`." The frozen-contract task created the boundary files (`commands/software/{mod.rs,inventory.rs,execution.rs}` and `presentation/software.rs`) and the grammar, but did NOT add the dispatch routing — the same situation the Analyze task resolved by adding its dispatch branch in `application/commands/mod.rs`, which the parent acceptance reviewer accepted as "a narrowly justified activation seam; it does not edit the frozen parser grammar". `application/commands/mod.rs` is the dispatch root, not the parser (`cli.rs`); adding the `software.*` routing branch is therefore in-scope activation wiring for this task, exactly mirroring the existing `clean.*`/`analyze.*` branches.

## Work items
1. In `crates/devsweep-cli/src/application/commands/mod.rs`: add `mod software;`, the `if command.starts_with("software.") { return software::run(cli, locale); }` branch (mirroring analyze), and the software route assertion — nothing else.
2. Complete `crates/devsweep-cli/src/application/commands/software/inventory.rs` per design (frozen inventory/plan handlers) if its current content is only a stub; fill `application/presentation/software.rs` rendering only if it is an empty contract stub — check both before writing; do not rewrite existing frozen content.
3. Implement.md Step 3 in full: CLI tests (`cargo test -p devsweep-cli software`), hostile fixtures, exact SID/source partials, stale/reinstall cases, locale-invariant documents, `git diff --check`, `just ci`.
4. Native standard-user evidence per implement.md (registry-view coverage, current-user MSI/MSIX identities, the all-MSI/manual gate, denied/corrupt sources, every ordered refusal row, size evidence/limits, truthful unknown last-used, no UAC, MTA worker shutdown, Cargo tree, license, binary/zip deltas). Record raw command + exit code + log for each; save under `evidence/` with a `step3-` prefix.
5. Append `implement.jsonl` journal lines.

## Report back
Final message: what was wired/implemented, every gate command + real exit code, native evidence list, deviations/blockers with exact error text.
