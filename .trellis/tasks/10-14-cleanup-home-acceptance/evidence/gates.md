# Integrated build and gate evidence

Date: 2026-10-14. This is current checkout evidence, not an inherited PASS.

## Rust

`just ci` passed using the unchanged canonical recipes: format, workspace
all-target check, 788 passing tests with 5 existing ignored tests, and workspace
all-target Clippy with warnings denied. Log:
`dist/cleanup-home-final/gates/just-ci-isolated.log`.

The first attempt could not start the core test executable in the existing
externally managed `target` link, after CLI tests passed. Its cause is unproven;
the original failure remains in `gates/just-ci.log`. `run-ci.py` temporarily
created `.cargo/config.toml` with only `build.target-dir = "dist"`, refused to
replace any existing config, ran `just ci`, and removed its exact file in
`finally`. The file's absence was verified afterward. No inherited
`CARGO_TARGET_DIR` contaminated Cargo-discovery fixtures.

## Native release

CLI release build passed (`gates/cli-release.log`). Desktop release with
`tauri/custom-protocol` passed (`gates/native-release-fresh-core.log`). Frozen
executables are in `dist/cleanup-home-final/native/`; they have not been installed.

The first desktop attempt used a stale release core rlib which Cargo reported
as Fresh despite its missing new advisory module. Touching the source timestamp
did not invalidate it. `cargo clean -p devsweep-core --release --target-dir dist`
refused the mixed artifact directory's missing CACHEDIR.TAG and made no cleanup.
The specifically identified old rlib was moved, without deletion, from
`dist/release/deps/libdevsweep_core-de370e8ee2669e6b.rlib` to
`gates/stale-core-de370e8ee2669e6b.rlib`; recompilation of core and desktop passed
in 3m35s. Diagnostic logs retain the original unresolved-import failure.
No product-source workaround or capability change was made for either build issue.

## Desktop

The initial restricted `just desktop-web-check` could not run its generated
PowerShell recipe (AuthorizationManager rejection). The normal-host retry
passed type generation, lint and typecheck, then found two old Stage assertions
expecting Clean's removed Earth/single Scan layout. The tests were adapted and
independently strengthened to assert named scope sections, a ready global action
with zero roots, a disabled project action, and no Earth. Other modes' assertions
were preserved. A subsequent run caught an asynchronous Base UI portal test
lookup; the accessible-role assertion now waits for the portal to open while
preserving its selection, pointer, portal and focus assertions. See
`settings-portal-review.md`.

The final complete `just desktop-web-check` passed: generated type parity,
ESLint, TypeScript, 63 files / 471 Vitest tests, 5 Node script tests and production
Vite build. Log: `dist/cleanup-home-final/gates/desktop-web-check-verified.log`.
The existing large-chunk advisory is retained; it is not a build failure.

## Actual npm follow-up

F's isolated real Node 22.23.2/npm 10.9.8 run exposed a Windows runtime argument
defect hidden by the process-fixture shim: a canonical verbatim script argument
fails in Node's loader, whereas the same bound script with its ordinary absolute
path succeeds. A focused dispatcher repair retains canonical support identities
and content hashes, converts only supported filesystem path prefixes for argv,
and verifies canonical equivalence before dispatch. Full Rust gate and release
binaries were refreshed after this fix; the first PASS and frozen binaries
above are pre-fix evidence. Actual Node/npm isolated CLI scan/plan/preview now
passes in both scopes and human locales (`native-cli-04/summary.json`). No
cleanup was executed.

## Final integrated revision

`just ci` **PASS**: 791 tests passed, 0 failed, 5 existing ignored tests,
format/check and workspace all-target Clippy with warnings denied. Raw log:
`dist/cleanup-home-final/gates/just-ci-final.log`. Temporary Cargo config was
verified absent afterward. The intervening npm-fixed run exposed a reproduced
MSIX shared-counter test race; a test-only lock preserves the original worker
and panic assertions, documented in `msix-test-isolation-review.md`.

Final production build inputs contain only main `index.html`, `hud.html`,
assets, Earth assets for unchanged modes and notices. Separate benchmark builds
are preserved outside Tauri's frontendDist at `benchmark-builds/`. An earlier
build made while those test pages were inside frontendDist was not promoted as
the final production binary. The main web build and final native build passed
(`production-web-isolated.log`, `native-release-production.log`).

Final frozen CLI/native executables, exact production web bytes and hashes:
`dist/cleanup-home-final/native-fixed/manifest.json`. The desktop SHA256 is
`6904d6bb2d5204f76060226257957e7aa5ab9933c37bf5528efb5b7cd3e88feb`;
CLI is `4636a525c0d89e6cbaa7707c08fa222501e69c004fdf4959cebe40abfd8ba304`.

Final tracked and untracked working sources are archived under `source-final/`:
source-manifest SHA256 `1620e0e012345be139374f75ab41859c0fb377b4d43d12c64a5b8f5aa1302ba8`;
ZIP SHA256 `20ba3ec1dd746a37fb37fdaa08ca8a21e50e02d5224c36f20e0109e97d21e510`.
The earlier CLI source snapshot also contains the then-active task-only Cargo
target configuration. The build manifest validates its exact two lines and
the two subsequent test-only MSIX file differences; both release commands
explicitly use `--target-dir dist`. These differences do not change release
behavior. The final source snapshot contains no temporary Cargo config.

No real user-cache cleanup, installation, commit, archive or publication was run.
