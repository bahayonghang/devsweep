# Native functional gate evidence

## Focused and complete suites

- Core native complete suite: **417 passed, 0 failed, 3 ignored**, 26.27 s. Command: `cargo test --locked --target-dir dist -p devsweep-core --lib`. The three ignored entries are the existing explicitly invoked Settings launcher and two read-only MSIX diagnostics; no failure was excluded. Raw output: `dist/native-test-isolation/core-suite-isolated-native.txt`.
- CLI all targets: **184 unit + 12 CLI contract + 7 five-mode contract + 3 provider-authority tests passed**. Command: `cargo test --locked --target-dir dist -p devsweep-cli --all-targets`. Raw output: `dist/native-test-isolation/cli-suite-current.txt`.
- Canonical **`just ci` passed, exit 0**, with scoped `CARGO_TARGET_DIR=<repository>/dist` in the native permission context. It ran workspace format/check/tests and all-target Clippy with `-D warnings`, without test filters or exclusions. Alongside the core/CLI counts above, Tauri passed **86 tests with two pre-existing fixture entrypoints ignored**. Raw output: `dist/native-test-isolation/just-ci-functional.txt`, ending `ci complete`.

## Failures found and corrected

1. The real Windows pipeline found that separate inspect-only Temp observations used one logical fingerprint, so trusted whole-plan validation rejected the report. New Windows/generated inspect observations now carry their exact path footprint. The complete seven-scenario Windows fixture passes unchanged after the correction.
2. Two Windows unreadable-directory fixtures used an inherited username ACL that returned success without denying enumeration. They now deny only `*S-1-1-0:(RD)` on their newly created fixture directory, verify `read_dir` actually fails, and restore with RAII even on assertion failure. Neither test silently skips inability to establish the fixture.
3. A default Cargo-target fixture inherited the gate's CARGO_TARGET_DIR override. The production scanner was correctly obeying the environment. The test now launches an exact child with only that override removed and an anti-recursion marker; production Cargo semantics are unchanged.
4. The old automatic native DNS test called the actual System32 executable with `/flushdns`. Its adapter discarded captured diagnostics, so its failed operation cannot be assigned a proven underlying cause from the saved output. The DNS client service was observed running; no service configuration was changed. The test now keeps a read-only real System32 identity check and dispatches to a disposable native executable that records exactly `['/flushdns']`. Existing failure mapping and fixed-request tests remain. No Optimize production behavior changed and no test was skipped.
5. CLI integration tests assumed the OS language was English and opened the user's settings store. Their subprocesses now use private APPDATA/XDG_CONFIG_HOME fixtures; tests of English wording explicitly request English. Production locale resolution and persistence are unchanged. The catalogue-rendering test now declares a viewport derived from the full catalogue size, matching its content-coverage purpose.

## Environment observations retained

Direct binaries under `target/debug/deps` previously ended with `0xc0000005` or `0xc000001d`, and the files were later absent. That cause remains unproven. Workspace-local `dist` builds and preserved copies ran the complete functional suites without those crashes. No shared cache was deleted and no unrelated process was terminated.

The sandbox initially denied the disposable HKCU registry fixture with OS error 5; the same full suite outside the sandbox passed that fixture. This was a native permission-context difference, not a product safety relaxation. The first unsandboxed rerun still failed the old real DNS mutation fixture; that distinct result was retained before its isolation repair.
