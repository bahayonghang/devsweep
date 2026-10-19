# C09 keyboard-only and IME probe (round 4)

Date: 2026-10-18. Binary: `devsweep-gpui-spike.exe`, SHA-256
`DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34`, 14179328
bytes, built through `just gpui-build`.

Rounds 1-3 could not activate the window: `SetForegroundWindow` returned False
because Windows' foreground lock rejects a programmatic activation. Round 4
satisfies the lock the way a real user keystroke does — synthesise an ALT tap,
then call `SetForegroundWindow`. That worked.

## Foreground and real keyboard input now work

| Check | Result |
| --- | --- |
| `SetForegroundWindow` after an ALT tap | **True**, `GetForegroundWindow()` == the app hwnd |
| `SendKeys "{TAB}"` then typing `kbdonly` | the Note edit's UIA `ValuePattern` read back **`kbdonly`** |
| `WM_CLOSE` | exit code 0 |

So real OS keyboard input reaches the app and mutates application state. This is
genuine keyboard input, not a synthetic UIA invoke.

## But keyboard-only navigation does NOT reach the mode tabs

With the window genuinely foreground, three `{TAB}` presses followed by
`{RIGHT}`, another `{RIGHT}`, and the app's own `alt-down` chord left the
selected tab at `清理` every time:

```
selected_tab_initial=清理
after_tab1 selected=清理
after_tab2 selected=清理
after_tab3 selected=清理
after_RIGHT selected=清理
after_RIGHT2 selected=清理
after_ALT_DOWN selected=清理
```

The UIA tree is unchanged before and after.

### Root cause, located in source

- The mode tabs are rendered with `.role(gpui::Role::Tab)` and
  `.aria_selected(active)` (`view.rs:300-306`). Note they are `Role::Tab`, not
  `Role::TabItem`, even though UIA reports them as `ControlType.TabItem`.
- The key bindings exist and are correct: `left` -> `ModePrev`, `right` ->
  `ModeNext`, `alt-down` -> `RowDown` (`view.rs:697-708`), dispatched from the
  shell root's `.on_action(...)` handlers (`view.rs:350-358`).
- **`view.rs:753` pins keyboard focus to the Note input at startup**:
  ```rust
  // EntityInputHandler only receives characters while the Note
  // control keeps keyboard focus. Do not retarget this to the shell root.
  window.focus(&view.note.focus_handle(cx), cx);
  ```
  The shell root therefore never holds focus, so its `ShellRoot` key context is
  never the active context and none of the bound actions dispatch.

This is a real accessibility gap in the spike, not a harness limitation: a
keyboard-only user cannot change mode. It is recorded as a defect.

## UIA keyboard focus is not exposed

`HasKeyboardFocus` was **False on every node** in every run, including runs where
typing demonstrably reached the Note edit and `GetGUIThreadInfo` reported
`hwndFocus` == the app window. So the application has real keyboard focus while
the UIA tree claims nothing is focused. That is an accesskit/GPUI focus
reporting gap.

## IME composition remains unavailable

`ImmGetContext` returned NULL for the window handle and the GUI-thread focus
handle. Attempting to install a context explicitly also failed:

```
ImmCreateContext=159258379
ImmAssociateContext(hwnd) prev=0        <- previously no context
ImmGetContext_after_assoc(hwnd)=0       <- still NULL after association
```

`hwndCaret` was 0 throughout, so there is no caret window to attach a composition
to. The spike hosts no child HWND per text field; it is a single GPUI window.
Chinese IME composition therefore stays **NOT_RUN** — this is an architectural
property of the spike, not something a test can work around.

## Verdict

C09-AC2 stays **NOT_ACCEPTED**:

| Clause | State |
| --- | --- |
| Native UIA exposes labels and roles | **PASS** — 13 nodes, Window/Pane/Button/Tab/5 TabItem/Edit/2 ListItem, Chinese labels, `aria_selected` on the active tab |
| Keyboard-only operation reaches all spike controls | **FAIL** — focus is pinned to the Note input; mode tabs are unreachable by keyboard |
| IME composition works | **NOT_RUN** — no IME context and no caret HWND exists to compose into |
| UIA exposes focus | **FAIL** — `HasKeyboardFocus` is False even while real keyboard input is being delivered |

Rounds 1-3 failed to activate the window at all; round 4 fixed that and thereby
turned "could not test" into two concrete, source-located failures.
