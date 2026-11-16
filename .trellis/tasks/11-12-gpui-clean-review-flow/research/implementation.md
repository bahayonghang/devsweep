# Clean review implementation

Implemented steps 2–8 on 2026-11-15. Scope remains GPUI only. The approved
AC6 and command/recycle decisions in the PRD/design are unchanged.

## Product changes

- `desktop-gpui/src/clean_workbench.rs`: retains typed ecosystem/kind and
  intent-derived recycle eligibility; caches whole-scope groups, subtotals,
  recommendations, eligible selection counts and tri-state. Command and
  InspectOnly findings share a nonselectable review-only group. Selection,
  group, recommendation, clear, protection and ignore edits invalidate preview.
  The selected plan and worker adapter independently refuse non-trash intents.
- The same model now retains the complete execution report and a cached result
  projection/index. Only succeeded MoveToTrash outcomes count as moved data.
  Partial/unknown estimates retain lower-bound qualification. Failure and skip
  reasons come from their outcomes. Cancellation/audit diagnostics qualify
  durable evidence; live/session totals share this projection. No new audit
  format or reconstructed history entries were introduced.
- `shell.rs`: All carries requested scope separately from effective scope.
  Unconfirmed All scans only explicit project roots; explicit one-shot global
  consent enables both flags. Rescan never reuses that consent. An accepted
  current scan clears the old scan-root stale notice; merely changing roots
  still invalidates it. Worker progress carries only core-owned display-safe
  observations, never execution authority.
- `view/page_clean.rs`, `view.rs`: distinct scanned overview/review/preview UI,
  state-appropriate header action, grouped bounded rows and full-group totals,
  fixed footer, paged grouped completion details, qualified capacity and data
  source explanations. Group headers are directly focusable CheckBox controls
  with stable accessibility IDs, localized none/partial/all labels, selected /
  eligible counts and true `aria_toggled` state. Native UIA evidence is separate.
- `clean_map.rs` reuses cached review group identities supplied by the
  workbench; existing map geometry remains a display projection. `ui.rs` adds a
  decorative mixed glyph while preserving the existing boolean checkbox API.
- `copy.rs` supplies English/Chinese text. Preview is described as read-only;
  actual confirmation is described as moving the selection to the Recycle Bin.
  Unscanned headers contain no numeric capacity/folder count. Display paths use
  the existing normalization helper.

No core/CLI files, cleanup rules, provider authority, Tauri code or persistent
formats changed. The desktop does not launch `devsweep.exe`.

## Read-only native scenes

The debug binary is:

```text
desktop-gpui/target/debug/devsweep-gpui-spike.exe
```

For explicit interactive review, launch it in a normal visible window with the
child-process environment variable:

```text
DEVSWEEP_CLEAN_REVIEW_FIXTURE=en:review
```

Locales are `en` and `zh`. Scene names are `landing`, `scanning`, `scanned`,
`review`, `previewed`, `complete`, `mixed`, `canceled`, `audit-failed`.
`en:review` / `zh:review` initially selects one of three eligible rows, exposing
Mixed; the fourth row is a disabled command finding in review-only. Group click
or Space can toggle eligible selection without creating authority.

`main.rs` uses an unbacked Session for these explicit scenes. They load fixed
in-memory observations/results, no preference files and **no cleanup plan**.
The page always identifies synthetic observations/results and disabled execution.
Scan dispatch, preview-plan construction and execution refuse these scenes even
after scope/root changes. The `previewed` scene says it has no executable digest;
it is not evidence of a real preview/confirmation chain. No synthetic report is
written to durable audit. Unknown scene/locale values do not launch a scene.

Normal startup must omit this variable. Use task-owned Python projects with a
`pyproject.toml` marker and nonempty `.pytest_cache`, `.mypy_cache`, `.ruff_cache`
folders. Save that explicit root, choose Project, scan, Review results, select
rows/group, and preview. New files may be too recent for recommendations, so
manual selection is expected. Isolate child-process APPDATA/LOCALAPPDATA to
task-owned stores. Actual Recycle Bin confirmation remains the real saved
snapshot + live digest + explicit confirmation path; only deliberately created
fixture directories may be moved. Do not confirm All against host providers.

## Verification actually run

- `cargo fmt --manifest-path desktop-gpui/Cargo.toml -- --check`: passed.
- `cargo test --manifest-path desktop-gpui/Cargo.toml --locked --bin
  devsweep-gpui-spike -- --test-threads=1`: **215 passed**, latest full run 9.78s.
- `cargo clippy --manifest-path desktop-gpui/Cargo.toml --locked --all-targets
  -- -D warnings`: passed.
- Debug build: passed; refreshed after native feedback.
- `git diff --check`: passed.

New regression coverage includes disjoint totals, empty groups, tri-state and
blocked members, review-only refusal, All flags/roots/fresh consent/rescan,
every selection invalidation, malicious command-plan rejection, mixed results,
confidence, cancellation/audit qualification and native-scene refusal. A test
subprocess reads a deliberately isolated Clean V1 store through public
`history::show_history`, comparing corresponding normal recorded counts/bytes
with the live result projection without mutating the parent test environment.
Existing search, offscreen selection, 10,000-row paging and keyboard regressions
remain covered. The new accessibility source contract protects real Toggle
semantics; it does not substitute for native UIA verification.

Some source files were temporarily memory-mapped by another Windows reader,
which made in-place rustfmt writes fail with OS 1224. Formatting was applied
from rustfmt output by an atomic replacement of only the affected source files;
the canonical read-only format check then passed.

Final `just ci`, `just gpui`, Python gates, native screenshots/keyboard/UIA and
any real task-fixture Recycle Bin acceptance are owned by the main session.
They are not claimed by these targeted checks. No commit, push or archive was
performed by this implementer.
