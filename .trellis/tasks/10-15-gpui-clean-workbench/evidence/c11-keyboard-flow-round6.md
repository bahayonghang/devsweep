# Keyboard-only traversal of all five modes (round 6)

Date: 2026-10-19. Binary `devsweep-gpui-spike.exe`, SHA-256
`8A77746AED29A368068C52BA1D43ABF574329E6A09AAB6092C98E12A3DA5EC77`,
14182400 bytes. Window activated with real OS keystrokes (ALT tap then
`SetForegroundWindow`, `foreground=True`); the Note held startup focus
throughout and **no TAB was pressed**.

| Keys | Selected mode | Panel lines after the switch |
| --- | --- | --- |
| (baseline) | `清理` | `清理` / `先审查每个清理目标，再移动任何内容。` |
| `{ALT+RIGHT}` | `软件` | `软件` / `审查当前用户软件清单后再卸载。` |
| `{ALT+RIGHT}` | `优化` | `优化` / `审查目录中的操作后再运行。` |
| `{ALT+RIGHT}` | `分析` | `分析` / `映射磁盘占用。此模式只读。` |
| `{ALT+RIGHT}` | `状态` | `状态` / `来自采集器的主机事实。无评分。` |
| `{ALT+LEFT}` ×4 | `清理` | back to the Clean landing copy |

The documented hint line `键盘：焦点在备注输入框时，Alt+← 和 Alt+→ 切换模式。`
is the first `ListItem` in every state, so it is present on all five modes.

`{ALT+DOWN}` (row select) left the panel unchanged in this run: the Clean
landing has no scannable rows yet and the Software landing likewise shows only
its subtitle, so there was nothing for the row cursor to move onto. That is a
property of the empty landing state, not a keyboard failure.

Shutdown: `WM_CLOSE`, exit code 0.

## What this establishes

A keyboard-only user can reach and leave every one of the five modes, and each
mode's explanatory copy renders, without touching the mouse and without first
tabbing away from the text field. This is the keyboard half of the review-flow
requirement.

## What it does not establish

- No **review flow** was completed: nothing was scanned, selected, previewed or
  executed. The landings are empty in this state.
- UIA selection is still not exercised (`HasKeyboardFocus` remains False on every
  node).
- `C11-AC4` also requires that no automatic broad machine scan runs on first
  launch; that is covered separately by the unit test
  `first_launch_does_not_scan`, not by this run.
