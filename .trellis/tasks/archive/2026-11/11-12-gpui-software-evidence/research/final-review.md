# Whole-task independent review

Date: 2026-11-15. Role: `trellis-check`. Scope: the complete core, CLI and
standalone GPUI change, associated fixtures, executable spec, code map and
task-owned native evidence helpers. Native acceptance remains separate.

## Findings fixed

- The new inventory source filter continued filtering Updates and Startup after
  its control disappeared. Reproduced an ARP-to-Updates transition producing an
  empty list. Restricted the new source condition to Uninstall and restored the
  existing eligibility-filter control in the other segments. A regression test
  switches after each ARP/MSI/MSIX choice to both Updates and Startup.
- The inventory-only date sort leaked into other segments. Those segments now
  project it to Name and retain their existing Source/Version/Size cycle. The
  inventory source choice remains stored for returning to the inventory.
- The new common spoken-row projection described Startup's `enabled` state as
  an English version and announced irrelevant publisher/date fields. Spoken
  labels now follow each segment's existing localized visible semantics;
  regression assertions cover Chinese update versions and startup state.
- GPUI lexical path validation accepted DOS reserved-name components before
  opening them to inspect metadata. It now rejects reserved names and extension
  variants before any OS path call, including numbered and superscript COM/LPT
  forms. Only string fixtures were used; no real port/device was probed.

The reserved-name policy follows Microsoft's
[file naming rules](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file).
The earlier core/CLI DisplayIcon parser correction and its red/green evidence
are retained in `verification.md`.

Reviewer product edits are limited to `desktop-gpui/src/software_review.rs`,
`desktop-gpui/src/view/page_workbench.rs` and
`desktop-gpui/src/software_icons.rs`. No core/CLI/public API or authority change
was made during this final pass.

## Boundaries verified

- New optional metadata remains V1 and omits absent values; fixed legacy
  fingerprint tests and malformed saved-date projection preserve input bytes.
  TUI changes only supply absent constructor fields. No removed desktop was
  restored.
- The GPUI icon route performs pure lexical and drive-mapping checks before
  filesystem calls, rejects non-fixed mappings and reparse components, retains
  checked handles through extraction, and releases native icon/bitmap/DC handles
  on every exit. PNG decoding and resource dimensions/output are bounded;
  prepared pixels use GPUI's BGRA order.
- One icon job contains at most four requests. Positive and negative cache
  entries are bounded to 256; requests use the bounded painted row window.
  Refresh and inventory generations reject obsolete results. No file/image I/O
  occurs in rendering and completed visible windows do not continuously churn
  the cache.
- Related-directory work calls only core discovery in process. Manual inventory
  rows remain inspectable; selection changes cancel and reject stale results.
  Candidate wording, unavailable results and qualified sizes remain explicit.
  This route does not create an uninstall/removal plan, digest, audit writer,
  registry backup or trash operation.
- The native launcher isolates preferences and APPDATA and uses an explicit
  owned binary. Both evidence helpers locate the repository after task archive;
  the fixture checks the fixed core fingerprint and writes only ignored runtime
  data. Neither helper automates Windows controls or executes host actions.
- The new spec and code map match the owning modules and in-process boundary.

## Verification

Commands run in `desktop-gpui/`, with `RUSTFLAGS=-C link-arg=/Brepro` for Cargo
build/test checks:

- Regression test initially failed: ARP inventory filtering hid `Git.Git` after
  switching to Updates.
- `cargo test --locked --target-dir dist/goal-gpui-test -- --test-threads=4`:
  **204 passed** after the product fixes.
- The lexical-path test was rerun after adding further reserved-name fixtures:
  **1 passed**.
- `cargo clippy --locked --all-targets --target-dir dist/goal-gpui-test -- -D warnings`:
  **passed**, including type checking.
- `cargo fmt --all -- --check` and `git diff --check`: **passed**.

The root `just ci` success and Python suite's exactly two predeclared gitignore
failures are existing receipts in `verification.md`; this reviewer did not repeat
them because final-pass product changes affect GPUI only. The Python suite is
not reported as passed. No host uninstall/update/startup/termination/cleanup was
performed. No commit or archive was made by this reviewer.

## Remaining acceptance

No unresolved product finding remains from this review. Main must run canonical
`just gpui` on these final fixes, then retain native English/Chinese and dark/light
evidence for real icons, fallback, sorting/filter composition, directory preview
and responsiveness. Unit/source checks do not prove native rendering, text-first
arrival timing or smooth scrolling on the real 1,220-row inventory.

## Native follow-up: unavailable WindowsApps parents

The native evidence in `verification.md` records loaded ARP icons, but Calculator
and 1Remote MSIX logos remained avatars. Read-only probes with the original
sharing/no-follow flags failed to open the WindowsApps parent with
ERROR_ACCESS_DENIED (5), both with FILE_READ_ATTRIBUTES and with zero desired
access. The PNG was not opened or decoded.

This is consistent with the approved strict parent-handle policy and its
unavailable-path avatar fallback. It is an honest native limitation, not evidence
of successful MSIX bitmap rendering. No ACL, sharing, elevation or path-check
exception is warranted by this review. No product code changed in this follow-up;
the recorded canonical `just gpui` pass (204 tests, release SHA-256
`d824781cc18d387dad9a191916b211d63e7f6b77db2f24c59910efaa9be58e37`) still applies.
The remaining native screenshots, responsiveness and preview cases remain with
the main session. No tests were repeated for this documentation-only note.
