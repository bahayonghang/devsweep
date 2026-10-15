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

- [ ] C16-AC1: Dependency ledger lists current/latest/selected/source/check date/reason for every direct dependency and relevant constrained transitive family.
- [ ] C16-AC2: Selected shared-core/CLI stable versions resolve and pass compatibility checks; newest GPUI candidates and any constraints are recorded, with final native pin/build acceptance explicitly owned by C09.
- [ ] C16-AC3: Format/check/test/clippy and existing core/CLI/legacy jobs run on the approved matrix; any declared core/CLI MSRV is actually tested. New GPUI native jobs and final dependency pin belong to C09 and do not block this child's earlier completion.
- [ ] C16-AC4: Functional contracts and native-baseline identities remain reproducible; dependency upgrade alone never counts as performance acceptance.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
