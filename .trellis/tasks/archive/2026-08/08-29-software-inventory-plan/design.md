# Design - Software Inventory and Plan

## Selected MSIX binding and dependency decision

`windows-sys 0.59` does not project the WinRT `PackageManager` object model.
The selected solution is a Windows-only direct dependency:

```toml
windows = { version = "=0.56.0", features = [
  "ApplicationModel",
  "Foundation",
  "Foundation_Collections",
  "Management_Deployment",
  "Win32_System_WinRT",
] }
```

This exact crate/version is already present in `Cargo.lock` through `trash`, so
the plan permits feature unification but no additional `windows` version. It is
Microsoft `MIT OR Apache-2.0`, declares Rust 1.62, and its cached archive is
10,807,828 bytes. Before/after release builds must show <=5 MiB uncompressed exe
growth and <=2 MiB zip growth; otherwise the task returns to planning.

Rejected alternatives are PowerShell/Get-AppxPackage, winget, Windows App SDK,
vendor uninstall strings, hand-maintained/generated raw COM bindings, or hiding
MSIX source failure. The user separately approved this exact Windows-target
version/feature declaration during planning. No other dependency is authorized;
the declaration is added only after the final planning summary is approved and
this leaf is started.

## MTA and API contract

One dedicated Software MTA thread calls `RoInitialize(RO_INIT_MULTITHREADED)`,
owns `PackageManager`, and serializes WinRT enumeration requests. The caller's
exact string SID comes from the current process token; the adapter invokes only
`PackageManager::FindPackagesByUserSecurityId(&HSTRING)`, never `FindPackages()`,
an empty-SID overload, or any all-user query. It reads `Package.Id`,
`PackageId.FullName`, version/architecture, framework/resource/optional flags,
and safe display metadata. Worker init/API/iteration failure yields an explicit
MSIX source state and never an empty-complete source. The worker is joined on
shutdown/cancel.

## Eligibility and refusal matrix

Refusal is selected by the first matching row; this priority is shared by plan,
CLI, TUI, and Desktop and is part of the fingerprint.

| Priority | Source/evidence | V1 eligibility | Stable reason |
| --- | --- | --- | --- |
| 1 | identity is DevSweep/protected | manual | `protected_product` |
| 2 | owning source is partial, denied, corrupt, or incomplete enough that dependency/conflict checks cannot finish | manual | `source_incomplete` |
| 3 | exact identity/provenance conflicts or required identity is malformed | manual | `conflicting_identity` |
| 4 | ARP `NoRemove=1` | manual | `no_remove` |
| 5 | hidden/SystemComponent or no safe displayable product record | manual | `hidden_entry` |
| 6 | Windows/system component or update/hotfix/service-pack classification | manual | `system_or_update` |
| 7 | MSIX framework/resource/optional/dependency package | manual | `dependency_package` |
| 8 | MSIX stub package | manual | `stub_package` |
| 9 | MSIX status is not healthy/registered for the exact current SID | manual | `unhealthy_package` |
| 10 | any MSI context (`USERUNMANAGED`, `USERMANAGED`, or `MACHINE`) | manual | `msi_execution_not_supported_v1` |
| 11 | registry-only ARP identity | manual | `registry_only_manual` |
| 12 | unsupported source/host capability | manual | `unsupported_source` |
| 13 | exact current-user MSIX main package with none of the above | selectable | `eligible_current_user_msix` |

Exact ARP/MSI correlation may preserve both provenance records but cannot make
an MSI selectable. If the MSIX enumeration or dependency/status pass is partial,
all affected MSIX entries remain visible and manual because eligibility cannot
be proven from an incomplete package graph.

## Optional evidence wire contract

`size` is a closed V1 union, always serialized and never inferred by a renderer:

