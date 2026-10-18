# C03 Technical design

Status: approved implementation design for Windows x64-first scope; bounded technical freeze precedes edits.

## Boundary and approach

Use the existing authoritative rule registry and intent reconstruction. New families are not loose path globs. Catalogue research must freeze exact rows before code and explicitly report candidates that cannot satisfy current no-permanent-delete policy.

## Ownership

- `crates/devsweep-core/src/rules/`
- `crates/devsweep-core/src/scan/`
- `crates/devsweep-core/src/clean_advisories/`
- `crates/devsweep-cli/src/application/commands/clean.rs`

These paths identify responsibility, not permission to refactor every file.
One owner edits a shared file at a time. Workers are not alone in this repository
and must not revert other workers' changes. Main session owns cross-child design,
spec updates and integration decisions.

## Data flow and compatibility

CLI request -> domain observation/plan -> live validation -> explicitly confirmed execution where supported -> durable audit/read-back -> typed output. Discovery never mutates system state.

Preserve existing supported schemas, command semantics, refusal and cancellation.
Version incompatible shapes and consult the parent CLI proposal. Rust native views
call shared services, rather than parsing CLI text or cloning domain logic.
Follow the latest-compatible dependency policy; any older required pin or compiler
compatibility change is documented and tested.

## Risks and rollback

Identity drift, partial observations, late worker results and broadening authority
are relevant failure cases. GPUI-specific native accessibility/IME/graphics are
unproven until C09. Native uninstaller/helper/provider behavior cannot be inferred
from fixture success alone. Keep operation-specific close/cancel semantics.

Retain existing CLI/Tauri paths while this child is developed. Roll back isolated
adapter changes without changing unrelated work or historical evidence. Installation,
publishing and Tauri retirement are not implicit consequences of passing tests.

## Technical freeze (2026-10-17)

Authority for this child: this section, `prd.md`, `implement.md`, and parent
`research/mole-014-rea.md`. The six existing Windows cache ids stay the only
trash-capable Windows cache rules. Do not reclassify them. Do not add a seventh
trash rule. Mole 014 names no shader directory, no WER directory, and no
download directory that this build can move to the Recycle Bin.

### Supported executable families

Keep these ids, families, child allowlists, and the seven-day direct-child temp
policy exactly as `rules/definitions.rs` and `rules/windows_caches.rs` implement
them today:

| Id | Family | Trash |
| --- | --- | --- |
| `windows.chrome.cache` | Chrome | exact `Cache`, `Code Cache`, `GPUCache` children of a recognized profile |
| `windows.edge.cache` | Edge | same Chromium children |
| `windows.firefox.cache` | Firefox | exact `cache2` child |
| `windows.vscode.cache` | VS Code | exact `Cache`, `Code Cache`, `GPUCache`, `CachedData` children |
| `windows.cursor.cache` | Cursor | same Electron cache children; not session or workspace storage |
| `windows.user_temp.stale` | current-user default `Temp` | individual direct entries, age at least seven days |

A missing `LOCALAPPDATA` / `APPDATA` root, or a `TEMP`/`TMP` value that is not
that default Temp, stays unavailable. Do not scan the user profile, another
account, `C:\Windows\Temp`, or a caller-supplied path as a substitute. Scan
still does not mutate. There is no CLI path argument that selects a cache root.
Generated project outputs (`target/`, `dist/`, `__pycache__/`, and the existing
generated-rule relatives) do not become targets because they are newer than
sources. A generated rule still needs its existing marker or evidence file.

### Coverage-only omissions

Global scan emits one provider-scope `ScanCoverage` row for each id below.
`status` is `unsupported`, `path` is null, `target_count` is 0, and `detail`
is the sentence in this table. These rows do not create a `CleanTarget`, do
not enter the rule registry, and do not make scan health partial by themselves.
Project-only scans do not emit them. Do not walk the named locations.

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

`windows.update.download` is the Mole 014 `SoftwareDistribution\Download` row.
That research assigns it to `maintenance.update_cache` as guidance because
permanent delete is unavailable. C06 owns that guidance command later. C03 does
not execute it and does not add a privileged helper call. Cargo home and Docker
stay forbidden and gain no coverage id and no target. Browser, design,
communication, and mail stores stay out of automatic selection. Permanent
delete stays absent.

Protection review stays the existing `clean protect` list. A protected cache
child is not a selected target. There is no new ignore command. Grouped totals
stay `ScanTotals`: `estimated_bytes` and `verified_bytes` are scan observations.
History moved-byte totals are a different field and must not be copied into
either scan total. Full report filtering means JSON keeps every coverage row;
do not add a flag that hides omissions. Privileged candidates are the omission
rows above. They are not selectable plan rows.

### Tests

Drive the production global scan and the existing scan-to-plan, preview, and
recorded trash path. Use temp fixtures and the existing Windows-cache test
seam. Do not clean this machine. Each of the six ids needs a positive fixture,
an adjacent personal-data file that stays, busy and unknown owner, reparse,
hard link or cloud placeholder, and a stale preview. Reuse tests that already
prove a case. Add only the missing case. One test plants fixture trees for the
omission ids and asserts zero targets plus the eight coverage rows. One test
shows a failed provider leaves a healthy sibling target in place and does not
report that failure as an empty success. One test shows estimated, verified,
and moved totals are distinct values. Fixture hashes before and after scan and
refused preview stay equal.

Write `.trellis/tasks/10-15-gpui-clean-coverage/coverage-manifest.md` with the
same six ids and eight omission ids. Do not check a PRD acceptance box until
the saved test log shows that case. Do not edit `desktop/`, root `Cargo.toml`,
`justfile`, `.github/`, `docs/`, or `.trellis/scripts/`. Do not commit.
