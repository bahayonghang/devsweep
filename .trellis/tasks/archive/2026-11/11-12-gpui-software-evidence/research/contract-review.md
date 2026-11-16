# Step 1 independent contract review

Date: 2026-11-15. Role: `trellis-check`. Scope: research only, before the
required user review and before implementation step 2.

## Findings resolved

- `installed_on: Option<String>` does not itself validate a populated date.
  With the main session's agreement, the proposal now requires shared core
  date validation/projection for saved and injected inventories as well as
  collectors. Invalid dates display unknown and sort last. Stored fields must
  not be rewritten before the existing fingerprint check.
- UTC-boundary tests now explicitly use fixture timestamps/offsets, without
  changing the machine timezone.
- Removed an extra blank line at the end of `contract.md`, found by checking
  the untracked document separately from the tracked diff.

## Verified conclusions

- Current `SoftwareEntryV1` omits absent optional display fields and the
  inventory fingerprint serializes entries. Omitting the new absent fields is
  therefore necessary for legacy fingerprint compatibility; implementation
  still needs the fixed legacy fixture test.
- Microsoft documents MSI `InstallDate` as potentially reflecting servicing,
  and MSIX `InstalledDate` as installation or latest update. The proposed
  qualified wording and UTC conversion match those semantics.
- `Package.Logo` has the documented pre-19041 restriction. The pinned GPUI
  source accepts `Arc<RenderImage>` with BGRA pixels, so an in-memory image
  route exists; extraction and responsiveness remain unverified.
- `discover_software_leftovers` reads protection data and discovers candidates
  without opening the uninstall journal. The separate planning path requires
  a succeeded uninstall. Related-directory wording is appropriate before
  uninstall and must not imply removal authority.
- Keeping MSI-only optional metadata unknown matches this child's explicit
  ARP/MSIX collection steps. No Tauri consumer needs restoration.

## Verification and limits

Inspected current core collection/model/fingerprint/leftover/protection paths,
CLI and GPUI saved-inventory consumers, and the pinned GPUI image APIs.
Checked the primary Microsoft pages linked in `contract.md`. The implementer's
host format samples were reviewed as recorded evidence; this review did not
repeat the host probes or independently measure icon loading.

No remaining factual blocker was found for presenting step 1 to the user.
Live fixed-drive/reparse-safe reads, bounded resource decoding, and native UI
responsiveness remain implementation checks, not guarantees established here.
Tracked `git diff --check` passed. Both new documents have no whitespace-error
diagnostics under `git diff --no-index --check` (its exit 1 denotes new-file
differences); only the repository's LF-to-CRLF conversion warnings remain.
Lint, type-check, Rust tests, native acceptance, and final gates were not run
for this documentation-only checkpoint. No product code or host settings changed.
