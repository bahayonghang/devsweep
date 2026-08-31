# Design - Optimize Catalogue and Execution

## Exact closed catalogue V1

| Stable id | Action | Exact identity / predicate | Terminal meaning |
| --- | --- | --- | --- |
| `dns.flush` | execute | native `ipconfig.exe`, argv `[/flushdns]`, 10 s | observed process result or unknown |
| `settings.storage_recommendations` | Settings | `ms-settings:storagerecommendations`, build >=22000 | launched only |
| `settings.search` | Settings | `ms-settings:search`, build >=22000 | launched only |
| `settings.energy_recommendations` | Settings | `ms-settings:energyrecommendations`, build >=22624 | launched only |
| `guidance.drive_optimize` | guidance | no dispatch variant | guidance shown |
| `guidance.system_integrity` | guidance | no dispatch variant | guidance shown |
| `guidance.filesystem_check` | guidance | no dispatch variant | guidance shown |
| `guidance.network_reset` | guidance | no dispatch variant | guidance shown |

The build 22000 floors are conservative project policy for the two Windows 11
review pages. The energy floor is Microsoft's documented minimum. OS build comes
from `RtlGetVersion`, exposed by `windows-sys` through
`Wdk_System_SystemServices`, not environment text or a manifest-sensitive
version helper. Query failure, unsupported build, Settings policy block, or
launch failure is explicit unavailable/failed evidence; no URI is substituted.

## Plan, authorization, and audit

`MaintenancePlanV1` stores catalogue version and exactly one id. Live preview
rechecks platform/build, resolves one non-deserializable enum variant, and hashes
catalogue version + id + action class + literal URI/program/argv + capability.
Run requires matching digest and confirm. An exclusive journal lock and
one-operation permit cover intent, dispatch, and terminal/unknown transition.
Guidance has no executor variant.

## Exact DNS/WOW64 algorithm

1. Call `IsWow64Process2(GetCurrentProcess())` and fail closed on error.
2. If the process machine is nonzero (32-bit process on 64-bit Windows), call
   `GetWindowsDirectoryW` and append the literal `Sysnative\ipconfig.exe`.
3. Otherwise call `GetSystemDirectoryW` and append literal `ipconfig.exe`.
4. Require an absolute drive/UNC-free local system path, exact case-insensitive
   basename `ipconfig.exe`, existing non-directory file, and no reparse/path
   search. Do not canonicalize `Sysnative` into `SysWOW64`.
5. Pass the path as program and one argv element `/flushdns` to the existing
   bounded runner. Never consult `PATH`, `%WINDIR%`, the environment, shell text,
   `SysWOW64`, or filesystem-redirection toggles.

The Settings adapter passes only one of the three literal URIs to
`ShellExecuteExW` with verb `open`, `SEE_MASK_FLAG_NO_UI`, and no parameters or
working directory. A successful return is `launched`; it is never maintenance
completion.

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-core/Cargo.toml` | keep existing `Win32_System_Threading` and add exactly `Wdk_System_SystemServices`, `Win32_System_SystemInformation`, `Win32_UI_Shell`, and `Win32_UI_WindowsAndMessaging`; the WDK feature is required only to expose `RtlGetVersion` |
| `crates/devsweep-core/src/optimize/mod.rs` | V1 service/public boundary |
| `crates/devsweep-core/src/optimize/catalogue.rs` | exhaustive eight-id registry and DTOs |
| `crates/devsweep-core/src/optimize/plan.rs` | untrusted plan, preflight, digest, validated enum |
| `crates/devsweep-core/src/optimize/windows.rs` | build, WOW64/System32, DNS, Settings adapters |
| `crates/devsweep-core/src/optimize/execution.rs` | one-operation state machine/outcomes |
| `crates/devsweep-core/src/optimize/audit.rs` | locked durable V1 journal |
| `crates/devsweep-core/src/optimize/tests.rs` | exhaustive/hostile/fake/native tests |
| `crates/devsweep-core/src/lib.rs` | export Optimize service |
| `crates/devsweep-cli/src/application/commands/optimize.rs` | exact list/plan/preview/run handler |

The CLI contract task first creates `application/commands/mod.rs`; this task
fills only its frozen `optimize.rs` submodule and does not own the module root.

Rollback disables the V1 catalogue/executor and keeps audits readable. This task
does not own parser grammar, Tauri/UI, shell/spec, or CleanupPlan.
