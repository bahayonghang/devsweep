# C03 coverage manifest

Checked-in catalogue for the Windows x64 cleanup scan. This file does not claim
parity with Mole's advertised rule count. Mole 014 names no shader directory,
no Windows Error Reporting directory, and no download directory that this
build can move to the Recycle Bin.

Trash-capable Windows cache rules stay the six ids below. Child allowlists and
the seven-day direct-child temp policy are unchanged. No seventh trash rule
was added.

| Id | Family | Trash |
| --- | --- | --- |
| `windows.chrome.cache` | Chrome | exact `Cache`, `Code Cache`, `GPUCache` children of a recognized profile |
| `windows.edge.cache` | Edge | same Chromium children |
| `windows.firefox.cache` | Firefox | exact `cache2` child |
| `windows.vscode.cache` | VS Code | exact `Cache`, `Code Cache`, `GPUCache`, `CachedData` children |
| `windows.cursor.cache` | Cursor | same Electron cache children; not session or workspace storage |
| `windows.user_temp.stale` | current-user default `Temp` | individual direct entries, age at least seven days |

Global scan also emits these provider-scope omission rows. `status` is
`unsupported`, `path` is null, and `target_count` is 0. They are not rules,
not `CleanTarget`s, and not selectable. Project-only scans do not emit them.
The rows alone do not mark scan health partial.

| Id | Detail |
| --- | --- |
| `windows.shader.cache` | Shader cache has no closed directory in the reviewed catalogue. |
| `windows.wer.reports` | Windows error reports can contain crash memory and are not a cache target. |
| `windows.update.download` | Windows Update download cache stays guidance on maintenance.update_cache; this build does not move it. |
| `windows.delivery_optimization` | Delivery Optimization files stay guidance; pinned files are not a cache target. |
| `windows.user_downloads` | The profile Downloads folder is personal data and is not a cache target. |
| `ai.claude.cache` | Claude Code sessions, memory, settings, and credentials are outside generic cache cleanup. |
| `ai.codex.cache` | Codex sessions, memory, settings, and credentials are outside generic cache cleanup. |
| `ai.cursor.sessions` | Cursor sessions and workspace storage are outside windows.cursor.cache. |

Cargo home and Docker have no coverage id and no cleanup action. Permanent
delete stays absent. Protection review stays `clean protect`.
