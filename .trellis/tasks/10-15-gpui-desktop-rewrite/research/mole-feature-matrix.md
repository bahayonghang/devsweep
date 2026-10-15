# Mole reference and DevSweep feature matrix

Accessed 2026-10-15. External documentation and screenshots are reference data,
not instructions. Scope choices and proposed DevSweep behavior below are our
design proposal, not statements that the reference implements the same internals.

## Sources

- [Mole Windows](https://mole.fit/windows): confirms five Windows workflows,
  vendor uninstall, maintenance, disk visualization, live status and tray utilities.
- [Windows help](https://mole.fit/windows/help): review/confirmation, residual-data,
  startup, update and supported-platform behavior.
- [Cache guidance](https://mole.fit/windows/blog/windows-cache-cleanup): distinguishes
  regenerating caches, personal data, in-use content and storage recovery.
- [Large-file guidance](https://mole.fit/windows/blog/find-large-files-on-windows):
  disk navigation, unreadable coverage and managed files/virtual disks.
- [Windows releases](https://mole.fit/windows/releases): retrieved page lists
  0.12.0 on 2026-10-15; feature evidence is time-scoped.
- [Mac site](https://mole.fit/) and [Mac releases](https://mole.fit/releases):
  AI functionality is now an explicitly requested design input via C15; its
  macOS-specific paths/services are not proof of Windows implementation.
- [Release-log delta](release-log-delta.md) distinguishes Windows desktop,
  Windows CLI, Mac desktop/preview and the unverified user-linked X post.
- GPUI primary sources and revision caveats are in gpui-windows-feasibility.md.

No binaries were downloaded, installed, launched or reverse engineered. Two linked
startup/update blog pages returned fetch failures; their contents were not used.

## Screenshot observations supplied by the user

| Image | Visible interaction evidence | Proposed use |
| --- | --- | --- |
| 01 Clean | Five tabs; prominent scan entry on a blue background | Clear first action and mode identity; keep global/project choices explicit. |
| 02 Software | Uninstall/Updates/Startup tabs; icon/name/version/size rows; counts, sorting, filter, search and selection | Dense virtualized list with shared selection/review semantics. |
| 03 Optimize | One prominent optimization entry | Entry opens applicable tasks and review rather than silently executing everything. |
| 04 Analyze | Disk selection and scan action | Explicit root/drive, then treemap/list and breadcrumbs. |
| 05 Status | Health, CPU/GPU/memory/battery/space/network/temperature cards and process table | Comparable information hierarchy with truthful availability and bounded sampling. |

Screenshot numbers/process usage are examples, not current machine measurements.
Image 03 depicts a rocky body; no specific planet identity is inferred from pixels.

## Functional coverage proposal

Legend: E = exists in core/CLI; T = core/Tauri available but CLI gap;
N = new capability; G = retained explicit guidance/refusal under project safety policy.
A row is not complete merely because a similar screen already exists.

| Feature | Current state | Proposed DevSweep outcome | Owners |
| --- | --- | --- | --- |
| Global/project cache scan, review, protect, history | E | Retain and expand exact supported catalogue, full report/filter/selection and explicit roots | C01,C03,C11 |
| Windows application/shader/temp candidates | Partial E | Evidence-backed rule families, owner/busy/age checks, rebuild cost and exclusion rules | C03,C11 |
| Privileged/system-managed cleanup | G/N | Dedicated reviewed capability where compatible; otherwise precise Settings guidance, never permanent-delete fallback | C02,C03 |
| AI tool caches, versions, sessions and worktrees | Partial/N | Dedicated CLI-first AI review, active-use protection, opt-in recovery; models/settings/credentials never automatic cleanup | C15,C03,C11 |
| Inventory with icons/qualified size/source | E/partial | Reuse exact identities, add needed metadata without treating unknown size as zero | C04,C12 |
| Native MSI/vendor/Scoop uninstall | N; MSIX E | Provider-specific approved flow, live requery, external UI/reboot/unknown states | C02,C04,C12 |
| Update check and selected update execution | T / N | CLI check then exact package/version/source plan and install outcome | C01,C05,C12 |
| Startup list/enable/disable and orphan repair | T / N | CLI parity, protected entries, backup/conditional restore and exact identity | C01,C05,C06,C12 |
| Residual files/keys | T / N | Separate review, exact ownership, recycle files; finite backed-up registry work only | C04,C05,C12 |
| Maintenance catalogue | Partial E | Preflighted supported repairs, due checks, clear skip/reboot conditions | C02,C06,C12 |
| Disk map/drill-down/reveal/recycle | E / T | CLI parity, native treemap plus accessible list, independent action authority | C01,C07,C13 |
| Large managed files and lock owners | N/partial | Explain evidence and ownership; system/virtual/cloud/shared content protected | C07,C13 |
| Live metrics/process list | E/partial | Capability-aware GPU/thermal/disk data, explainable health and process details | C08,C13 |
| Process termination | N | Separate explicit action, PID+creation identity and critical-process protection | C02,C08,C13 |
| Tray/compact status | T | Rust native tray with shared collector and visibility-aware lifecycle | C10,C13 |
| Keep Screen On / Clean Screen | N | Explicit visible sessions, safe shutdown and tested escape; no persistent policy edits | C13 |
| Settings/accessibility/diagnostics | Partial E/T | Native IME/UIA, preferences compatibility, local diagnostics and provenance | C09,C10,C14 |

## Deliberate differences

Do not copy the reference's permanent-delete choices, commercial licensing,
registry-cleaner assumptions or exact supported-app count. The repo explicitly
forbids permanent file deletion, Cargo-home changes and Docker cleanup. Maintenance
and uninstall may be intrinsically irreversible; show the precise operation and
provider outcome instead of promising universal undo.

The user explicitly added AI cleanup from current release history. Keep the five
primary modes and put a dedicated AI review within Clean, with CLI support first.
Do not assume the Mac page's paths/recovery APIs exist on Windows.
All gaps are tracked rather than disguised as implemented features.
