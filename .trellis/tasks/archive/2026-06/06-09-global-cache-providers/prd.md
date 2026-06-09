# Global cache providers

## Goal

Add MVP global cache providers that discover cache locations and expose official command-backed or inspect-only cleanup actions.

## Parent Context

- Parent task: `.trellis/tasks/06-09-design-split-analysis`
- Depends on: `foundation-cli-domain-model`, `execution-engine-audit`
- Source design sections: npm, pip, Cargo home, pnpm, Yarn providers.
- User decision: Docker builder cache is deferred out of MVP.

## Requirements

- Add provider discovery for:
  - npm cache verify / clean command plan
  - pip cache dir/info/purge command plan
  - pnpm store path/prune command plan
  - Yarn cache discovery/clean command plan with version-aware behavior
  - Cargo home inspect-only target
- Prefer official commands and never delete opaque cache internals directly.
- Represent provider results as `CleanTarget` values with evidence, risk, reversible flag, and `CleanAction`.
- Include command path/argv/cwd data in plans so the executor can run without shell composition.
- Handle missing tools as non-fatal unavailable providers.
- Keep Cargo home cleanup inspect-only by default.
- Exclude Docker builder cache from this child.

## Acceptance Criteria

- [x] Provider tests or command mocks cover available and missing-tool cases.
- [x] npm, pip, pnpm, and Yarn actions use official command plans only.
- [x] Cargo home provider does not mark `bin`, credentials, or the whole cargo home as cleanable.
- [x] Global provider targets include evidence and risk level.
- [x] No provider directly deletes cache internals.
- [x] Docker is not part of this child's implementation or acceptance criteria.

## Out Of Scope

- Docker builder cache.
- Docker images, containers, volumes, or system prune.
- Advanced Cargo home component cleanup.
- Project artifact scanning.
- TUI provider screens beyond data needed by later UI work.
