# C09 UIA / IME / keyboard probe (round 3)

Date: 2026-10-18. Binary: devsweep-gpui-spike.exe
SHA-256: D1E778D32CAE1E7D0AB99BD8FFCEAEA1090A4AB2CA2F6CDE424C7D20E90EE061
Bytes: 14179328
Launched from an empty temp directory (unrelated CWD).

## UIA tree (ControlViewWalker, 13 nodes)

window_name=DevSweep  window_class=Zed::Window

    name=DevSweep                    type=ControlType.Window
    name=DevSweep                    type=ControlType.Pane
    name=DevSweep 菜单               type=ControlType.Button
    name=模式                        type=ControlType.Tab
    name=清理                        type=ControlType.Pane
    name=Note                        type=ControlType.Edit
    name=清理                        type=ControlType.TabItem
    name=软件                        type=ControlType.TabItem
    name=优化                        type=ControlType.TabItem
    name=分析                        type=ControlType.TabItem
    name=状态                        type=ControlType.TabItem
    name=清理                        type=ControlType.ListItem
    name=先审查每个清理目标，再移动任何内容。  type=ControlType.ListItem

Labels and roles are exposed for every control: the mode capsule is a Tab with
five TabItems, the beaver menu is a Button, the note field is an Edit, and the
panel lines are ListItems carrying their own text as the accessible name.

## Focus

HasKeyboardFocus was False on every node, and GetGUIThreadInfo reported
hwndFocus=530102 == the Zed window handle. SetForegroundWindow returned False
(foreground was 528268), so no keystroke reached the window in this run.
UIA focus is NOT demonstrated: the toolkit sets keyboard focus at the window
level while exposing no focused descendant element.

## IME

GetGUIThreadInfo succeeded (hwndFocus=530102, hwndCaret=0). ImmGetContext
returned NULL for the window handle and for the GUI-thread focus handle, so no
composition string could be read. Chinese IME composition is NOT_RUN.

## Shutdown

WM_CLOSE -> process exited within the 15 s wait, exit code 0.

## Verdict

C09-AC2 remains NOT_ACCEPTED. Labels and roles are now evidenced (an
improvement over the previous dump, which lacked the Tab/TabItem/ListItem
roles), but keyboard focus, keyboard-only operation and IME composition are
still NOT_RUN.
