# C09 keyboard navigation: root cause proven (round 5)

Date: 2026-10-18. Binary `devsweep-gpui-spike.exe`, SHA-256
`DEF83167353380A4F6380CD6EC168B62C1FAD401E85D731AADB1A824C6128B34`.

Round 4 established that real OS keystrokes reach the app (window activated by
synthesising an ALT tap before `SetForegroundWindow`; typing `kbdonly` appeared in
the Note edit's UIA value). It also found that `{TAB}`×3, `{RIGHT}`, `{RIGHT}` and
`alt-down` all failed to change the selected mode tab. This round identifies why.

## The discriminating experiment

`down` has **no** `SpikeInput` binding, but it *is* bound globally to `MenuDown`,
and with the menu closed and surface `Mode` it performs the same mode advance as
`right` (`shell.rs:663-667` handles `NavKey::Right | NavKey::Down` identically).

With the window foreground and the Note holding its startup focus, and **no TAB
pressed first**:

| Keys sent | Selected tab | Meaning |
| --- | --- | --- |
| (baseline) | `清理` | — |
| `{DOWN}` | **`软件`** | shell action **did** dispatch and advance the mode |
| `{DOWN}` | **`优化`** | advanced again |
| `{RIGHT}` | `优化` | **unchanged** — `right` did not reach the shell |

Then, to see who consumed `left`: type `xy`, press `{LEFT}`, type `Z`.
The edit read back **`xZy`** — the caret moved left inside the text field, so the
`SpikeInput` binding consumed `left` before it could reach the shell.

## Root cause

`right` and `left` are each bound **twice**:

```
KeyBinding::new("left",  ModePrev,        None),            // view.rs:698
KeyBinding::new("right", ModeNext,        None),            // view.rs:699
KeyBinding::new("left",  input::Left,     Some("SpikeInput")),  // view.rs:709
KeyBinding::new("right", input::Right,    Some("SpikeInput")),  // view.rs:710
```

The Note input owns keyboard focus from startup (`view.rs:753`), so its
`SpikeInput` key context is the most specific active context. When `right` or
`left` arrives, GPUI resolves the *more specific* `SpikeInput` binding first, the
text field consumes it as a caret move, and `ModeNext`/`ModePrev` never dispatch.

`down`/`up` have no `SpikeInput` competitor, so they reach the shell normally —
which is exactly what the table above shows.

## What this means

This is **not** a total focus failure. Shell actions do dispatch (F10 opens the
menu with five `MenuItem` nodes: `System`, `设置`, `保护`, `规则`, `历史`; `down`
changes mode). The defect is narrower and precisely located: **the mode-switch
keys are shadowed by the text input's own arrow-key bindings.**

Practical effect for a keyboard-only user: `left`/`right` never switch mode, and
`up`/`down` only do so when the menu is closed and the surface is `Mode`.
`alt-down` (row select) is likewise unreachable while the Note has focus, since
it is bound only on the shell root and no `SpikeInput`-scoped equivalent exists —
consistent with round 4.

## Correction to the round-4 write-up

Round 4 attributed the failure to `view.rs:753` pinning focus so that the
`ShellRoot` context "never becomes active". That is too strong: the context
resolution does reach the shell for keys the input does not bind. The accurate
statement is the shadowing described above. The earlier evidence files are kept
unchanged; this file supersedes their interpretation.

## Fix direction

The mode keys need a binding that wins while the Note is focused. Options, in
order of preference:

1. Add `SpikeInput`-scoped bindings for the mode actions so the Note forwards
   `alt-left`/`alt-right` (or `ctrl-left`/`ctrl-right`) to `ModePrev`/`ModeNext`,
   leaving plain arrows to caret movement. This keeps text editing intact and
   gives keyboard users an unambiguous mode chord.
2. Move mode switching off the plain arrows entirely onto a chord that the text
   field does not claim, and document it in the visible copy.

Either way the requirement is: with the Note focused, a documented chord changes
the selected mode, and plain arrow keys still move the caret.
