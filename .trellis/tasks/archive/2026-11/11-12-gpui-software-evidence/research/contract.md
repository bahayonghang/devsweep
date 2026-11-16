# Software evidence contract (step 1)

Research date: 2026-11-15. Baseline: dev at a425db7. This is a proposal for
the required user review, not approval to implement steps 2–7.

Review outcome, 2026-11-15: the user approved the four summarized conclusions
and authorized steps 2–7, final verification, native acceptance and archiving.
Commits remain deferred until a separate user instruction.

## 1. Decisions proposed for review

- Keep Software inventory and the CLI envelope at V1. Add only optional
  `icon: { path: String, index: i32 }` and `installed_on: "YYYY-MM-DD"`.
  Missing/invalid/unavailable evidence stays absent; never invent a date or icon.
- Use ARP's allowlisted `DisplayIcon`/`InstallDate` values, and MSIX's
  `Package.Logo()`/`Package.InstalledDate()` on the existing joined MTA.
  No installer command fields, executable discovery, directory timestamps, or
  network image lookup.
- An installation date is **not necessarily the first installation date**.
  Label it **Installed / serviced** / **安装/维护日期**. Explain source semantics:
  ARP = installer-reported date (may reflect servicing);
  MSIX = installed or last updated, normalized to UTC.
- Retain the pre-uninstall **read-only related-directory preview**. It can call
  `discover_software_leftovers` now; it must not create a plan, digest,
  selection for removal, audit record, registry backup, or trash operation.
- GPUI's pinned dependency supports in-memory bitmap display. Load icons in a
  bounded background worker; keep letter avatars until images arrive or fail.

## 2. Sources, observed formats, and parsing

### Current source ownership

| Source | Owning code today | Extension boundary |
| --- | --- | --- |
| ARP | `software/arp.rs:33–61, 110–180, 183–299`: HKCU/HKLM × Registry32/Registry64; an explicit value allowlist | Add the two read-only metadata values to `ArpRecord` and observation conversion. Bad optional metadata must not downgrade eligibility or whole-source completeness. |
| MSI | `software/msi.rs:40–113, 132–214`: current-user unmanaged/managed and machine contexts, `MsiEnumProductsExW` + bounded `MsiGetProductInfoExW` properties | Existing design specifies ARP/MSIX metadata. Keep MSI-only rows unknown in this implementation rather than inventing a cross-source identity join. |
| MSIX | `software/msix.rs:29–94, 265–403`: exact current-process SID, joined MTA, PackageManager enumeration | Read logo/date independently as optional fields; failure of either must not discard an otherwise valid package or change removal eligibility. |
| Assembly | `software/mod.rs:85–107, 206–283`; `model.rs:247–268, 344–357` | Carry metadata through internal observations and deterministic assembly. A same-identity metadata disagreement becomes unknown metadata, without creating an eligibility conflict. |

The ARP source guard at `arp.rs:647–660` currently forbids icon fields along
with command fields. Step 2 must update that obsolete icon prohibition while
retaining both command-field prohibitions. Use exact test locations when editing;
line numbers above are baseline navigation, not a substitute for re-reading.

### Read-only host sample

The probe used `Microsoft.Win32.RegistryKey.OpenBaseKey` for both views/hives,
`OpenSubKey(..., false)`, and only named `GetValue` calls for `DisplayName`,
`DisplayIcon`, and `InstallDate`, with `DoNotExpandEnvironmentNames`.
It never fetched whole property bags or installer command values. Counts below
are **registry observations**, including duplicated views, not unique apps:
634 named records; icon absent 347, comma/nonnegative-index 154, plain/other 82,
quoted 51; date eight-digit 594, absent 38, other string 2.
User-profile prefixes are redacted in these observed examples.

| Observed application / field | Raw value | Proposed result |
| --- | --- | --- |
| Antigravity / icon | `<PROFILE>\AppData\Local\Programs\antigravity\Antigravity.exe,0` | Local path, index 0 |
| ArcMD / icon | `<PROFILE>\AppData\Local\Programs\ArcMD\ArcMD.exe` | Local path, default index 0 |
| BongoCat / icon | `"<PROFILE>\AppData\Local\BongoCat\bongocat-app.exe"` | Strip one balanced outer quote pair; index 0 |
| Cubox / icon | `<PROFILE>\AppData\Local\Google\Chrome\User Data\Default\Web Applications\_crx_…\Cubox.ico` | ICO path, index 0; the app-specific directory is abbreviated here |
| act / date; icon | `20261104`; absent | `2026-11-04`; absent icon |
| BongoCat / date | `20261107` | `2026-11-07` |
| Antigravity / date | absent | Unknown |
| A HUB / date | `Tue Jun 17 16:17:16 2025` | Unknown; do not locale-parse or guess |

