# Wave C helper implementation and focused validation

2026-10-13. Implementer ownership of `crates/devsweep-core/src/rules/windows_caches.rs`
is handed back to the functionality coordinator after this checkpoint. No real
application cache or temporary data was cleaned. Fixtures use `tempfile` and
all probes are read-only; normal fixture teardown is confined to those fixtures.

## Closed API and integration

- `WindowsCacheFamily::{Chrome, Edge, Firefox, VsCode, Cursor, UserTemp}`.
- `OwnerState::{Idle, Busy, Unknown}` and `owner_state(&[&str])`: closed callers
  supply executable basenames. Native ToolHelp enumeration must finish normally;
  enumeration/encoding failure, an invalid query or its 32768-process ceiling
  returns `Unknown`. A matching running process returns `Busy`.
- `discover_windows_caches(family)` returns exact child paths and typed
  `WindowsCacheUnavailable { path, status: ScanCoverageStatus, detail }` records.
  Recognized but busy/unknown/protected children stay visible. Root/profile
  failures may have no candidate and must never become actions themselves.
- `validate_windows_cache_location(family, path)` proves the same default
  application root, profile/install metadata and exact allowed child for pure
  rule reconstruction. It does not treat a running app as a whole-plan failure.
- `validate_windows_cache(family, path)` reacquires the location proof, owner
  activity and tree eligibility for preview/live execution. Neither API moves
  files or runs a cleanup command. User protections remain an additional shared
  executor/scanner guard, not a second protection store in this module.

The functionality implementer owns definitions, registration, scan/plan/live
safety integration and production roundtrip tests. New browser/editor/temp
rules must remain initially unselected. This helper does not certify those
integration steps by itself.

## Supported footprints and proof

| Family | Exact supported layout | Required proof / exclusions |
| --- | --- | --- |
| Chrome / Edge | Default current-user LocalAppData vendor `User Data`; each registered profile's `Cache`, `Code Cache`, `GPUCache` | `Local State` `profile.info_cache` membership, safe leaf name, valid bounded `Preferences` object, no-follow ancestors and subtree, owner idle |
| Firefox | Default current-user roaming `profiles.ini` plus matching local `Profiles/<leaf>/cache2` | `ProfileN`, `IsRelative=1`, exact `Profiles/<leaf>`, both profile directories; no whole profile or persistent roaming storage |
| VS Code / Cursor | Default RoamingAppData `Code` / `Cursor`; `Cache`, `Code Cache`, `GPUCache`, `CachedData` | `User` context plus standard user/system installation executable and `resources/app/product.json` matching `applicationName`; owner idle |
| User Temp | Default LocalAppData `Temp`; individual direct entries only | Root and every entry owned by current TokenUser SID; every descendant at least seven days old; live no-sharing read-only open on every entry; no reparse, secret/repository/persistent-state sentinel |

Application paths/metadata and the narrow Electron cache inference are sourced
in [windows-cache-boundaries.md](../research/windows-cache-boundaries.md).
Chromium's registered profile storage is additionally supported by its
[profile attributes implementation](https://raw.githubusercontent.com/chromium/chromium/main/chrome/browser/profiles/profile_attributes_storage.cc).

Temp ownership compares the file-handle security descriptor owner with the
process token user, using
[GetTokenInformation](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-gettokeninformation),
[GetSecurityDescriptorOwner](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-getsecuritydescriptorowner)
and `EqualSid`. The live no-sharing open follows
[CreateFileW sharing semantics](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew).
Handles are opened without following the final reparse point and their metadata
is checked; failed security queries and locks refuse eligibility. These are
point-in-time observations, so the shared executor must recheck immediately
before dispatch. They do not promise that another process cannot open a file
after the check, or authorize terminating an application.

## Focused evidence

Build command: `cargo test --locked -p devsweep-core --lib --no-run --message-format=json`.
Helper source SHA256 at handoff:
`d29fec808bfa8bc8a816f4b88896a954bc552b4300738482cd5988fb46f4d241`.
The emitted native executable was copied before running:

```text
target/performance/cleanup-earth/wave-c-check-01/devsweep-core-tests.exe
SHA256 cd8f57f8dd38c8bcd148b28d3fafd273b5feeec95a70c8f0e4192132d5407cc7
devsweep-core-tests.exe rules::windows_caches --test-threads=1 --nocapture
```

**9 passed, 0 failed, 0 ignored**, 0.53 seconds, native Windows. The retained
[raw focused log](windows-cache-focused-tests.txt) contains all test names.
Coverage includes both Chromium families, registration/foreign-path rejection,
independent healthy profile retention, bounded metadata, Firefox local/roaming
separation, both editor installation proofs, persistent/agent-data sentinels,
busy/unknown owner states, budget refusal, cloud/unverified no-traversal, native
ToolHelp current-process detection, and genuine current-user SID/no-sharing
Temp eligibility with locked and newer-descendant negatives.

The cloud/unverified test uses the repository's `PathReparseProbe` seam and
asserts no descendant is visited; it does not claim a real Cloud Files provider
or junction was created. Native Temp positives and locked-file negatives use
actual Windows handles/security descriptors, not mocked ownership.

An earlier direct Cargo run passed the native Temp, native owner and sentinel
tests, then exited `0xc0000005` without finishing; it is a failed attempt, not
pooled into the pass above. Other parallel work observed native test failures
and disappearing binaries too; no cause is established here. After the passing
copied run, the coordinator found its executable absent from the `target/`
directory while logs/exit code remained. Therefore the SHA/path identify the
executed artifact historically; the binary is **not retained for reuse**.
Future native evidence copies should use the separately observed stable `dist/`
location and document retention explicitly.

## Limits and outstanding integration

- Default layouts only. Custom browser roots, absolute/custom Firefox profiles,
  editor portable/override layouts and custom Temp outside LocalAppData/Temp
  remain explicitly unsupported/unavailable. Functional integration subsequently
  repaired mixed Firefox registries: healthy supported profiles continue while
  custom/ambiguous entries retain section-specific unavailable reasons. Structural
  registry corruption still refuses the provider. The expanded focused fixture
  passed in the 24-test rules group (`dist/native-test-isolation/rules-current.txt`).
- Each family has a two-second / 50000-entry verification budget, each directory
  4096 entries, each profile registry 128 profiles, metadata one MiB and traversal
  depth 64. Budget exhaustion is typed, never represented as clean/empty.
- Current helper entrypoints do not accept cancellation; the scanner can check
  between families. Runtime integration must assess the bounded-family latency
  and add the existing cancellation observer if its acceptance requires it.
- Real scan-to-v2-to-preview and fake dispatch/sentinel integration, owner/config
  drift and user-protection tests belong to functionality integration. No claim
  of F-AC5/7/8 completion is made by these nine focused tests.
- Full `just ci`, non-Windows verification and packaged CLI/Tauri parity remain
  outstanding at this handoff.
