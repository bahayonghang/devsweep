# Desktop Frontend Guidelines

> React and TypeScript guidance for the Tauri desktop application under
> `desktop/src/`. The ratatui guidance in `.trellis/spec/frontend/` does not
> apply to this webview surface.

## Scope

The desktop frontend is an untrusted presentation and interaction boundary over
Tauri commands. Rust validates plans, selections, confirmation digests, paths,
and cleanup authority. React owns workflow state, accessible controls, truthful
formatting, and immediate invalidation of stale previews.

## Guides

- [Component Guidelines](./component-guidelines.md)
- [State Management](./state-management.md)
- [Type Safety](./type-safety.md)

## Pre-Development Checklist

Before changing `desktop/src/`:

1. Read all three guides above.
2. Read the parent task's frontend flow and current Tauri command signatures.
3. Trace every changed field from core Serde output through `api/` decoding,
   reducer state, and rendering.
4. Confirm that no decision is based on a reconstructed action, invented path,
   or display string.
5. Confirm whether a dependency is production or development only. Do not add a
   production dependency without explicit approval.

## Quality Check

Run with the Node version required by `desktop/package.json`:

```powershell
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
```

Then run `just ci` at the repository root when frontend work changes shared
contracts or the Tauri workspace build.

Review the built surface at desktop and narrow viewports. Verify keyboard focus,
loading, empty, error, confirmation, and result states. Search visible copy for
claims that data was freed; the approved language is estimated recoverable
capacity and, for trash outcomes, moved to trash pending emptying.

## Completion Checklist

- [ ] Scan progress is indeterminate and displays backend phase/message only.
- [ ] Inspect-only targets cannot be selected by row, select-all, or reducer.
- [ ] Irreversible commands are explicitly visible before confirmation.
- [ ] Selection changes and rescans synchronously invalidate dry-run state.
- [ ] Execute requires a current dry-run digest and a second confirmation.
- [ ] Structured command errors have recovery-oriented copy.
- [ ] Capacity confidence remains verified, partial lower bound, or unknown.
- [ ] No visible copy says space was freed or released.
- [ ] Lint, typecheck, tests, build, and relevant Rust gates pass.

