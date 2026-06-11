# Optimize TUI visual hierarchy and layout

## Goal

Improve the existing `devsweep tui` visual hierarchy, layout rhythm, and
terminal-size behavior without changing cleanup semantics. The TUI should feel
like a dense, calm developer tool: fast to scan, keyboard-first, explicit about
risk, and readable on common Windows terminal sizes.

## Confirmed Facts

- `devsweep` is a Rust terminal UI built with `ratatui`, not a web frontend.
- Current TUI code lives in `src/tui.rs` and already includes app state,
  update/effect handling, render helpers, overlays, footer actions, and
  `TestBackend` tests.
- Current information architecture has five views: Dashboard, Global,
  Projects, Rules, and Jobs/Logs.
- Current Dashboard/Global/Projects body uses a fixed three-column layout:
  categories, targets, and details.
- Current header, tabs, body, overlays, and footer are pure render paths fed by
  `App` state.
- Recent archived tasks already improved confirmation copy, path display,
  footer action pills, and cleanup progress. This task must not duplicate those
  behavior changes.
- Project safety rules require cleanup to stay dry-run/explicit, permanent
  delete to remain disabled, and render code to avoid scanner/executor side
  effects.

## Design Read

Use `design-taste-frontend` as an audit lens only. The skill is primarily for
web landing pages and redesigns, so the applicable parts are: audit first,
avoid decorative noise, use consistent tokens, keep content scannable, and make
states complete. The terminal-specific read is:

- Surface: terminal product UI for developer cleanup planning.
- Audience: developers using keyboard workflows in Windows/PowerShell and
  cross-platform terminals.
- Vibe: serious devtool, safety-first, dense but calm.
- Adapted dials: `DESIGN_VARIANCE 4`, `MOTION_INTENSITY 1`,
  `VISUAL_DENSITY 7`.

## Requirements

- Preserve the existing five-view information architecture unless a later
  product decision explicitly changes it.
- Keep render functions deterministic and free of filesystem, process, cleanup,
  or expensive size-estimation side effects.
- Introduce or tighten a small semantic visual system for the TUI, such as
  surface, border, selected row, muted text, accent, warning, danger, and risk
  styles.
- Improve the target list so selection state, risk, estimated size, and target
  identity are stable and easy to scan at representative terminal widths.
- Add responsive layout behavior for wide, medium, and narrow terminals instead
  of assuming the current fixed three-column body always fits.
- Keep risk and cleanup strength readable without relying only on color.
- Keep the contextual footer one row where possible and width-aware when the
  terminal cannot fit every action.
- Improve empty, loading, error, confirmation, progress, jobs/logs, and details
  presentation only where needed for consistency with the visual hierarchy.
- Add or update `TestBackend` coverage for representative terminal sizes and
  key UI states.
- Keep the canonical gate green with `just ci`.

## Acceptance Criteria

- [ ] Dashboard, Global, Projects, Rules, and Jobs/Logs remain available with
      their existing keyboard navigation.
- [ ] Main TUI render tests cover at least a wide size, a medium size, and a
      narrow-but-supported size.
- [ ] The main target list includes stable visible labels or columns for
      selected state, risk, size, and target identity.
- [ ] The selected row is distinguishable through text/shape and color, not
      background color alone.
- [ ] Risk levels remain text-visible as Low, Medium, High, or Dangerous.
- [ ] Header and footer do not obscure critical state or key actions at the
      supported sizes.
- [ ] Overlays still render for help, details, dry-run, confirmation, and
      cleanup progress at supported sizes.
- [ ] No scanner, provider, executor, cleanup-plan serialization, audit JSONL,
      permanent-delete, or default-selection safety behavior changes are
      required for this task.
- [ ] `cargo test tui::tests` passes during iteration.
- [ ] `just ci` passes before implementation is reported complete.

## Out of Scope

- New cleanup providers or target discovery rules.
- Changes to cleanup action semantics, executor behavior, audit records, or
  permanent-delete policy.
- A web UI, dashboard framework, image assets, icons, animations, or live
  browser work.
- A full module split unless the implementation proves that a small `src/tui/`
  split removes real complexity.
- Manual live terminal smoke testing by default. Prefer headless `TestBackend`
  coverage; run live TUI only if the implementation needs visual confirmation.

## Product Decision

- Supported terminal size target: full layout at `100x28` and graceful degraded
  layout at `80x24`.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