Negative icon indices, environment-variable icons, UNC/device paths, and invalid
Gregorian dates were **not observed** in this sample. Cover them with constructed
fixtures; do not present them as host findings.

### DisplayIcon rules

1. Bound string length; trim outside whitespace. Reject NUL/control characters.
   Accept one balanced quoted path with optional comma + signed decimal index,
   or an unquoted path optionally ending in comma + signed decimal index.
   Parse the final comma suffix only when it is unambiguously an integer; a
   quoted path can contain commas. Reject unmatched quotes, trailing arguments,
   malformed/overflowing indices and ambiguous unquoted comma forms.
2. Default index is 0; preserve negative indices. Microsoft's
   [ExtractIconExW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-extracticonexw)
   distinguishes a nonnegative ordinal from a negative resource identifier.
   Constructed cases: `"C:\Apps\Acme, Inc\app.exe",-101`,
   `C:\Apps\app.dll,+2`, overflow, `,oops`, and unbalanced quotes.
3. Minimum policy: reject unresolved `%NAME%` placeholders instead of expanding
   arbitrary environment values. The existing registry string reader accepts
   REG_SZ/REG_EXPAND_SZ but does not expand them (`arp.rs:555–573`).
4. Core records syntactically valid **local absolute drive paths** only.
   Reject UNC, URLs, relative/drive-relative paths, alternate streams and device
   namespaces. A normal `\\?\C:\...` path may be normalized to its drive spelling
   only if the implementation can prove equivalence; otherwise reject it.
   Do not stat/read the icon in the inventory parser: a missing local file is
   valid location evidence and will produce an avatar at load time.
5. GPUI independently enforces live fixed-disk, no-reparse path policy before any
   image/resource read (section 5). A drive letter alone is not proof of locality.

### Dates: exact meaning and normalization

ARP: trim surrounding whitespace, accept exactly eight ASCII digits
`YYYYMMDD` with a real Gregorian date (year 1–9999), emit `YYYY-MM-DD`.
Do not timezone-shift an installer date that contains no time zone.
Reject empty strings, `20260230`, non-leap February 29, slashes, epoch numbers,
and weekday/month-name strings. No file creation/mtime, registry-key timestamp,
or scan-time fallback.

Microsoft's [Uninstall registry mapping](https://learn.microsoft.com/en-us/windows/win32/msi/uninstall-registry-key)
and [MsiGetProductInfoExW](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msigetproductinfoexw)
say MSI-backed InstallDate can change after servicing. That is why the UI must
not promise an original installation date. A future MSI-only metadata extension
can use its existing bounded property reader, but is not needed to add guessed
values or a join in this task.

