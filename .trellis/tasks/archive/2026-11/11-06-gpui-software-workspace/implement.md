# Implementation plan

## Activation gate

Read the parent final plan and obtain its explicit implementation approval before
task.py start. Parent/child links do not enforce order; observe the dependency
below. No implementation is authorized by task creation alone.

After footer/focus child; implement after Clean to reduce shared view.rs/shell.rs conflicts.

## Ordered work

1. Test transition-owned first load, same-tab no-op, reentry and stale jobs.
2. Remove duplicate cards and introduction-return state from the visible path.
3. Render one selected TabList, segment toolbar, bounded rows and context.
4. Correct capability/empty/error copy and migrate UIA locator assumptions.
5. Run generation/authority regressions, keyboard actions and native layouts.

## Verification

First inventory dispatched exactly once; same-tab no-op; explicit refresh;
segment transitions and mode return; stale load rejection; filtered-empty versus
empty; unavailable winget; protected/permission/unknown-size rows; no first-click
execution; preview invalidation; Leftovers refusal; UIA reacquisition and focus.

Run affected module tests during iteration, then just gpui-check, just gpui-test
and just gpui-build. Run root just ci for final integration as required by the
repository; it does not cover the standalone GPUI workspace. Use isolated stores
and fixtures. Parent implement.md defines the native screenshot/UIA matrix.

## Review gates

- [ ] All child acceptance criteria have direct evidence.
- [ ] Shared shell focus, search, state and safety contracts pass.
- [ ] Native rendering/UIA evidence is separate from source-level tests.
- [ ] No unrequested backend authority, dependency or schema change.
- [ ] Rollback keeps preference data intact and restores coherent UI/state.
