# C09 keyboard mode chord: fixed and verified natively (round 6)

Date: 2026-10-18/19. Binary `devsweep-gpui-spike.exe`, SHA-256
`8A77746AED29A368068C52BA1D43ABF574329E6A09AAB6092C98E12A3DA5EC77`,
14182400 bytes, built through `just gpui-build` (`/Brepro`).

## The defect that was fixed

`left`/`right` were bound twice: globally to `ModePrev`/`ModeNext`, and to
`input::Left`/`input::Right` with the `SpikeInput` predicate. The Note holds
focus from startup (`view.rs:753`), so the more specific `SpikeInput` context
won and the caret consumed the arrow key. A keyboard-only user could not change
mode with the arrows.

## The fix

```rust
KeyBinding::new("alt-left",  ModePrev, Some("SpikeInput")),
KeyBinding::new("alt-right", ModeNext, Some("SpikeInput")),
```

The shell root already listens for `ModePrev`/`ModeNext` and the Note sits on its
action dispatch path, so the chord bubbles to the existing handlers. Plain
`left`/`right` keep their `SpikeInput` caret bindings unchanged.

The chord is also documented in the visible copy, in both locales:

```
键盘：焦点在备注输入框时，Alt+← 和 Alt+→ 切换模式。
Keyboard: Alt+Left and Alt+Right switch mode while the Note has focus.
```

`copy::mode_keys_hint` renders as the first line of `panel_lines`, so it appears
in the app and is asserted by `mode_keys_hint_renders_in_every_panel`.

## Native verification

Window activated (ALT tap then `SetForegroundWindow`, `foreground=True`), Note
holding its startup focus, **no TAB pressed first**, real OS keystrokes:

| Keys sent | Selected mode |
| --- | --- |
| (baseline) | `清理` |
| `{ALT+RIGHT}` | **`软件`** |
| `{ALT+RIGHT}` | **`优化`** |
| `{ALT+LEFT}` | **`软件`** |

The hint line is present in the accessibility tree as a `ListItem`:
`键盘：焦点在备注输入框时，Alt+← 和 Alt+→ 切换模式。`

**Caret regression check:** typing `xy`, then `{LEFT}`, then `Z` still reads
back **`xZy`**, and the selected mode stayed on `软件` throughout — plain arrows
move the caret and do not change mode.

Shutdown: `WM_CLOSE`, exit code 0.

## What this closes and what it does not

This closes the keyboard-navigation half of **C09-AC2**: a keyboard-only user can
now change mode while the text field holds focus, with a documented chord.

Still open in C09-AC2:

- **UIA focus reporting.** `HasKeyboardFocus` remains False on every node even
  while real keystrokes are delivered. Not fixed.
- **IME composition.** NOT_RUN, and now proven to be an environment limitation
  rather than an app defect: Notepad, with a native edit control and a real
  focused child HWND, also gets a NULL `ImmGetContext` in this session. See
  `c09-ime-environment-round5.md`.

So C09-AC2 stays NOT_ACCEPTED, but its keyboard clause moved from **FAIL** to
**PASS**.
