# C09 UIA focus: works after a real focus cycle (round 8)

Date: 2026-10-19. Binary `devsweep-gpui-spike.exe` (release, `/Brepro`).

This supersedes the round-7 conclusion that the UIA focus gap was a plain app
defect. It is a **first-activation** defect with a concrete mechanism, and it
resolves once the window genuinely gains focus a second time.

## The interleaved control experiment

Same session, same activation helper (ALT tap then `SetForegroundWindow`),
measured GPUI, then Notepad, then GPUI again.

| Phase | `FocusedElement` | `hasFocus` | Focused nodes |
| --- | --- | --- | --- |
| GPUI, first activation | `DevSweep` / `ControlType.Window` | False | **0 of 13** |
| Notepad, same session | `File` / `ControlType.MenuItem` | **True** | **5 of 89** |
| GPUI, **re-activated after Notepad** | **`Note` / `ControlType.Edit`** | **True** | **1 of 13** |
| GPUI, after typing `z` (edit value `z`) | `Note` / `ControlType.Edit` | True | 1 of 13 |

`GetForegroundWindow() == hwnd` was confirmed True in every activation, so the
window really was foreground each time.

## What this means

- The environment **does** deliver UIA focus to this app. Notepad proves the
  session is healthy, and GPUI's own second measurement proves the app can report
  focus correctly.
- The failure is specific to the **first** time the window becomes foreground.
  AccessKit's `is_host_focused` flag is seeded from
  `GetForegroundWindow() == hwnd` at adapter init
  (`gpui_windows/src/window.rs:1082`), which is false for a window that is not yet
  foreground. The flag then only changes on `WM_SETFOCUS` / `WM_KILLFOCUS` /
  `WM_ACTIVATE`. A single programmatic `SetForegroundWindow` on a window that was
  already created does not always produce a fresh activation cycle, so the flag
  stays false and `accesskit_consumer::tree::Tree::focus_id()` returns `None`
  (`self.is_host_focused.then_some(self.focus)`), which suppresses focus for the
  entire tree.
- Once the window loses and regains foreground for real, the flag flips and focus
  is reported correctly and stays correct through subsequent input.

## Why round 7 read it as an app defect

Round 7 compared GPUI against Notepad but measured GPUI only once, on its first
activation, and tried repaints, clicks and typing — none of which produce a
foreground *transition*. The missing step was a genuine deactivate/reactivate
cycle, which is what a real user does by clicking another window and coming back.

## Consequence for acceptance

C09-AC2's "UIA exposes focus" clause is now **evidenced as working**: with the
window in a real foreground cycle, `FocusedElement` resolves to the `Note` edit
with `HasKeyboardFocus=True` and the tree reports 1 focused node.

Two honest limits remain:

1. The **first-activation** case still reports no focus. That is an upstream
   AccessKit/`gpui_windows` seeding behaviour, not something `desktop-gpui` can
   change, and it is a real usability wrinkle for assistive technology: a screen
   reader attached before the window first gains focus would initially see no
   focused element.
2. This does not make the app a five-mode shell, and it does not exercise UIA
   *selection* (moving a selection through assistive technology), which still
   requires the real review flows.
