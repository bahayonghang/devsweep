# C09 IME: the session has no IME context at all (round 5)

Date: 2026-10-18. This establishes whether the GPUI app's NULL `ImmGetContext`
is an app defect or an environment limitation. It is the latter.

## The system does have IMEs installed

```
keyboard_layout_count=2
  layout[0] hkl=0x08040804 langid=0x0804 ImmIsIME=True     (zh-CN)
  layout[1] hkl=0x04090409 langid=0x0409 ImmIsIME=True     (en-US)
```

Both installed layouts report `ImmIsIME = True`, so the machine is not missing
IME support.

## But a plain Win32 app cannot get an IME context either

Baseline: Notepad (`Untitled - Notepad`), activated the same way as the GPUI
probe (ALT tap, then `SetForegroundWindow`), with a real focused child edit
window.

| Probe | Notepad | GPUI spike |
| --- | --- | --- |
| Thread keyboard layout | `0x08040804`, `ImmIsIME=True` | `0x08040804`, `ImmIsIME=True` |
| `hwndFocus` | `1708248` (a real child edit control) | the top-level window |
| `ImmGetContext(window)` | **0** | **0** |
| `ImmGetContext(hwndFocus)` | **0** | **0** |
| `hwndCaret` | 0 | 0 |

Notepad is a mature, fully IME-capable Win32 application with a dedicated child
edit control and a proper focus HWND — and it **also** returns a NULL IME context
in this session. So the NULL result is a property of this environment (no
interactive IME/TSF context is available to these processes), not a defect in
`desktop-gpui`.

## Consequence

Chinese IME composition **cannot be tested in this session** for any
application, so C09-AC2's IME clause stays **NOT_RUN**. It is an environment
capability gap, not something a code change can fix, and it must not be recorded
as PASS on the strength of the GPUI app alone.

This also means the earlier rounds' NULL readings should not have been read as
evidence against the GPUI text field. The GPUI app differs from Notepad in one
real way — it has no child caret HWND (`hwndFocus` is the top-level window,
`hwndCaret` is 0) because GPUI renders text itself rather than hosting a native
edit control. That architectural difference is real and worth noting, but it is
not what produced the NULL context here, since Notepad with a native edit control
gets NULL too.

## Reproduce

Launch Notepad, activate it, then call `ImmGetContext` on its main window and on
`GetGUIThreadInfo(...).hwndFocus`. Both return 0 in this session.
