# Ratatui app experience

## Goal

Build the interactive TUI experience around the already-defined scanner, plan, executor, provider, and job boundaries.

## Parent Context

- Parent task: `.trellis/tasks/06-09-design-split-analysis`
- Depends on: `foundation-cli-domain-model`, `project-scanner-json-plan`, `execution-engine-audit`, `global-cache-providers`
- Source design sections: TUI information architecture, app state machine, scan engine, async jobs/logs.

## Requirements

- Implement the main TUI layout:
  - Dashboard
  - Global tab
  - Projects tab
  - Rules view or placeholder if rules are not yet editable
  - Jobs/Logs tab
  - Details panel
  - Confirmation modal
  - Keyboard help
- Follow a TEA-style state/update/view boundary.
- Keep render functions pure: no scanning, cleanup, command execution, or large size calculation in view code.
- Support target selection, selected totals, filtering/search, details, dry-run preview, confirmation, job progress, and cancellation.
- Surface risk, evidence, command/trash action, reversibility, and audit/job status clearly.
- Ensure irreversible command-backed cleanup requires stronger confirmation than reversible trash-backed cleanup.

## Acceptance Criteria

- [ ] TUI can render a representative app state with targets, details, and jobs/logs.
- [ ] Update tests cover key events for scan, selection, details, confirmation, clean, filter, help, and quit.
- [ ] Render path has no file-system mutation or command execution.
- [ ] Worker/job events update state without freezing the UI.
- [ ] Confirmation copy differs for trash-backed and irreversible command-backed actions.
- [ ] TUI can show Docker as unavailable/deferred only if needed by docs, not as MVP functionality.

## Out Of Scope

- Implementing scanner or executor internals.
- Docker cleanup.
- Permanent delete.
- Full release packaging.

