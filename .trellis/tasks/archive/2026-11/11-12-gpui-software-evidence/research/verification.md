# Software evidence verification

Date: 2026-11-15. Branch: `dev`. Starting HEAD: `a425db7`.
The user approved the step 1 contract before implementation. Implementation
and archive were initially held without commits at the user's request. The
user subsequently authorized local commits, with no push. Product commit:
`e8336dcaf3da5de54e85f718292a8f783a911ed4`.

## Core and CLI implementation / independent review

`trellis-implement` completed steps 2–3, then `trellis-check` independently
reviewed the core/CLI scope. The reviewer reproduced an unquoted DisplayIcon
tail-argument parsing failure (`C:\Apps\app.exe --flag=x.txt`) before fixing
the boundary and adding regression cases. Legal spaces, quoted commas and
version dots remain supported. No public API changed during review.

Verified after that fix:

- Core Software tests: 88 passed, 2 ignored. The existing ignored tests are
  `profile_msix_inventory_phases` and `profile_msix_metadata_calls`, explicit
  read-only profiling diagnostics.
- CLI Software unit tests: 22 passed.
- CLI `cli_contract` integration suite: 15 passed.
- Core/CLI all-targets `cargo check` and Clippy with `-D warnings`: passed.
- Root format check and tracked `git diff --check`: passed.

Coverage includes valid/missing/malformed source metadata, optional getter
failures, strict dates/UTC projection, fixed legacy fingerprint, populated V1
round trip, null/omitted fields, saved malformed-date presentation, unchanged
old fields and bilingual human output. These checks use fixtures, not real
uninstall/update/startup/termination/cleanup actions.

## GPUI and final gates

The GPUI implementer completed steps 5–7. From `desktop-gpui/`:

- `cargo test --locked --target-dir dist/goal-gpui-test -- --test-threads=4`:
  203 passed.
- `cargo clippy --locked --all-targets --target-dir dist/goal-gpui-test -- -D warnings`:
  passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.

Coverage includes positive/negative cache limits, unsafe/missing/corrupt icon
paths, intermediate/final reparse points, one icon job and obsolete-generation
delivery, unknown-last dates without payload rewriting, publisher search with
source/eligibility filters, manual ARP read-only directory preview, selection
change cancellation, and preserved Updates/Startup sorting.

Whole-task independent review is retained in `final-review.md`. It fixed source
filter/date-sort leakage between segments, restored the non-inventory filter
control, corrected segment-specific spoken labels, and rejected DOS reserved
path components before OS calls. A cross-segment regression first failed and
then passed. Final GPUI tests: 204 passed, plus a focused reserved-name fixture
rerun. All-target Clippy/type checking, format and diff checks passed. Reviewer
edits were GPUI-only, so the existing root gate remained valid.

Canonical GPUI gate and native acceptance are recorded below when completed.
Unit results alone do not establish native icon-loading or UI responsiveness
acceptance.

### Root gate

`RUST_TEST_THREADS=4; just ci` passed, exit 0. This uses the same bounded
parallelism described by the repository's `test-stable` recipe; no test or
assertion was skipped to pass the gate. Format, all-target check, workspace
tests and Clippy passed. No provider-authority timeout or retry occurred.
The full root suite retains three existing explicit native diagnostic ignores:
the Settings ShellExecute probe and the two MSIX profiling diagnostics.

### Python tool tests

`python -m unittest discover -s tools/tests` ran 23 tests and exited 1 with
exactly the two predeclared gitignore subtest failures:

- `target/debug/devsweep.exe`: Git exits 128 because `target` is a symbolic link.
- `.trellis/hooks.local.json`: no repository ignore rule, Git exits 1.

No additional Python failure occurred. These baseline failures remain visible;
the full Python suite is **not** reported as passed. No unrelated ignore policy
or test was changed to conceal them.

### Read-only current-machine CLI sanity

The tested debug CLI ran `software inventory --format json --output` against
the real host, exited 0, and emitted V1 `software.inventory/succeeded` with
1,220 entries, 476 icon locations and 786 dates. The raw inventory stays under
ignored `evidence/runtime/`. ISO dates and the closed icon object shape were
checked without printing the inventory. No vendor command fields appeared.
This proves metadata collection, not successful loading of every icon.

The matching `software leftovers scan` CLI was also exercised with the owned
missing-icon inventory fixture and one exact-name directory under isolated
LOCALAPPDATA. It exited 0 with one candidate. The sentinel hash stayed unchanged
and no audit directory was created. This is fixture discovery evidence, not a
real uninstall or proof that a candidate is safe to remove.

### AC3 and scope source checks

Searching `desktop-gpui/src` for `Command::new`, `std::process::Command`,
`CreateProcess`, `ShellExecute` and `LoadLibrary` found only the pre-existing
`process_spawn.rs` helper. Its callsites are in existing test modules, while
production Analyze reveal delegates `explorer.exe /select,` to core. Neither
path changed. No GPUI `devsweep.exe` spawn was added. The new image dependency
pins the already-locked image version and enables PNG only; no dependency
version was upgraded. The repository root
gitignore and `tools/tests` have no diff.

### Canonical GPUI gate

`RUST_TEST_THREADS=4; just gpui` passed, exit 0: format, all-target Clippy,
204 tests and the locked release build. Release SHA-256:
`d824781cc18d387dad9a191916b211d63e7f6b77db2f24c59910efaa9be58e37`.

Native follow-up did not modify product code: ARP icons loaded, while Calculator
and 1Remote MSIX logos stayed as avatars. A read-only probe located the refusal
at opening the WindowsApps parent directory, ERROR_ACCESS_DENIED (5), before
opening/decoding PNG data. Both original FILE_READ_ATTRIBUTES and metadata-only
zero desired access failed with the same sharing/no-follow flags. The approved
strict parent-handle policy is retained; no ACL, sharing, elevation or path-check
exception was introduced. This MSIX icon limitation is recorded in native
acceptance rather than being claimed as successful image loading.

### Native acceptance

Completed the four real-inventory language/theme cases, both sort/filter
language cases and English/Chinese isolated related-directory/fallback cases.
The ten referenced screenshots and exact observation/coverage limits are in
`../evidence/native-acceptance.md`. Both fixture previews kept their file sets
and hashes unchanged and created no audit directory. The owned windows exited
normally. No product code changed after canonical gates.
