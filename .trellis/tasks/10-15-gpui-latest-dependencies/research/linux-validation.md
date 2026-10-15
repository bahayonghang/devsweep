# Linux compatibility validation

Date: 2026-10-15. Host: existing Ubuntu-24.04 WSL distribution on the Windows
development machine. Native macOS and hosted CI are not inferred from WSL.

## Toolchains and isolation

Installed Linux Rust 1.98.0 supplies Cargo/rustc. A registered Rust 1.88.0
installation contains rustc/rustdoc but lacks Cargo, so the initial exact
`cargo +1.88.0` invocation failed before compiling. No toolchain was installed.
The Windows development compiler remains the user's selected installed 1.99.0.

Earlier compiler-compatibility checks explicitly set `RUSTC` and `RUSTDOC` to
the existing Linux 1.88.0 binaries while invoking Cargo 1.98.0. These are mixed
toolchain receipts, not exact Cargo/Rust 1.88 validation. Source changes after
those receipts require renewed checks before final acceptance.

Tests use `env -u CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=4`,
`RUST_TEST_THREADS=4` and explicit Cargo
`--target-dir /mnt/d/Documents/Code/Rust/Exp/devsweep/dist/c16-target-linux`.
An initial test attempt that exported `CARGO_TARGET_DIR` was stopped during
compilation because that variable changes scanner fixture authority. It is not
used as product evidence. Check-only use of that build variable is recorded
separately from semantic test runs.

## Retained initial receipts

All following logs are under ignored `dist/c16/evidence/`.

| Log | Result |
| --- | --- |
| linux-msrv-check.log | Exact 1.88 preflight FAIL: Cargo component missing; no compilation |
| linux-rustc188-cargo198-check-final.log | Mixed-toolchain core/CLI all-targets check PASS |
| linux-rustc188-cargo198-test-isolated.log | CLI/integration 214 PASS; core 455 PASS/1 FAIL: Optimize recovery lock unavailable |
| linux-recovery-lock-isolated.log | Exact failing recovery test rerun PASS 1/1; not full-suite acceptance |
| linux-rust198-test-reviewed.log | Rust/Cargo 1.98: CLI/integration 214 PASS; core 454 PASS/2 FAIL: Optimize lock conflict and concurrent permit tests |

The two full-suite failures remain failures. Source inspection and a separate
deterministic fork/lock experiment are tracked in linux-lock-findings.md.
An isolated passing test does not erase a full-suite failure. The approved narrow
fix must preserve live-owner mutual exclusion and have its own regression test;
no single-thread workaround or timeout relaxation is accepted as that fix.

## Final verification

After the audit-lock correction and corrected optional-probe fixture, the complete
four-thread run passes with installed rustc 1.98.0 (88d9e12ae) and Cargo 1.98.0
(797e8a9bc). This is Linux portability evidence, not Windows 1.99 or exact MSRV
evidence. Source: `evidence/check-source-receipt-final.json`, aggregate
`a12e191faecba39fd8cec3d3af1069834f20d94bb3fd9affc884935a2ebd3485`.
Cargo.lock remains `8b3db0d1771fa999d6f8b3d2df695aecae7752e8ef94ae3fa04529108f5cfefa`.

Commands use `cargo +1.98.0 check`, `test`, and `clippy`, each with
`--locked -p devsweep-core -p devsweep-cli --all-targets --target-dir` and the
isolated target above; Clippy adds `-- -D warnings`. The exact sequential wrapper
is `dist/c16/verify-linux-reviewed.sh`. It sets the installed Cargo binary PATH
explicitly, clears `RUSTC`, `RUSTDOC` and `CARGO_TARGET_DIR`, captures each exit
code and fails if any command fails.

| Receipt in dist/c16/evidence | Result | SHA-256 |
| --- | --- | --- |
| linux-reviewed-v2-check.log | PASS | b7da1b10ecd29abb5b7486ac1fce2c2b1c94f9a5cf1f3d2a16f1baca5cbce9d7 |
| linux-reviewed-v2-test.log | PASS, 674 tests; 0 failed/ignored | da8385f8e0340f26f3b545bd1008a7d2ed115cf119d618be4a909481045e53fc |
| linux-reviewed-v2-clippy.log | PASS, warnings denied | 77ded128ded1f41928f0b36b888a41a9fe9bdfecdad55990d60336251b448f1c |
| linux-reviewed-v2-exit-codes.txt | check=0, test=0, clippy=0 | ea20303caeaddc7bddfdc1d49ea4a9f4624a3d0e70d6e60ef69843a9d8b77996 |

The test total is 190 CLI unit + 25 CLI integration + 457 core unit + 2 public
API tests. It includes the Unix process-tree tests and deterministic inherited
description lock regression. No test was disabled or serialized to hide contention.

Additional retained unsuccessful attempts:

- `linux-final-test.log`: the newly added status fixture declared no truncation
  while retaining an enumerated count of 4096 for one row. The strict decoder
  rejected it. The fixture was corrected to one enumerated row, preserving its
  schema assertions. This is not a production outcome failure.
- The first inline wrapper produced blank exit-code fields through shell argument
  forwarding; `linux-final-exit-codes.txt` is invalid as an exit-code receipt.
  Individual logs remain evidence, and the final file-based wrapper above avoids
  that ambiguity.
- `linux-reviewed-exit-codes.txt` records 127 for all phases: the first non-login
  shell invocation lacked Cargo on PATH and did not compile. The v2 wrapper
  names the existing Cargo PATH explicitly; no tool was installed.
