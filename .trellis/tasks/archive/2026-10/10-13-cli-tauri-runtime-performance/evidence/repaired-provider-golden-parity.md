# Repaired-provider B3/C2 golden parity

Result: **PASS**, six isolated cases and twelve release CLI scans. The complete
JSON envelopes, stdout bytes, and exit codes match between corrected B3 and C2.
No envelope fields or array order were removed or normalized. This is a
correctness check, with no timing or real-provider performance claim.

Build identities:

- B3 CLI: `1cf858d35b1be1cddaaae6284b605e89b6055024d7162121c09531b5fdc91266`.
- C2 CLI: `a5881ebf98eed803581619db2cf0cb967093cf0151887c67b274511047701277`
  (byte-identical to C1; subsequent changes were desktop-only presentation
  visibility and asynchronous worker limits).
- Native query fixture: `3fe3d70c030e1c0eac6d58a084975197872afbb598f8e7031069d1a3eda9426f`.

## Cases and expected outcomes

Each row passes both full-envelope parity and independent semantic expectations.
Counts are per build.

| Case | Targets | Exit | Health / coverage | Verified behavior |
| --- | ---: | ---: | --- | --- |
| Native Go queries | 2 | 0 | Complete; 22 coverage units | `go.mod_cache.clean` and `go.build_cache.clean` resolve the two declared cache roots, retain built-in action intents, and remain unselected. |
| Configured known roots | 7 | 0 | Complete; 22 coverage units | Gradle caches/wrapper, NuGet and Go honor configured roots; Cargo home, Go directory fallback and Maven remain inspect-only; Ivy is retained in the report. |
| Configured npm cache | 1 | 0 | Complete; 22 coverage units | Standard npm shim resolves through the native Node fixture and fixed npm script; the declared custom cache remains an unselected built-in action. |
| Nested tox/Python | 3 | 0 | Complete; 1 root | `.tox`, nested `.pytest_cache`, and nested `__pycache__` appear; nested targets use the nearer `tox.ini` project boundary. |
| Healthy plus missing root | 1 | 5 | Partial; 2 roots, 1 diagnostic | Healthy Node target survives; failed missing-root coverage and its diagnostic remain in the complete envelope. |
| Yarn delegation | 0 | 0 | Complete; 22 coverage units | Yarn is explicitly Unsupported with delegation detail, no target and no provider query. Unsupported does not imply overall Partial. |

All 21 fixture files (556 bytes total), directory membership, file hashes and
modification timestamps remain unchanged after each scan. The fixtures use
spaces and Unicode paths with fixed modification times. The provider executables,
script entrypoints and configuration files are separately hash-checked before
each run. Provider call logs contain only twelve allowed read-only queries across
the whole batch: Go `env GOMODCACHE`, `version`, `env GOCACHE`, and npm `config get
cache` / `--version`. No simulated cleanup invocation marker was created.

## Isolation, protocol and retained evidence

The new `tools/provider-golden-parity.py` uses a cleared environment with a small
Windows loader-variable whitelist, private home/application-data/config/temp
roots, and a PATH containing only each case's fixture binaries. Every CLI argv is
`clean scan`; project cases receive only explicit fixture roots. Provider logs,
stdout/stderr, process records and result files are outside all scanned fixture,
home, cache and temporary scopes, so B3 cannot change C2's observed data.

The native executable fixtures simulate provider query responses; this checks
the real release scanner/resolver/output pipeline without requiring installed Go
or npm and does not certify those tools' current real-machine configurations.
The npm fixture records `.npmrc` and returns the matching declared query root;
it does not run the actual npm JavaScript implementation. No cleanup, preview,
execute, maintenance, or shell command was invoked.

Preparation launched no subprocess. Runtime and native-acceptance owners released
the exclusive slot before the twelve scans; it was returned before the inherited
idle measurement. All twelve CLI children exited naturally (ten exit 0 and two
expected partial exit 5). Exact PID/creation-identity checks found no remaining
CLI, and Toolhelp32 plus full executable paths found no fixture Go/Node survivor
or unavailable provider identity. An initial CIM inventory was access-denied and
was not accepted as zero-process evidence. There was no forced termination.

Raw evidence is under ignored
`dist/performance/cleanup-earth/provider-golden-01/`:

- `prepared.json`: exact environment/argv, fixture manifests and immutable hashes.
- `driver.py`: original execution driver, SHA-256
  `27e3ab832f6669fa034dfbc3e86ad53605adc9db3766edbcb012dff470cf107e`.
- `runs/<case>/<B3|C2>/`: full stdout/stderr, process identity and run results.
- `result.json`: original execution result, including initial assertion failures.
- `reassessment.json` and `reassessment-driver.py`: offline semantic reassessment.
- `ownership-closeout.json`: exact owned-process completion evidence.

The initial semantic checker compared expected filesystem paths as raw strings.
It incorrectly rejected Rust's Windows `\\?\` project-path prefix and mixed
separators in known-cache paths. All six full-envelope comparisons already passed
on that first run. The checker now compares expected path identity using Windows
case/separator/prefix equivalence; it does not transform either saved envelope.
Offline reassessment hash-checks the saved outputs, repeats semantic checks and
confirms the current fixture manifest. All six pass, without another CLI or
provider subprocess. Both the original failures and driver remain retained.

## Checks

- `python -m py_compile tools/provider-golden-parity.py`: pass.
- `python tools/provider-golden-parity.py prepare dist/performance/cleanup-earth/provider-golden-01`: six fixtures, zero subprocesses.
- `python tools/provider-golden-parity.py run dist/performance/cleanup-earth/provider-golden-01`: all six raw parity checks pass; initial path-only expected-value assertion failures retained.
- `python tools/provider-golden-parity.py reassess dist/performance/cleanup-earth/provider-golden-01`: six semantic checks pass, zero subprocesses.
- Scoped `git diff --check`: pass.

No product Rust/TypeScript source or existing measurement harness was changed.
