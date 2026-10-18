# C16 Latest compatible dependencies and Rust toolchain

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: Parent planning convergence and approval.

## Goal

Audit and adopt current stable dependencies across shared Rust code and the new GPUI application with reproducible pins and documented exceptions.

## Requirements

C16-R1. At implementation start, inspect official package/upstream release sources and inventory every direct dependency plus resolved transitive constraints used by core/CLI/new GPUI.

C16-R2. Prefer the latest stable compatible releases; for GPUI Windows functionality absent from registry releases, evaluate the latest upstream revision and pin one immutable coherent commit.

C16-R3. Refresh the Rust toolchain/MSRV policy as needed for accepted current dependencies, with documented compatibility impact and explicit CI coverage; Rust 1.88 is the current baseline, not a reason to choose stale libraries silently.

C16-R4. Keep Cargo.lock and exact evaluated revisions, license/advisory review and a dependency decision ledger. Never leave wildcard/floating main dependencies as the reproducibility policy.

C16-R5. Separate old Tauri baseline reproducibility from new-app dependency choice. Inventory baseline dependencies too; any intentionally unchanged legacy dependency gets a scoped reason, not an unreported exception.

## Acceptance criteria

- [x] C16-AC1: Dependency ledger lists current/latest/selected/source/check date/reason for every direct dependency and relevant constrained transitive family.
- [x] C16-AC2: Selected shared-core/CLI stable versions resolve and pass compatibility checks; newest GPUI candidates and any constraints are recorded, with final native pin/build acceptance explicitly owned by C09.
  Note: Local `just ci` exit 0 on rustc 1.99.0 is recorded in acceptance-status.md. That receipt is not an MSRV 1.88.0 pass and not a hosted matrix pass. C09 owns the native pin.
- [ ] C16-AC3: Format/check/test/clippy and existing core/CLI/legacy jobs run on the approved matrix; any declared core/CLI MSRV is actually tested. New GPUI native jobs and final dependency pin belong to C09 and do not block this child's earlier completion.
  Note 2026-10-18: the declared core/CLI MSRV **1.88.0 is now actually executed on Linux**. With `RUSTC` pinned to the installed `1.88.0-x86_64-unknown-linux-gnu` and cargo 1.98.0 driving it — verified with `cargo check -v`, which invokes that exact rustc path — `cargo check -p devsweep-core -p devsweep-cli --locked --all-targets` and `cargo test ... --all-targets` both exited **0** (770 passed, 0 failed, 0 ignored). That required `#[cfg(windows)]`-gating 8 `devsweep-core` and 6 `c01_shipped_parity` tests that assume Windows registry/volume/path semantics and were failing the hosted `ubuntu-latest` jobs. Windows/macOS 1.88.0, the hosted matrix, and 1.88.0 fmt/clippy/deny/audit remain NOT_RUN. See acceptance-status.md.
- [x] C16-AC4: Functional contracts and native-baseline identities remain reproducible; dependency upgrade alone never counts as performance acceptance. Workspace `just ci` exited 0 on 2026-10-17 (`devsweep-just-ci-5.txt`, 911 passed, 0 failed, 5 ignored, `ci complete`, rustc 1.99.0). Two fresh locked `desktop-gpui` release builds of an earlier tree matched SHA-256 `bf592d32b8b6e612d63a5d94c1bf4071ca36d0f45c57f496b64cb335553fc14a` and 13437440 bytes. A warning-free pair that both lacked an `incremental` log line matches: `devsweep-gpui-release-d` and `devsweep-gpui-release-e` are SHA-256 `095f51d77da5da5729c2341d24c726d460ca1a44ede2204789c13227e521fe62` and 13437952 bytes. Both builds exited 0 and neither log contains `warning:`. The earlier `release-warn` binary, SHA-256 `8d07070bcfa935ec93fa231fa0b5c5ed60f5461aaa3785749982c1beebed1f22` and 13348864 bytes, had an mbx `1 incremental` line and is not this identity. Locked `desktop-gpui` tests after `software_landing()` exited 0 with 48 passed and 0 failed (`devsweep-gpui-test-after-landing.txt`). These receipts are not a performance-protocol pass, not an MSRV 1.88.0 pass, and not a hosted matrix pass. A fresh Tauri baseline rebuild was not run in this session.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
