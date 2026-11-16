# Implementation check

Reviewed on 2026-11-15 against the uncommitted task implementation on `dev`
(`faf9dc6` baseline). The approved AC6 and Command review-only decisions remain
in force. Code review and final Rust gates are complete. Native acceptance is
deferred at the user's request; this receipt does not mark the task accepted or
ready for archive.

## Findings (fixed)

### Read-only fixture could leave Clean and reach host-backed pages

- Files: `desktop-gpui/src/shell.rs`, `desktop-gpui/src/main.rs`.
- Issue: starting an unbacked Session did not isolate navigation. `view::run`
  enables background jobs, so mode navigation could reach Status sampling or
  Software inventory, and History could read the fixed audit stores. Only Clean
  scan/execution had been refused. Also, a non-Unicode fixture environment value
  could fall through `env::var` to normal startup.
- Fix: while a Clean fixture marker is present, Session refuses other modes,
  mode/menu navigation, support-menu/settings entry and AI entry. Clean row,
  group, scope and preview-refusal exercises remain available. Normal Session
  behavior is unchanged. Startup now reads `var_os` and rejects a non-Unicode
  value before normal preference loading.
- Regression: `clean_native_fixture_cannot_navigate_to_host_backed_surfaces`
  enables background jobs, exercises mouse-equivalent and keyboard navigation,
  asserts Clean/Mode remains active and no job/operation is queued, and rejects
  malformed locale/scene keys. The existing tests retain scope/root-edit,
  missing-plan and preview/execution refusal coverage. The non-Unicode branch
  was source-reviewed; it was not exercised through a native launch.

### Map figures lost capacity confidence and search scope context

- Files: `desktop-gpui/src/view.rs`, `view/page_clean.rs`, `copy.rs`.
- Issue: the new header/group totals preserved partial sizes, but the map's
  four statistical figures still used bare byte formatting. A partial estimate
  could therefore appear exact. Search changed those figures without explaining
  their narrower scope next to the figures.
- Fix: derive a confidence-preserving capacity for the mapped set and each
  disjoint review band, retaining the existing search-filtered map behavior.
  Explain in both languages that map figures follow search and group subtotals
  cover the full scan. Accepted groups and offscreen selection are unchanged.
- Regression: `clean_map_capacities_preserve_confidence_and_filtered_scope`
  checks mixed exact/partial band totals, bilingual lower-bound wording, filtered
  capacity, and unchanged full accepted capacity.

### Chinese confirmation did not identify the Recycle Bin

- File: `desktop-gpui/src/copy.rs:605`.
- Issue: the action said `确认移出`, despite R5 requiring an explicit Recycle Bin
  move. Main's native review independently identified this wording.
- Fix: `确认移入回收站`. The existing shared helper updates header/footer labels.
  The execution authorization path was not changed.

## Findings (not fixed)

No unresolved code findings were identified after the local fixes and passing
gates. Native acceptance remains unfinished, as requested by the user. In
particular, source mapping from Mixed to UIA Indeterminate is verified, but the
native tool has not exposed a numeric ToggleState value. A localized
`Partially selected` accessible name is not substituted for that observation.

The earlier root stderr test failure remains recorded in `root-gate.md`.
Its isolated rerun and the final full gate passed, without modifying root code
or weakening assertions. The timing explanation remains a supported hypothesis,
not a proven root cause or a newly declared baseline exception.

## Review coverage

- Refreshed task context and the referenced Clean review/advisory, desktop
  runtime, scan coverage and visual contracts. Read the real execution, plan,
  ranking and scan sections of the oversized quality guidelines directly.
  The new `backend/clean-review.md`, index/runtime links and code map agree with
  the checked implementation; no public-spec correction was required.
- Traced real intent from accepted targets through row eligibility, group and
  recommendation selection, keyboard/click routes, saved selection construction,
  `begin_execute` and the worker adapter. Command/InspectOnly stay visible and
  cannot enter this recycle flow; the adapter independently rejects a non-trash
  intent before validation/dispatch.
- Checked requested versus effective All scope, explicit roots, one-shot global
  consent and consent-free project-only rescans with omission wording. Progress
  uses core-owned observation snapshots and never enters plan authority.
- Checked cached accepted grouping, full-group subtotals, eligible-only tri-state,
  stable target/control IDs, bounded filtered windows, offscreen selection,
  sorting and 10,000-row paging regressions. Group controls use Checkbox +
  `aria_toggled`, without selected-item or self-active-descendant semantics.
- Checked immediate digest invalidation for row/group/recommended/clear,
  roots, protection and ignore edits, plus epoch/selection checks and independent
  core live digest validation. Checked six visible states, fixed `sheet_frame`
  footer and bilingual status/action copy.
- Checked retained full outcomes and metadata, succeeded-MoveToTrash-only
  capacity/counts, failure/skip reasons, confidence and common completion/session
  projection. The isolated public History reader test compares corresponding
  normal records; cancellation/audit cases qualify incomplete durable evidence
  rather than reconstructing missing targets or using cumulative history.

## Verification

Commands were serial; no concurrent Cargo gate was run by this reviewer.

| Check | Actual result |
| --- | --- |
| `cargo fmt --manifest-path desktop-gpui/Cargo.toml -- --check` | Pass after formatting |
| `cargo test --manifest-path desktop-gpui/Cargo.toml --locked --bin devsweep-gpui-spike clean_ -- --test-threads=1` | 64 passed; 153 filtered out; 0.80s |
| `cargo build --manifest-path desktop-gpui/Cargo.toml --locked --bin devsweep-gpui-spike` | Pass; debug rebuilt |
| `$env:RUST_TEST_THREADS='1'; just gpui` | Exit 0: format, all-target Clippy `-D warnings`, 217 tests passed / 0 failed / 0 ignored (10.68s), release build passed |
| `$env:RUST_TEST_THREADS='4'; just ci` | Exit 0: format, type-check, 863 tests passed / 0 failed / 3 ignored, Clippy `-D warnings` passed |
| `git diff --check` | Pass |

The first local formatting attempt hit Windows OS 1224 on a mapped `shell.rs`.
Formatting was applied with an atomic same-directory file replacement and the
read-only format check passed. An initial .NET replacement attempt failed on a
null backup argument; the subsequent atomic move succeeded and left no temporary
source file. The new map regression initially had a String argument mismatch,
then used a path query where the existing search indexes target IDs. These test
setup mistakes were corrected; the focused and final full gates then passed.

Full gate stdout was automatically retained at:

- `C:\Users\lyh\AppData\Local\rtk\tee\1794793061_just_gpui.log`
- `C:\Users\lyh\AppData\Local\rtk\tee\1794793162_just_ci.log`

Final debug artifact for a future copied native review window:

```text
D:\Documents\Code\Rust\Exp\devsweep\desktop-gpui\target\debug\devsweep-gpui-spike.exe
SHA256 9D731E3791C24763134615EB7AB8DC4B230C2281A3ADB98B661D36B4F0ED8981
```

No Python gate was rerun. No reviewer native automation, manual host scan,
cleanup, uninstall, update, startup action, commit, push or archive was performed.
Existing authorized test fixtures are covered by the Rust test results above;
they do not replace six-state screenshots, keyboard/UIA acceptance or a real
task-owned Recycle Bin flow. Native evidence and remaining acceptance belong to
the main session.