```text
{state:"available", value_bytes:u64,
 basis:"reported_estimate"|"measured_installed_location",
 source_code:"arp_estimated_size_kib"|"msi_estimated_size_kib"|"msix_installed_path",
 observed_at_unix_ms:u64}
{state:"partial", lower_bound_bytes:u64,
 basis:"measured_installed_location", source_code:"msix_installed_path",
 reason_code:string, observed_at_unix_ms:u64}
{state:"unknown", reason_code:string}
```

For ARP/MSI observations, a present nonnegative `EstimatedSize` value with exact
source provenance is KiB and is converted with checked `* 1024`;
overflow/type/fuzzy-correlation failure is `unknown` and the UI
must label `reported_estimate` as estimated, not freed space. For current-user
MSIX, the MTA adapter calls the exact package's `InstalledPath`; the path stays
inside core and is passed to the existing bounded/cancelable/no-follow sizing
boundary. Complete walks are `available/measured_installed_location`; incomplete
walks are `partial` lower bounds; denied/missing paths are `unknown`. No related
user data is guessed or included.

`last_used` is also always present. Its complete V1 wire type is:

```text
{state:"unknown", reason_code:"no_supported_exact_source"}
```

V1 has no documented exact-identity provider and therefore emits this value for
ARP, MSI, and MSIX. Adding an `available` variant requires a later versioned
schema decision with a named provider; an implementation cannot insert a local
source code into V1. `InstallDate`,
`Package.InstalledDate`, UserAssist, event/telemetry data, and fuzzy executable,
name, or publisher matching are explicitly rejected. This still implements the
field, source semantics, fixtures, renderer state, and future fail-closed
extension boundary without fabricating activity.

## Inventory, fingerprint, and preview

ARP, MSI, and MSIX adapters return tagged observations plus independent source
evidence. ARP reads only approved identity/display/value fields and never reads
`UninstallString`, `QuietUninstallString`, or `DisplayIcon`. Exact authoritative
relations may collapse duplicate views while preserving provenance; display
name/publisher/path never merge identities.

The fingerprint hashes schema version, source evidence, tagged identity,
version, scope, ordered eligibility/refusal, and optional evidence. `software plan --inventory <FILE> --select
<SOFTWARE_ID>... --output <FILE>` stores only the fingerprint and identities.
Preview re-enumerates live state and creates non-serializable strategy tokens;
the serialized preview contains action class/scope/refusal/digest, not program,
argv, vendor strings, or guessed paths.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-core/Cargo.toml` | exact direct `windows = =0.56.0` feature declaration |
| `Cargo.lock` | verify feature unification with no added `windows` version |
| `crates/devsweep-core/src/software/mod.rs` | Software V1 service and public boundary |
| `crates/devsweep-core/src/software/model.rs` | tagged identities, source evidence, eligibility/refusal matrix, optional size/last-used DTO, and inventory/fingerprint DTO |
| `crates/devsweep-core/src/software/arp.rs` | safe 32/64 HKCU/HKLM registry adapter plus allowlisted classification and EstimatedSize evidence |
| `crates/devsweep-core/src/software/msi.rs` | MSI user/machine identity inventory adapter |
| `crates/devsweep-core/src/software/msix.rs` | dedicated-MTA PackageManager current-SID adapter, package status/dependency checks, and installed-path size evidence |
| `crates/devsweep-core/src/software/plan.rs` | untrusted selection, revalidation, preview digest/token |
| `crates/devsweep-core/src/software/tests.rs` | hostile/source/identity/stale/dependency tests |
| `crates/devsweep-core/src/lib.rs` | export Software inventory/preview interface |
| `crates/devsweep-cli/src/application/commands/software/inventory.rs` | frozen inventory/plan handlers only |

The CLI contract task first creates `application/commands/mod.rs` and the empty
`application/commands/software/mod.rs` boundary; this task fills only
`software/inventory.rs`.

Rollback removes the Software V1 schema and direct feature declaration together;
it does not alter CleanupPlan or cleanup inventory.
