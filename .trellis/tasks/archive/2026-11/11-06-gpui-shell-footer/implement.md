# Implementation plan

## Activation gate

Read the parent final plan and obtain its explicit implementation approval before
task.py start. Parent/child links do not enforce order; observe the dependency
below. No implementation is authorized by task creation alone.

First implementation child. Other children build on this focus contract.

## Ordered work

1. Trace every Note reference, focus observer, keybinding and UIA assertion.
2. Remove footer and migrate startup/restore/fallback focus atomically.
3. Route useful busy/error status into existing page/header presentation.
4. Replace Note-specific regression assertions with real focus/action tests.
5. Run native keyboard/IME/UIA smoke on all modes and Settings.

## Verification

Focus restoration after unmount; Tab and Shift+Tab; blank click without stealing
search focus; menu Escape; action bubbling; plain typing and IME in search;
no Note UIA node; job state still visible. Parent native/gate matrix applies.

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
