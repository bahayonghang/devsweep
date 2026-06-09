# Global cache providers design

## Scope

Implement global cache discovery providers for npm, pip, pnpm, Yarn, and Cargo home. Docker remains out of scope for this child.

## Boundaries

- Providers produce `CleanTarget` values only. They do not execute cleanup.
- Command-backed cleanup is represented as `CleanAction::Command` with executable path, argv, and cwd data.
- Missing tools are reported by returning no target for that provider, not by failing the whole scan.
- Global cache providers must not delete cache internals directly.
- Cargo home remains inspect-only. It may report evidence about known directories but must not mark cargo `bin`, credentials, or the whole cargo home as cleanable.

## Provider contracts

- npm:
  - Discover availability with an executable lookup.
  - Use official npm cache commands in the action plan.
  - Include evidence that identifies the cache command/provider.
- pip:
  - Discover availability with an executable lookup.
  - Use `pip cache` command plans for inspection and purge.
  - Do not assume a cache directory is cleanable by path deletion.
- pnpm:
  - Discover availability with an executable lookup.
  - Use official store/prune commands in the action plan.
- Yarn:
  - Discover availability with an executable lookup.
  - Plan version-aware official cache cleanup commands.
  - Prefer command invocation over direct cache path deletion.
- Cargo home:
  - Discover `CARGO_HOME` or default home cargo path.
  - Produce inspect-only target(s) with evidence and non-cleaning action.

## Data flow

1. A provider checks whether its tool or home directory exists.
2. It builds one or more `CleanTarget` records containing evidence, risk, reversibility, and action.
3. The existing executor receives the target action later and runs commands without shell composition.

## Safety invariants

- Every target has evidence.
- Every command action has argv split into arguments, not a shell command string.
- No provider returns a delete action for global cache internals.
- Missing optional tools are non-fatal.
- Cargo home is never returned as a destructive cleanup action.

## Validation strategy

- Unit tests use fake command resolution and fake environment/home paths.
- Tests cover available and missing-tool paths for command providers.
- Tests assert action kind and argv content rather than requiring live npm/pip/pnpm/yarn installations.
- Tests assert Cargo home avoids destructive action and forbidden paths.
