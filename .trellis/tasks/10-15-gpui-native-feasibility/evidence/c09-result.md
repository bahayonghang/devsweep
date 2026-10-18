# C09 native evidence

Date: 2026-10-17. Spike binary: `desktop-gpui/target/release/devsweep-gpui-spike.exe`.

| Launch | PID | Creation time | Window | Shutdown |
| --- | --- | --- | --- | --- |
| 1 | 62380 | 2026-10-17T03:14:42.8675000-04:00 | `Zed::Window`, title `DevSweep native spike`, hwnd seen in 513 ms | `WM_CLOSE`, exit code 0 |
| 2 | 74992 | 2026-10-17T03:14:51.8336260-04:00 | `Zed::Window`, title `DevSweep native spike`, hwnd seen in 453 ms | `WM_CLOSE`, exit code 0 |

Working directory for both launches: `C:\Users\lyh\AppData\Local\Temp\c09-unrelated-launch` (empty). SHA-256 of the launched binary: `493B826857B6B9BC9349ACB6C30AB579EBAD0964117C0A040EA72888A00569B9`. The file contains the packaged sentence and the PNG signature. Root `Cargo.lock` SHA-256 stayed `5BA0DF4FBFE12B32628E5F11AE277C59C80CBDFF8F5E3AD221C4927BE941532A`.

Launch 1 idle sample, about 2 seconds after the window appeared: working set 76722176 bytes, private bytes 105402368, CPU 0.328125 seconds. Neither process had a child process. Loaded module names did not include WebView, Node, or Tauri.

Screenshot: `c09-launch-1.png`. It shows the copied DevSweep icon, the packaged English sentence, the packaged Chinese sentence, the text field, and the Acknowledge button. PrintWindow returned a 25297-byte PNG with 43 sampled colors.

## UIA

`c09-uia-before.txt` lists a window named `DevSweep native spike`, an image named `DevSweep`, an edit named `Note`, a button named `Acknowledge note`, and a status named `Idle`. Two text nodes have empty names. `HasKeyboardFocus` was false on every node in that dump.

A later foreground probe (`c09-uia-focused.txt`) still reported `HasKeyboardFocus=False` on every node while `GetForegroundWindow` and `GetGUIThreadInfo.hwndFocus` were the Zed window. UIA focus was not demonstrated.

## Keyboard

The first keybd_event attempt did not change the status because the Zed window was not foreground. After `SetForegroundWindow` succeeded, Tab then Enter changed the status to `Acknowledged:` (`c09-uia-probe-keys.txt`). That is one keyboard-only action. It is not a UIA focus proof.

## IME

The foreground thread layout was `0x0804` (zh-CN). Installed languages were `en-US` and `zh-Hans-CN`. The host language list was not changed. `ImmGetContext` returned null for the Zed window, the GUI-thread focus hwnd, the caret hwnd, and every child hwnd. No composition string was read. Chinese IME composition is NOT_RUN.

## Not claimed

C09-AC2 and C09-AC4 are not passes. Rust 1.88.0 was not installed for this run and was not downloaded. The compiler that linked the spike was rustc 1.99.0.

`cargo test --manifest-path desktop-gpui/Cargo.toml --release --locked` exited 0 (7 tests). `just ci` exited 0 for the existing workspace. `desktop-gpui` is not a workspace member, so `just ci` does not build it. The native gate is the separate locked release build, which exited 0. A second clean rebuild was not compared for bit-identical output. The dependency pin is the locked revision, not a floating branch.
