# C14 launch of the demand-driven release binary

Date: 2026-10-19. Binary SHA-256
`6DD3806B20E3A8A1032F73609E4BB2F98FD44F4A5C8ED35BFBB97E489D9C5C47`,
14193152 bytes. Source commit `0d691dd299fe69a827cde5b1805d2b0d0286b884`.
The binary was built before that commit from the same source and was not
rebuilt afterward.

Each run used an empty directory under the temp folder, waited until
`MainWindowHandle` was set, listed loaded module names and owned child
processes, then posted `WM_CLOSE`.

| Run | PID | Title | Modules | Module hits | Owned children | Exit |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 163916 | DevSweep | 78 | none | none | 0 |
| 2 | 98424 | DevSweep | 78 | none | none | 0 |

Module and child names were matched against `webview`, `node.dll`, `node.exe`,
and `tauri`. Both lists were empty. A packaged installer and a native action
beyond launch stay NOT_RUN. C14-AC3 stays unchecked.
