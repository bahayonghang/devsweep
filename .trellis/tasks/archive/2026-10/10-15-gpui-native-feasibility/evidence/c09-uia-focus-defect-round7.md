# C09 UIA focus: proven to be an app defect, not the environment (round 7)

Date: 2026-10-19. Binary `devsweep-gpui-spike.exe`, SHA-256
`8A77746AED29A368068C52BA1D43ABF574329E6A09AAB6092C98E12A3DA5EC77`,
14182400 bytes.

## The control experiment

Same session, same activation method (ALT tap then `SetForegroundWindow`,
`foreground match=True` for both), same UIA API.

| | Notepad | GPUI spike |
| --- | --- | --- |
| `AutomationElement.FocusedElement` | `Text editor`, `ControlType.Document`, `hasFocus=**True**` | `DevSweep`, `ControlType.Window`, `hasFocus=False` |
| `belongs_to_app` | True | True |
| Nodes with `HasKeyboardFocus` | **5 of 37** | **0 of 13** |

So this environment **does** deliver UIA focus information — Notepad reports a
focused document and five focused nodes. The GPUI app reports none. **The UIA
focus gap is therefore a real defect in the app, not a limitation of the session.**

## It is not a stale-tree problem

Hypothesis tested: accesskit returns its initial tree with root focus on
activation, and an idle app never repaints, so the tree stays stale. Disproved:

| Action | `GetFocusedElement` | Focused nodes |
| --- | --- | --- |
| after activation | `DevSweep` / Window / False | 0 of 13 |
| after `alt-right` (forces a repaint) | `DevSweep` / Window / False | 0 of 13 |
| after a second `alt-right` | `DevSweep` / Window / False | 0 of 13 |
| after a **real mouse click on the Note** | `DevSweep` / Window / False | 0 of 13 |
| after typing `q` (edit value became `q`) | `DevSweep` / Window / False | 0 of 13 |

Real clicks and real keystrokes reach the app — the Note's value changed to `q` —
yet the tree still reports the root window as the focused element and no node as
keyboard-focused.

## Where the code decides this

The accessibility tree itself is being built and sent: 13 nodes are present, with
correct roles and labels. Focus is separate from node construction.

In GPUI (pinned rev `322cd509dcccaee1f6bc44725d270598ae8171ad`):

- `crates/gpui/src/elements/div.rs:2326-2336` — during prepaint, a div with
  `.track_focus(handle)` registers the node and then:
  ```rust
  window.a11y.set_focusable(node_id, focus_handle.id);
  if focus_handle.is_focused(window) {
      window.a11y.set_focus(node_id);
  }
  ```
- `crates/gpui/src/window/a11y.rs:233-250` — `set_focus` only records the node
  when it is already in the tree and was registered focusable that frame;
  otherwise it logs `it has an id but no role` and falls back to root focus.
- `crates/gpui/src/window.rs:3786-3810` — the tree update is only sent when
  accessibility is active at both the start and end of the frame.

The Note div does have a role (`Role::TextInput`) and does have an element id
(`note-input`), and its node appears in the tree as `ControlType.Edit` with the
name `Note`. So the node exists; the branch that never runs is
`focus_handle.is_focused(window)` returning true at prepaint time, leaving
accesskit's `focus` at `ROOT_NODE_ID`.

## Consequence

C09-AC2's "UIA exposes focus" clause stays **FAIL**, and it is now a
**reproducible app defect with a located code path** rather than an untested
claim. This also blocks the UIA halves of C11-AC4, C12-AC4, C13-AC1 and C13-AC4.

## What was ruled out

- Not an environment gap (Notepad reports focus here).
- Not a missing role or element id (the node is in the tree with both).
- Not a stale tree (repaints, clicks and typing all leave it unchanged).
- Not accessibility being disabled (13 nodes are being published).

## What a fix would need

Make the focused element's `focus_handle.is_focused(window)` true at prepaint
time, or explicitly report the focused node through the accessibility builder
(for example via the `report_active_descendant_focus` path that `div.rs` already
exposes), and verify that `GetFocusedElement` then returns the `Note` edit rather
than the root window.
