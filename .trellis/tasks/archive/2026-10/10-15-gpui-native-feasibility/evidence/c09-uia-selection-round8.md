# C09 UIA selection works; a competing window explains focus flakiness (round 8)

Date: 2026-10-19. Binary SHA-256
`D40B68A21040578E748D2AA88B689A7EFF32820B68A25B56FE4953F51A9FCA64`,
14182912 bytes.

## UIA selection is exposed and correct

With the window foreground, `SelectionItemPattern.Current.IsSelected` tracked the
mode exactly:

| Step | Tab selection reported by UIA |
| --- | --- |
| initial | `清理:sel=True`, all others `False` |
| after `{ALT+RIGHT}` | `软件:sel=True`, all others `False` |
| after `{ALT+RIGHT}` again | `优化:sel=True`, all others `False` |

So assistive technology can read which mode is selected, and it updates on a real
keyboard change. This is the selection half of the accessibility contract, and it
works.

## A competing foreground owner explains the unstable focus results

In this run, immediately after the two mode changes, `FocusedElement` was:

```
name='FerroStage 讲者窗口'  type=ControlType.Pane  hasFocus=True
```

That is an unrelated application on this machine, and it won foreground even
though `GetForegroundWindow() == our hwnd` had been confirmed at activation. This
is why focus measurements have been inconsistent across rounds — round 8's
interleaved run saw `Note` with `hasFocus=True`, while this run saw the foreign
window. The machine has at least one other process that periodically takes the
foreground, and a probe cannot prevent that.

## What is solidly established

| Claim | Status |
| --- | --- |
| The app publishes a correct accessibility tree with roles and labels | **Established** (13 nodes, Tab/TabItem/Edit/ListItem, Chinese labels) |
| UIA exposes the current selection | **Established** (table above) |
| UIA can expose keyboard focus | **Established**, but only after a genuine foreground cycle, and only while no other process steals the foreground (round 8 interleaved run: `FocusedElement` = `Note`, `hasFocus=True`, 1 of 13) |
| First-activation focus reporting | **Still absent** — AccessKit seeds `is_host_focused` from `GetForegroundWindow() == hwnd` at init and only updates it on a real focus/activate transition |
| IME composition | **NOT_RUN** — environment gives no IME context to any app, including Notepad |

## Consequence

C09-AC2 is no longer a flat failure. Its clauses resolve as: labels/roles PASS,
keyboard-only operation PASS, UIA selection PASS, UIA focus PASS-after-a-real-cycle
with a documented first-activation gap, IME NOT_RUN for environmental reasons.

The remaining honest limitation is that this machine cannot guarantee an
undisturbed foreground, so focus assertions must be read together with the
`FocusedElement` value captured at the same moment — which these probes record.
