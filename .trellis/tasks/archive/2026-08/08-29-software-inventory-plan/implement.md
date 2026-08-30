# Implement - Software Inventory and Preview Plans

Start only after explicit approval of the final planning summary and the frozen
CLI contract. The user separately approved the exact Windows-target
`windows = =0.56.0` declaration and five named features during planning; that
approval covers no other dependency and does not itself start this leaf.

## 1. Verify and add the selected binding

1. Record pre-change `cargo tree`, `Cargo.lock`, release binary, and release zip.
2. Add only `windows = =0.56.0` with the five named features under the Windows
   target dependency table; require no second/new `windows` package version.
3. Build a dedicated MTA worker probe that initializes WinRT, gets the current
   process SID, constructs `PackageManager`, enumerates packages for that SID,
   reads exact `PackageId::FullName`, and shuts down cleanly.
4. Compare release sizes. Stop if the uncompressed binary grows >5 MiB or the
   release zip grows >2 MiB; do not change version/features or add Windows App
   SDK/PowerShell as a workaround.

Focused validation:

```powershell
rtk cargo tree -p devsweep-core -e features
rtk cargo check -p devsweep-core --all-targets
rtk cargo test -p devsweep-core software::windows
```

Rollback point: revert `Cargo.toml`/`Cargo.lock` and the MTA adapter together;
keep MSIX explicitly unsupported rather than silently returning an empty source.

## 2. Implement source adapters and immutable inventory

1. Add ARP 32/64 HKCU/HKLM and MSI user/machine adapters with source-local
   evidence and no reads of dangerous command/icon fields.
2. Add the MTA-owned current-user MSIX adapter and exact tagged identities.
3. Implement the ordered eligibility/refusal table exactly: all MSI and
   registry-only entries remain manual; only a complete, healthy, non-system/
   framework/resource/dependency/stub/protected/conflicting current-user MSIX
   package is selectable.
4. Implement the closed size union: checked KiB estimates for ARP/MSI and a
   bounded/cancelable/no-follow `InstalledPath` measurement for MSIX without
   serializing the path. Implement the last-used union with
   `unknown/no_supported_exact_source` for all V1 sources; do not read
   install/update dates, UserAssist, events, telemetry, or fuzzy mappings.
5. Implement exact deduplication, fingerprint including refusal/evidence,
   untrusted selection, live revalidation, opaque preview token, and digest.

Focused validation:

```powershell
rtk cargo test -p devsweep-core software::inventory
rtk cargo test -p devsweep-core software::plan
```

Rollback point: remove the whole Software V1 inventory/plan module; never reuse a
partially written fingerprint or convert observations into CleanupPlan.

## 3. CLI, native, and full gates

Implement only frozen `software inventory` and `software plan`; do not edit the
parser. Run hostile fixtures, exact SID/source partials, stale/reinstall cases,
and locale-invariant documents.

```powershell
rtk cargo test -p devsweep-cli software
rtk git diff --check
rtk just ci
```

Native standard-user evidence records registry-view coverage, current-user MSI/
MSIX identities, the all-MSI/manual gate, denied/corrupt sources, every ordered
refusal row, size evidence/limits, truthful unknown last-used, no UAC, MTA worker
shutdown, Cargo tree, license, and binary/zip deltas. Stop for any
other-user enumeration, admin requirement, vendor command field, dependency
change, or frozen CLI change.
