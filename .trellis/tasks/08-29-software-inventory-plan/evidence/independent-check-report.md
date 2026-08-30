# Independent Trellis Check Report

## Child verification

Overall result: **PASS**. PRD acceptance criteria AC1-AC5 are satisfied after two in-scope fail-closed fixes. No blocker, dependency change, contract expansion, parser edit, execution capability, or protected-file edit was introduced by this check.

- AC1: PASS. The ARP adapter reads only allowlisted identity/display/classification/size values; forbidden uninstall command and icon names occur only in hostile tests. Malformed or oversized native values become partial/unknown, and hostile display data cannot enter selection plans, program data, or argv.
- AC2: PASS. All four ARP hive/view identities, all three MSI contexts, and exact current-SID MSIX identities remain tagged. The refusal matrix is first-match ordered, all MSI/ARP records are manual, and only qualifying current-user MSIX records are selectable. Duplicate observations now merge all refusal facts deterministically.
- AC3: PASS. Removed, changed, reinstalled, expired, manual, unknown, duplicate, and digest-mismatched selections fail closed. Preview revalidation accepts only the same exact healthy/selectable current-user MSIX identity and version.
- AC4: PASS. The parser file has a zero diff; the narrow `software.*` dispatch seam mirrors the already-active Analyze seam. Preview/uninstall remain unavailable, serialized plan payloads have exactly five fields, and no command line, guessed path, installed path, or cleanup authority is serialized.
- AC5: PASS. Reported KiB uses checked integer conversion, MSIX sizing uses the shared bounded/cancelable/no-follow walker, paths stay inside the adapter, size states remain closed, and all native last-used observations are exactly `unknown/no_supported_exact_source`.

## Findings (fixed)

- File: `crates/devsweep-core/src/software/mod.rs`, `crates/devsweep-core/src/software/tests.rs`
- Issue: Exact-identity duplicate merging retained the first conflicting metadata observation and merged only `protected`/`source_incomplete`. Reversing duplicate input could change the fingerprint or drop refusal facts such as `no_remove`, `dependency`, `stub`, `unhealthy`, and `unsupported`.
- Fix: Normalize scope from the tagged identity, merge every refusal flag with fail-closed OR semantics, collapse conflicting optional metadata/size to deterministic non-authoritative values, and add an order-independence/refusal regression test.

- File: `crates/devsweep-core/src/software/msix.rs`
- Issue: `windows` 0.56 `IIterable::IntoIterator` unwraps `First()` and turns `MoveNext()` errors into iterator termination. An MSIX iteration failure could therefore panic or be misreported as an available/complete source, contrary to the explicit partial-source contract.
- Fix: Replace implicit WinRT iteration with fallible `First`/`HasCurrent`/`Current`/`MoveNext` and `IVectorView::Size`/`GetAt` helpers. Cancellation or any collection failure preserves collected observations but marks the source partial, so all affected MSIX entries remain manual.

## Findings (not fixed)

None.

## Verification

- Lint: pass. `cargo fmt --all -- --check` exit 0; `git diff --check` exit 0 (only existing LF/CRLF notices); workspace Clippy passed inside `just ci`.
- TypeCheck: pass. Workspace Cargo check passed inside `just ci`; `npm --prefix desktop run typecheck` exit 0.
- Tests: pass. `cargo test -p devsweep-core software` exit 0 (14 passed); `cargo test -p devsweep-cli software` exit 0 (5 passed); `just ci` exit 0.
- Dependency: pass. The direct dependency remains Windows-target-only `windows = "=0.56.0"` with the five approved named features; the `devsweep-core` tree contains `windows` 0.56.0 only. Recorded license/Rust-version/archive facts remain consistent with the cached package evidence.
- Release size: pass. Independent final binary is 3,900,416 bytes, +413,184 bytes over baseline (<5 MiB); zip is 1,541,981 bytes, +150,480 bytes (<2 MiB). `just release-archive` exit 0.
- Native evidence: pass. Recomputed counts from the recorded document match the summary (1,293 entries: 823 ARP, 291 MSI, 179 MSIX; 5 available and 3 partial sources). All 291 MSI entries are manual, no non-MSIX entry is selectable, all 179 MSIX entries are current-user scope, the selected plan ID is a selectable MSIX entry, TTL is 900,000 ms, and the plan has only the five frozen fields.
- SID/MTA: pass. The focused Windows test uses the current process token SID, calls `FindPackagesByUserSecurityId`, joins the MTA worker, and reports zero active workers afterward. The independent host check confirms a non-elevated identity, a valid `S-1-...` SID shape, and no observed UAC prompt without recording the SID value.
- Authority audit: pass. No Software-to-`CleanupPlan` conversion, removal call, vendor uninstall/icon read, all-user MSIX query, network access, serialized installed path, or new process/shell execution path exists. Existing cleanup command program/argv separation is untouched.
- Spec sync: not required. The task design already states that WinRT iteration failures are partial and that exact conflicts are fail-closed; the fixes bring code into that existing contract without adding a new convention.

Raw independent logs:

- `independent-core-software-tests.log`
- `independent-cli-software-tests.log`
- `independent-cargo-fmt-check.log`
- `independent-git-diff-check.log`
- `independent-just-ci.log`
- `independent-desktop-typecheck.log`
- `independent-cargo-tree.log`
- `independent-release-archive.log`
- `independent-native-host-check.log`
