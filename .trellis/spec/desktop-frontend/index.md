# Desktop Frontend Guidelines

> React and TypeScript guidance for the Tauri desktop application under
> `desktop/src/`. The ratatui guidance in `.trellis/spec/frontend/` does not
> apply to this webview surface.

## Scope

The desktop frontend is an untrusted five-mode presentation and interaction
boundary over Tauri commands. Rust validates plans, selections, confirmation
digests, paths, and cleanup authority. React owns typed route composition,
mode-local workflow state, accessible controls, truthful formatting, shared
operation coordination, and immediate invalidation of stale previews. Shell
code never owns domain authority.

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
6. Confirm the mode is present in the typed registry only when its complete
   adapter is available; unavailable routes have no placeholder.
7. Trace heavy work through the coordinator's cancel-and-join lifecycle and
   prove stale completion cannot revive state.
8. Consume the core presentation tag/store and CLI-owned catalogue metadata;
   do not redefine locale precedence, keys, units, accelerators, or truncation.
9. Preserve every Clean preview/selection/digest/confirmation invariant in
   `state-management.md` when changing shell composition.

## Quality Check

Run with the Node version required by `desktop/package.json`:

```powershell
mise exec node@22 -- npm run types:generate
mise exec node@22 -- npm run lint
mise exec node@22 -- npm run typecheck
mise exec node@22 -- npm run test
mise exec node@22 -- npm run build
```

Then run `just ci` at the repository root when frontend work changes shared
contracts or the Tauri workspace build.

Review the built surface at 390, 800, 1024, and 1440 CSS pixels in English and
Simplified Chinese. Verify keyboard focus/navigation, deep-link/back, loading,
empty, error, confirmation, and result states; forced colors/high contrast;
reduced motion; locale accelerator collisions; full accessible/copyable long
data; and original brand accessibility. Search visible copy for claims that
data was freed; the approved language is estimated recoverable capacity and,
for trash outcomes, moved to trash pending emptying. Record native Windows
scaling evidence separately without changing the system scale automatically.

## Completion Checklist

- [ ] All five available primary modes and supporting destinations use one typed
      registry; unavailable features are absent.
- [ ] Route/deep-link/back, focus restore, keyboard navigation, locale changes,
      and mode-local state retention pass.
- [ ] Heavy work is cancel-requested and joined before replacement; stale events,
      close, and unmount leave zero owned work.
- [ ] Desktop/TUI use the same closed locale store and CLI-owned catalogue
      metadata; corrupt/unknown bytes are preserved and machine output is
      unaffected.
- [ ] Deep gray-green tokens, original brand, high contrast, reduced motion,
      bounded non-hero motifs, and 390/800/1024/1440 layouts pass in both locales.
- [ ] Accelerators are collision-free; binary units remain canonical; user data
      truncation preserves complete accessible/copyable text; authority/action
      copy never truncates.
- [ ] Scan progress is indeterminate and displays backend phase/message only.
- [ ] Inspect-only targets cannot be selected by row, select-all, or reducer.
- [ ] Irreversible commands are explicitly visible before confirmation.
- [ ] Selection changes and rescans synchronously invalidate dry-run state.
- [ ] Execute requires a current dry-run digest and a second confirmation.
- [ ] Structured command errors have recovery-oriented copy.
- [ ] Capacity confidence remains verified, partial lower bound, or unknown.
- [ ] No visible copy says space was freed or released.
- [ ] Lint, typecheck, tests, build, and relevant Rust gates pass.