MSIX: use
[Package.InstalledDate](https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.package.installeddate?view=winrt-26100),
not the older phone-only `InstallDate`. It represents installation **or latest
update**, with a documented Visual Studio deployment caveat. Convert its
[DateTime.UniversalTime](https://learn.microsoft.com/en-us/uwp/api/windows.foundation.datetime?view=winrt-26100)
(100 ns ticks around 1601-01-01) using checked arithmetic to the **UTC civil day**.
Reject API failures, zero/sentinel or unsupported dates outside year 1–9999.
Never consult machine timezone/locale in core. Both CLI and GPUI display the
same ISO date, with an MSIX explanation **Installed / last updated (UTC)** /
**安装或最近更新（UTC）**. This may differ by one day from local Windows displays.
Test a UTC-midnight boundary with fixture timestamps and contrasting offset
values; do not change the machine timezone or other global settings.

The two optional fields carry no new source enum: the existing exact tagged
identity already distinguishes ARP/MSI/MSIX. Missing display evidence uses
**Unknown** / **未知**. No extra status/reason taxonomy is required for these
two optional decorations.

## 3. MSIX logo source and observed results

Use
[Package.Logo](https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.package.logo?view=winrt-26100)
rather than manually parsing AppxManifest.xml. The getter can fail for packages
other than the current package on Windows versions before 10.0.19041; that is
an unknown icon, not a source failure.

A read-only Windows PowerShell 5.1 WinRT probe used PackageManager and the
current SID, then inspected three exact package names. Observed:

| Package | Logo result (local file URI) | InstalledDate as projected by PowerShell |
| --- | --- | --- |
| Calculator 11.2607.0.0 | `file:///C:/Program%20Files/WindowsApps/Microsoft.WindowsCalculator_11.2607.0.0_neutral_split.scale-125_8wekyb3d8bbwe/Assets/CalculatorStoreLogo.scale-125.png` | `2026-08-29T08:12:22.9402973-04:00` |
| Store 22609.1401.13.0 | `file:///C:/Program%20Files/WindowsApps/Microsoft.WindowsStore_22609.1401.13.0_x64__8wekyb3d8bbwe/Assets/AppTiles/StoreStoreLogo.scale-125.png` | `2026-11-10T11:51:25.6489189-05:00` |
| Notepad 11.2609.11.0 | `file:///C:/Program%20Files/WindowsApps/Microsoft.WindowsNotepad_11.2609.11.0_neutral_split.scale-125_8wekyb3d8bbwe/Assets/NotepadStoreLogo.scale-125.png` | `2026-11-11T12:16:36.9916856-05:00` |

Calculator and Notepad logos are in a **resource sibling package**, not the main
package's InstalledLocation. Therefore joining a manifest path to the main
InstalledPath would miss real icons. Accept only a validated `file:` URI with
no remote authority/query/fragment; percent-decode safely into a local drive
path. Do not pass a raw URI to GPUI. An unresolved `ms-appx:`/`ms-resource:`
value becomes unknown rather than a guessed path. Revalidate the decoded path
before use; set icon index 0.

The pinned windows 0.62.2 bindings provide `Package::Logo`,
`Package::InstalledDate`, and `Foundation::DateTime::UniversalTime`;
core already enables ApplicationModel and Foundation. No WinRT feature change
is needed for these getters.

## 4. V1 compatibility and consumer changes

`SoftwareEntryV1` is closed (`model.rs:247–268`); its existing optional
display fields are omitted on serialization when absent. Follow that Software
convention, not Status's explicit-null convention:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub icon: Option<IconEvidenceV1>, // closed { path: String, index: i32 }
#[serde(default, skip_serializing_if = "Option::is_none")]
pub installed_on: Option<String>, // validated ISO Gregorian date only
```

Explicit JSON null may decode as None; serialization omits None. Version and
all existing field names/meanings stay unchanged. Date parsing/normalization
belongs to core; GPUI must not reinterpret locale-dependent date strings.

`Option<String>` alone does not validate populated dates during serde decoding.
Collectors emit only validated ISO Gregorian dates. CLI and GPUI must use one
shared core validation/projection rule for dates from saved or injected
inventories as well: malformed values display as **Unknown** / **未知** and
sort with missing dates at the end. Preserve the decoded serialized fields
until the existing fingerprint validation has run; do not silently normalize
or rewrite a saved inventory. This does not require choosing a new public date
type or helper API name at this research checkpoint.

The parent `research/status-version-strategy.md` supports coordinated V1
extensions, but its removed Tauri consumer list is historical. Here inventories
can be saved, so compatibility must also cover saved inventories:
`software/mod.rs:383–393` hashes the serialized entries. Omission of both absent
fields preserves the old canonical representation and fingerprint; test this
with a fixed legacy fixture. Present metadata participates in the existing
entry fingerprint, without becoming uninstall authority. Existing selection
plan, live-preview and audit formats are unchanged. Old binaries with
deny_unknown_fields will reject *new* populated entries: claim backward reads
by the updated binary, not forward compatibility with old binaries.

Current consumers requiring coordinated checks:

- Core model/assembly and ARP/MSIX adapters; constructors in `software/tests.rs`,
  `software/model.rs`, `software/leftovers.rs`, and `software/execution/tests.rs`.
- CLI `application/commands/software/inventory.rs:59–89, 91–130` accepts
  saved raw inventory or the V1 envelope and serializes core directly.
  `application/presentation/software.rs:22–95` plus both i18n catalogues need
  date text. Add software-specific JSON/legacy tests to `tests/cli_contract.rs`;
  its existing closed Status decoder is not a Software decoder.
- CLI fixture literals in `tests/c01_shipped_parity.rs` and TUI software tests;
  TUI consumes the shared Rust type and needs no UI redesign.
- GPUI `software_review.rs:426, 1005, 1309–1312, 1341` owns inventory projection,
  injected JSON decoding and sealing; tests build inventories in memory.
  There is no remaining Tauri decoder to update.

## 5. GPUI loading feasibility and limits

`desktop-gpui/Cargo.toml:17–18` pins Zed commit
`322cd509dcccaee1f6bc44725d270598ae8171ad`. Its checked-out
`crates/gpui/src/assets.rs:42–67` provides BGRA `RenderImage::new`;
`elements/img.rs:42–52, 107–111, 201` accepts `Arc<RenderImage>` via `img`.
This avoids URI/network loading and lets the UI use prepared pixel data.

For EXE/DLL/ICO locations, ExtractIconExW with one requested icon, followed by
bounded bitmap conversion, is feasible without launching the file. Destroy
returned HICONs; if using
[GetIconInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-geticoninfo),
also release its bitmap handles. Never use ShellExecute, shell associations,
thumbnail extensions, or ordinary LoadLibrary to load arbitrary code.
For MSIX PNGs use a bounded local decoder and convert RGBA to GPUI BGRA.

Before any read, reject unsafe lexical paths, check
[GetDriveTypeW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdrivetypew)
for DRIVE_FIXED, and reject reparse points in **every component**, including
junctions/mount points. Inspect the drive mapping itself so a local-looking
alias to a remote target cannot pass. A final canonical path check alone
happens too late to satisfy “no network access.” Reuse existing no-follow
helpers where they meet that pre-read requirement. Keep unavailable paths as
avatars; do not repair paths or request elevation.

Proposed small bounds: one background loader, at most 256 cached success/failure
entries keyed by software ID plus current evidence, 64×64 BGRA output, capped
input bytes/pixel dimensions, and a bounded pending set for visible rows.
Refresh invalidates changed evidence and ignores old-generation deliveries.
Text renders before loading is scheduled; loading never happens in render.
These are implementation recommendations to verify in step 5, not measured
latency guarantees. No icon extraction or GPUI native rendering ran in step 1.

## 6. Read-only directory preview and implementation checks

`leftovers.rs:493–505, 549–604` validates exact selected IDs, rejects duplicates,
and discovers paths; it does **not** require an uninstall audit.
`LeftoverEnvironment::production:354–406` reads the existing protection store;
`UserProtectionList::load:259–278` returns an empty in-memory view if absent
without creating a file. Candidate discovery at `leftovers.rs:803–895` performs
bounded no-follow sizing and returns observations.

It may return the still-installed ARP InstallLocation and name-matched AppData,
ProgramData, or personal directories. “Certain” refers to the source/path
association, not permission to delete an installed application's files.
Show **Related directories — preview only** / **相关目录 · 仅预览** and explain
that these are candidates for review after uninstall, possibly still in use.
Respect protections and show unavailable/error states; do not present an empty
result as complete evidence of absence.

`plan_software_leftovers:508–521, 606–664` separately opens the Software journal
and requires a successful uninstall for the same identity/operation
(`require_succeeded_uninstall:749–762`). GPUI must not call this path in the
current task. The equivalent existing CLI remains
`software leftovers scan --inventory FILE --select ID`
(`application/commands/software/leftovers.rs:36–45`), with no new CLI grammar.

Required fixture/check matrix after approval:

| Area | Checks |
| --- | --- |
| Core | Observed formats; constructed quotes/commas/negative index/overflow/unsafe paths/placeholders; missing/non-string/bad dates and leap years; optional MSIX getter failures, file URI decode, UTC boundary; no command fields |
| V1/CLI | Fixed legacy JSON decode and unchanged fingerprint; populated-field round trip; null/omitted fields; malformed saved/injected dates display unknown without rewriting fingerprint inputs; unchanged envelope/old fields; English and Chinese date/unknown copy |
| GPUI | No network/relative/device/reparse/non-fixed paths; missing file/corrupt image -> avatar; bounded positive/negative cache and stale generation; text-first delivery; date sort unknown-last; source/eligibility/search composition |
| Preview | Fake filesystem/protection roots; no uninstall audit needed; no new plan/audit/backup/trash writes; installed-directory candidate wording |
| Final | Native English/Chinese evidence, `just ci`, `just gpui`, `python -m unittest discover -s tools/tests`; record the two known gitignore failures separately |

Validation performed for this document: current source and pinned dependency
inspection; read-only named registry values and three WinRT package metadata
samples; Microsoft primary documentation verification. No product code changed,
no cleanup/host mutation executed, and no code test gates claimed.

Review checkpoint: approve the two optional wire fields, the qualified date/UTC
policy, and read-only related-directory wording before step 2. Remaining
implementation risks are reparse-safe icon reads, resource decoding and measured
UI responsiveness; if a source cannot satisfy the path policy, use the avatar.
