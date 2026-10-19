# Local toolchain decision

Date: 2026-10-15.

The user answered the proposed isolated Rust 1.88 download with:

> 使用本机最新版本工具链应该是1.99

Use the installed Windows rustc/Cargo 1.99.0 for current development and final
Windows validation. Do not download or install Rust 1.88, change the global
default, or wait for that proposed installation. Existing Linux WSL tools may
provide separate portability evidence without any new installation.

The current dependencies do not require raising `workspace.package.rust-version`
above 1.88.0, so this local compiler choice does not silently remove that existing
compatibility declaration. Current-compiler success is not exact MSRV success.
Keep the existing MSRV CI jobs and mark missing exact toolchain/platform runs
NOT_RUN. Earlier Linux rustc 1.88 with Cargo 1.98 receipts remain precisely labeled
mixed-toolchain evidence. C09 may require a separately documented compiler policy
for its final GPUI revision under the approved latest-compatible policy.

Implementation approval remains valid for all children in their recorded order;
this decision supplies no commit, push, install, or publishing authorization.
