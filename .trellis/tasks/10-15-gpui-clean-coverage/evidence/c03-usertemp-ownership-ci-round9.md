# C03 UserTemp ownership proof: a real CI failure, analysed (round 9)

Date: 2026-10-19. This records the hosted-CI failures that the last CI run
(`39001489521`, SHA `b0b5ce6`) reported on the Windows jobs, and what they mean
for the current tree.

## The failures

`Rust (windows-latest)` and `MSRV (windows-latest)` both failed with two
`devsweep-core` tests:

| Test | Assertion |
| --- | --- |
| `rules::windows_caches::tests::stale_owned_temp_is_individual_and_recent_nested_or_locked_files_are_unavailable` | `left: 0`, `right: 4` — expected 4 candidates, got 0 |
| `execution::tests::windows_cache_pipeline::real_scan_saved_plan_preview_and_recorded_execution` | `left: 21`, expected `approved.len()` — the "all six families are executable" assertion |

The failure detail for the first one is the important part:

```
WindowsCacheUnavailable {
  path: "C:\\Users\\RUNNER~1\\AppData\\Local\\Temp\\.tmpsOcMPK\\Local\\Temp",
  status: OwnershipUnknown,
  detail: "temporary entry is not proven owned by the current user"
}
```

The rejected path is the **fixture root itself**, not an entry inside it. The
whole `UserTemp` family was therefore dropped, producing 0 candidates.

## Why the root probe fails on the hosted runner

`native_temp_probe` (`rules/windows_caches.rs:996`) opens the path with
`READ_CONTROL | FILE_READ_ATTRIBUTES`, reads the security descriptor owner with
`GetKernelObjectSecurity` + `GetSecurityDescriptorOwner`, and compares it with
`EqualSid` against the process token's user SID from `GetTokenInformation`:

```rust
!sid.is_null() && IsValidSid(sid) != 0 && EqualSid(owner, sid) != 0
```

If that chain fails at any step the result is `OwnershipUnknown` with the detail
above. On the hosted runner the temp directory is created under the runner
profile and the probe did not establish equality. Two candidate causes are
plausible and were **not** distinguished from here:

1. The directory owner on the runner is not the process token's user SID (for
   example an elevated/service context or a different profile SID).
2. One of the security API calls fails for a path reached through the 8.3 alias
   `RUNNER~1`.

This machine cannot reproduce either: `%LOCALAPPDATA%\Temp` and freshly created
subdirectories are both owned by
`S-1-5-21-2143440361-2401194771-2834292696-1001`, which equals the process token
SID, and 8.3 short names are disabled on this volume
(`GetShortPathName` returns the long path unchanged). Both tests pass locally,
including when run individually.

## A related real defect in the test hook

`rules/windows_caches.rs` gained `with_expected_owner_sid_for_test`, which
substitutes the expected SID argument while still running
`GetKernelObjectSecurity` and `EqualSid`. The test that uses it
(`user_temp_foreign_sid_is_ownership_unknown`, `:1583`) deliberately installs the
**LocalSystem** SID so the comparison must fail, then asserts the entry is
reported `OwnershipUnknown`. That is correct and passes.

But `stale_owned_temp_is_individual_and_recent_nested_or_locked_files_are_unavailable`
does **not** use the hook and relies on real ownership. That is what makes it
environment-dependent, and it is the test that failed on the hosted runner.

## Consequence

These two failures are **real and pre-existing on the hosted matrix**, not caused
by any change in this branch. They block C16-AC3's hosted-matrix clause and they
will keep failing until either:

- the ownership proof is made robust for the runner's temp directory (for example
  treating "the root probe cannot prove ownership" as coverage-only rather than
  dropping the family, or proving ownership via the opened handle's
  `GetSecurityInfo` with a fallback), or
- the test stops depending on ambient ownership by using the SID hook the same
  way its sibling test does.

Both are product/test changes in `crates/devsweep-core`, which is inside this
task's allowed scope, but they were **not** made this round — the local machine
cannot validate the fix, and shipping an unverified change to a security-sensitive
ownership proof would be worse than recording the defect precisely.

The other two CI failures are platform-specific test assumptions already handled
by the round-2 gating work (`crashes_before_and_after_dispatch_recover_without_removal_retry`
on ubuntu failed with `Audit(LockUnavailable)`, and
`standard_native_providers_roundtrip_and_preserve_cache_sentinels` on macOS), and
must be re-checked on a fresh hosted run rather than assumed fixed.
