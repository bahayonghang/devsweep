# Global cache providers implementation plan

## Success criteria

- npm, pip, pnpm, and Yarn providers emit official command-backed `CleanTarget` plans when tools are available.
- Missing npm, pip, pnpm, and Yarn tools are skipped without failing provider collection.
- Cargo home emits inspect-only evidence and never a destructive action.
- Provider tests cover available and missing-tool paths.
- Existing CI/test commands pass.

## Steps

1. Inspect current domain model and provider modules.
   - Verify: identify existing `CleanTarget`, `CleanAction`, provider traits, command runner or executable lookup helpers.
2. Read applicable backend specs.
   - Verify: backend index checklist has been followed before code edits.
3. Add or extend global provider module.
   - Verify: provider outputs preserve existing domain contracts and no shell-composed command strings are introduced.
4. Add tests with mocked executable lookup/environment paths.
   - Verify: tests cover available and missing-tool cases plus Cargo home inspect-only behavior.
5. Run formatting, lint, and tests.
   - Verify: repo-local `just ci` if available, otherwise the closest cargo fmt/clippy/test set.
6. Update task artifacts if implementation discoveries change scope.
   - Verify: PRD acceptance criteria still match the implemented behavior.

## Rollback points

- If provider integration requires broad domain changes, stop and revise `design.md` before continuing.
- If Yarn version detection cannot be modeled without live command execution, implement conservative command-backed plans and document the tradeoff in this task.
