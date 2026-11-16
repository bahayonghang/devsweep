# Software inventory display evidence

## 1. Scope / Trigger

Apply when changing Software inventory metadata, saved-inventory compatibility,
CLI date presentation, GPUI software icons/sort/filter, or the pre-uninstall
related-directory preview. Display evidence never grants uninstall or cleanup
authority. Software inventory remains V1.

## 2. Signatures

- `software inventory --format json` emits the existing V1 envelope and the
  core `SoftwareInventoryV1`.
- `SoftwareEntryV1.icon: Option<IconEvidenceV1>` where the closed object has
  `path: String` and `index: i32`.
- `SoftwareEntryV1.installed_on: Option<String>` carries an ISO Gregorian
  date, `YYYY-MM-DD`, when a collector has valid evidence.
- `software::validated_installed_on(Option<&str>) -> Option<&str>` validates
  dates for CLI and GPUI presentation without changing the inventory payload.
- `discover_software_leftovers(&SoftwareInventoryV1, &[String],
  Option<&Arc<FlagCancelObserver>>) -> Result<SoftwareLeftoverPreviewV1,
  SoftwareLeftoverError>` is the shared read-only discovery entrypoint.
  Equivalent CLI: `software leftovers scan --inventory FILE --select ID`.

## 3. Contracts

Both new fields use `serde(default, skip_serializing_if = "Option::is_none")`.
Absent fields and explicit null decode as `None`; serialization omits `None`.
The fixed legacy inventory fixture must retain its exact fingerprint. Populated
metadata participates in the existing inventory fingerprint but does not change
the exact selection identity or eligibility rules. This is backward readability
by the updated binary, not a promise that older closed readers accept new keys.

`software/evidence.rs` owns pure parsing and date validation. ARP reads only the
allowlisted `DisplayIcon` and `InstallDate` additions. Optional metadata failure
must not discard a valid entry or downgrade source completeness. Do not read
vendor uninstall command fields. MSI-only rows keep unknown display evidence;
do not infer cross-source matches.

DisplayIcon parsing accepts a local absolute drive path, balanced quotes and an
optional signed decimal resource index. Default index is zero; negative values
are resource identifiers. Ambiguous unquoted paths, tail arguments, overflow,
environment placeholders, UNC/URL/device/relative/ADS paths and invalid path
components produce no evidence. Core parsing performs no filesystem reads.

ARP dates accept exactly eight digits with a valid Gregorian date. MSIX uses
`Package.Logo` and `Package.InstalledDate` independently on the existing joined
MTA. Use a resolved local `file:` logo URI, including resource sibling packages;
do not concatenate a manifest path to the main package directory. Convert WinRT
date ticks to the UTC civil day with checked arithmetic. Never infer dates from
file timestamps or the current time. ARP dates may reflect servicing, and MSIX
dates may reflect the last update. Human copy says **Installed / serviced** /
**安装/维护日期**, with the MSIX UTC/update explanation.

CLI and GPUI share the core date projection. Invalid saved/injected strings
display **Unknown** / **未知** and sort with missing dates last. Preserve the
serialized fields until the existing fingerprint validation has run; rendering
must not normalize and reseal a saved inventory.

GPUI loads images on the background executor and displays in-memory pixels.
Text and fallback avatars do not wait for icons. Validate local drive mappings,
fixed-disk status and every component's no-reparse status before any resource
read. Retain required handles during the read, bound input/output and pending
work, cache failures as well as successes, and ignore obsolete deliveries.
Reject reserved DOS-name components such as `NUL.ico` or `COM1.png` in the
pure lexical gate before even opening a path; do not probe real devices in tests.
Include the documented superscript-digit and multiple-extension forms from
[Windows naming rules](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file).
An existing image can still be unavailable when a protected parent such as
WindowsApps refuses the required directory handle. Preserve the avatar; a
successful attribute-only path query does not replace the retained no-follow
parent-handle proof. Do not change ACLs or elevate to obtain a decorative icon.
Never use network image loading, shell handlers, executable launch or ordinary
`LoadLibrary` for inventory icons. Unavailable paths keep the letter avatar.

The GPUI implementation uses at most one icon job with four requests per batch,
256 cached success/failure entries, a 64 MiB file-size cap, a 512 by 512 decode
limit and at most 64 by 64 output pixels. Oversized resources use the avatar;
do not relax a limit merely to make every installed application show an icon.

Source and eligibility filters compose with search and sorting. Rows and spoken
labels use localized source names and qualified size evidence; internal IDs stay
in identity/automation fields. Unknown size is not zero.

Related-directory discovery is available before uninstall, including for a
focused manual-review entry. Describe observations as related directories or
post-uninstall candidates, possibly still in use. This UI path must not call
`plan_software_leftovers`, build a removal plan/digest, open an audit writer,
back up registry keys, or move files. Discovery errors and incomplete sizing
remain explicit. Removal authorization belongs to the separate host-actions
workflow and still requires a succeeded uninstall plus live confirmation.

## 4. Validation & Error Matrix

| Input/state | Required result |
| --- | --- |
| Missing/malformed source icon or date | Unknown decoration; identity and eligibility unchanged |
| Missing new keys in a legacy inventory | Decode successfully; original fingerprint remains valid |
| Malformed saved date string | Preserve payload; display unknown and sort last |
| Network, alias-to-network, device or reparse icon path | Reject before image/resource read; avatar |
| Missing/corrupt/oversized icon | Bounded failure cached; list remains usable |
| Refresh/selection changes during background work | Obsolete result cannot replace current evidence |
| Related-directory discovery without uninstall audit | Read-only observations allowed; no removal authority |
| Unavailable/incomplete directory discovery | Explain uncertainty; do not claim confirmed absence |

## 5. Good / Base / Bad Cases

- Good: `"C:\Apps\App, Inc\app.dll",-101` records only path and resource ID.
- Base: an application with no date/icon shows unknown date and a letter avatar.
- Bad: a repair date is labelled first installation, or viewing a directory
  candidate creates a removal plan while the application is still installed.

## 6. Tests Required

- Parser fixtures: observed quotes/indices/dates and constructed unsafe paths,
  trailing arguments, overflow, leap-year failures, URI decoding and UTC edges.
- ARP non-string optional metadata and MSIX optional getter failures do not
  change whole-source validity or uninstall eligibility.
- Fixed legacy fingerprint, populated round trip, absent/null defaults,
  unchanged V1 envelope and old fields, and both CLI human languages.
- GPUI path rejection before reads, cache limits/negative results, stale work,
  text-first delivery, unknown-last dates and composed source/eligibility/search.
  Cover reserved DOS-name components with string fixtures, never device opens.
- Related-directory fixtures prove no plan/audit/backup/trash writes and no
  succeeded-uninstall prerequisite for discovery.
- Native real-inventory evidence covers icons, fallback, sorting/filtering and
  responsiveness in English and Chinese. Run `just ci`, `just gpui`, and the
  repository Python tests; record actual failures rather than weakening gates.

## 7. Wrong vs Correct

Wrong: treat `installed_on: String` as validated because serde accepted it, or
canonicalize an arbitrary icon path after already following a network junction.

Correct: project dates through `validated_installed_on` after input validation;
reject unsafe drive mappings/components before reading icon resources, and
render unknown metadata without creating host-action authority.
