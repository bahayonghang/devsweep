# Keep-awake core/CLI independent review

Scope: the approved steps 3–8 slice only. This review does not clear the step 9
or step 10 gates, and does not review MSI or Updates implementation completion.
No Computer Use, native power request, host mutation, commit, push or archive.

## Findings fixed

- `status/keep_awake_session.rs`: socket inactivity timeouts could be renewed by
  each partial read, delaying a query or joined shutdown far beyond the stated
  budget. Reads now have an absolute deadline: 400 ms on the server and 2 s for
  client replies. A slow-byte fixture asserts that partial input cannot renew it.
- `history/mod.rs` and `status/keep_awake_audit.rs`: new Status/Software-support
  History reads were unbounded, and Status append checked metadata then read the
  entire file. Actual reads now use `take(16 MiB + 1)` and refuse oversized data.
  Existing Clean/Software/Optimize reader behavior is unchanged. Status writer
  and History also reject nonfinite/out-of-range durations and malformed optional
  SHA-256 digests in the closed V1 record. An unterminated Status tail is excluded,
  so an interrupted terminal does not turn its preceding start into success.
- CLI `commands/status.rs`: a session failure after the initial lease JSON could
  emit a second JSON document, or replace the original failure with output-exists
  when a named sink was used. Late failures use the existing after-output marker,
  preserve the real exit class and report their diagnostic on stderr.
- CLI `commands/history.rs`: errors for `--domain status` incorrectly named the
  Clean store. The requested domain is retained; unspecified domains use History.
- Core `status/keep_awake.rs`: simultaneous audit and backend failure retained
  only the audit diagnostic. The unknown-after-dispatch category is preserved
  together with the native failure detail.
- Human Keep-awake CLI errors now use bilingual catalogue reasons and preserve
  underlying diagnostic details. JSON code, message and shape remain locale
  neutral. Existing Software keys were preserved; the GPUI owner requested the
  provider-unsupported message explicitly mention source identity.

## Contract review

- Opaque immutable core plans, live SHA-256 previews and explicit execute inputs
  are the production authority. No GUI-signed bool shortcut, environment-based
  production authority, network START, child CLI, daemon or owner receipt exists.
- Generations belong to the relevant owner. Unrelated owners do not invalidate
  one another's review. Time/remaining milliseconds do not change a digest;
  acquisition, clear/failure and real expiry invalidate old authority.
- One joined worker owns backend acquisition, release and destruction. Every
  service command reconciles expiry, so a busy queue cannot starve the lease.
  Failed expiry clear is retained for explicit retry/shutdown and is not retried
  or logged on every poll. Backend Drop thread identity is asserted explicitly.
- Explicit action started-audit failure refuses its OS call. Terminal failure
  remains unknown. Expiry, cancellation and owner exit still attempt safe clear
  when their audit fails. Lock/oversize fixtures assert zero acquisition calls.
- Stop/expiry keeps the endpoint observable until shutdown. Repeated reviewed
  toggles reuse one lease/token/endpoint and owner thread. Shutdown joins the
  transport and owner even when cleanup reports failure. Unreachable old tuples
  stay unavailable/unknown; action history is never used to synthesize off.
- History reads the Status domain and Software support lifecycle records under
  fixed paths. Started-only records summarize as unknown. Closed decoding and
  redaction keep credentials and executable details out of output.

## Findings outside owned files

Read-only GPUI inspection found that a failed clear left a persistent core error,
while the GUI only polled on retry and could never request another explicit Stop.
This was sent to the GPUI owner, who reports a reviewed Stop retry and fake
failure/recovery regression were added. GPUI verification remains that owner's
and the main review's responsibility; this report does not claim its native UI
or complete GPUI gate passed.

## Verification

- Owned Rust formatting, `git diff --check`, and catalogue JSON parsing: passed.
- Coordinated focused tests, using `--locked --target-dir dist/ci-target` and
  `--test-threads=4`, passed:
  - core `--lib keep_awake`: 20 passed;
  - core `--lib history`: 14 passed;
  - CLI `--lib keep_awake`: 4 passed;
  - CLI `--lib history`: 5 passed;
  - CLI `--test keep_awake_contract`: 2 passed.
  These are 45 successful test executions, including the file-history test
  shared by the two core filters. The first CLI compile caught two new tests
  appended outside the existing tests module; they were moved inside it before
  the successful rerun. Integration tests query/stop a fake core session from a
  real CLI subprocess and check closed-endpoint unknown in JSON and both locales.
- Type checking of the focused test binaries: passed. Dedicated all-target
  type-check/Clippy and canonical `just ci` / `just gpui` remain reserved for the
  main session's one final gate. They were not run or claimed by this reviewer;
  the compiler slot was released to the MSI reviewer after focused success.
- The subsequent shared all-target Clippy found four local style errors in
  this slice. Two nested checks now use let-chains, duration divisibility uses
  `is_multiple_of`, and the test call tuple has a private alias. Owned formatting
  and diff checks passed; the GPUI checker owns compiler revalidation. These
  mechanical changes preserve the Rust 1.88 MSRV and public interfaces.
- Native power effectiveness, screenshots, UAC, real packages and other platforms:
  unverified. All action tests use injected backends, isolated journals and
  loopback fixtures; these are contract evidence only.
