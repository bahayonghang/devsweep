# Implementation plan

## Activation gate

Read the parent final plan and obtain its explicit implementation approval before
task.py start. Parent/child links do not enforce order; observe the dependency
below. No implementation is authorized by task creation alone.

After footer/focus child. Does not depend on Clean or Software redesign.

## Ordered work

1. Prove public font enumeration and root typography with the pinned GPUI API.
2. Add isolated catalogue normalization/filter/state tests and bounded selector.
3. Connect draft, dirty, preview, save, restart and missing-family handling.
4. Add About metadata and platform URL activation with error/copy behavior.
5. Run isolated V1/V2/concurrent/save-failure fixtures and actual font/IME smoke.

## Verification

System/installed serialization; non-Latin names and duplicate aliases; 2000+
catalogue rows without truncating searchable membership; stale refresh, no
matches, empty/error; dirty/no-op Save; corrupt/future bytes; unrelated patches;
restart and missing saved family; native text and input rendering, link activation.

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
