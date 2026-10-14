# Independent functional boundary review — 2026-10-13

This is a focused Phase 2.2 review of the cleanup child before corrected runtime
reference B is frozen. It is not a final programme or performance acceptance.
The main session retains the native smoke and full quality gates. Production
files are shared with active implementers; findings below were sent to the
functional owner before edits, so concurrent fixes do not overwrite one another.

## Scope and method

Read the functional PRD, design, implementation plan, coverage matrix, check
context, canonical backend/desktop contracts and the original source-backed
cleanup research. The injected hook pointed at the runtime child; the dispatch's
functional child path was used explicitly instead.

Traced these production paths:

- Global discovery through typed provider outcomes, real report conversion,
  registry reconstruction, live provider binding, confirmation digest and the
  final authorized command. Windows Node shims execute native Node with a fixed
  script argument; shell text is not dispatched.
- Generated project and Windows browser/editor/temp proof, inspect-only
  representation, whole-plan validation, live owner/marker/content checks and
  the existing audit/executor boundary.
- Core root persistence, Tauri scope/scan/rescan commands, closed generated IPC,
  frontend startup/save generation fencing, selection and confirmation
  invalidation, full-report search/pagination and truthful result presentation.
- The native smoke driver's task-owned process/debug-port checks, real bridge
  observation, offline local-resource check and complete CLI/Tauri report
  comparison. No production service is replaced by that driver.

No user cache cleanup, software maintenance, external reference script or
unrelated process termination was performed. Native test/build slots are
serialized with the main session and implementers. Static findings are labeled
separately from executed regression evidence.

## Findings referred for coordinated production fixes

### F1 — Unrepresentable Windows cache roots poison the full plan (P1)

`rules/windows_caches.rs::cache_children` deliberately retains existing unsafe
or wrong-kind children. `scan/global/mod.rs::add_windows_cache_targets` converts
their refusal into an inspect-only target. However,
`rules/registry.rs::resolve_inspect_only` calls
`validate_windows_cache_location`, which rejects a reparse cache root or a
regular file named `Cache`. That scanner-generated inspect target cannot pass
the real registry. Tauri's `clean_dry_run` validates the complete report plan,
so unrelated selected healthy targets also fail. CLI `clean plan` extracts only
the explicit selected subset, so healthy-only CLI selection can avoid this
failure; the claim is not that every CLI plan is blocked.

Minimal regression: registered Chrome Default and Profile 1, valid Preferences,
a regular Default/Cache file, a real Profile 1/Cache directory, plus a healthy
project. Prove full-report-to-plan validation and healthy selection still work,
the malformed cache has precise unavailable coverage, and retained data is
unchanged. Add a root-reparse counterpart, not only a nested reparse child.

Agreed direction: keep unrepresentable candidates in typed coverage, without
weakening executable or external-plan validation. Functional owner is responsible
for production changes and real pipeline tests.

### F2 — Bounded .NET marker search silently reports absence (P1)

`rules/generated.rs::generated_marker` enumerates only the first 1,000 entries
for `.csproj`/`.fsproj`/`.vbproj`, discards entry errors and returns `None` when
the marker is later. `scan_generated_project` treats that as no project and
records no budget/failure diagnostic. The later ordinary walk does not revisit
that directory's marker decision. Supported bin/obj artifacts can disappear
while root coverage claims complete/empty.

Required fix: retain bounded work but distinguish exhausted/unreadable marker
discovery from a verified absent marker; propagate a typed incomplete outcome.
Test a marker beyond the cap plus healthy unrelated targets. Do not replace the
cap with unbounded enumeration. Functional owner is implementing this change.

### F3 — Unix Node provider interpreter is outside the binding (P1)

`process/provider_dispatch.rs` binds Node plus a fixed script for `.cmd`/`.bat`,
but `.js` and extensionless paths pass through unchanged. On Unix a canonical
npm entry can be a Node shebang script; its hash alone does not bind the actual
Node interpreter. `execution/provider.rs` adds an interpreter binding only for
pip. An unchanged entry script, reported version and cache footprint therefore
do not prove the interpreter is unchanged before dispatch.

Primary source verification on 2026-10-13: the official
[npm entry](https://raw.githubusercontent.com/npm/cli/latest/bin/npm-cli.js) and
[Yarn Classic entry](https://raw.githubusercontent.com/yarnpkg/yarn/master/bin/yarn.js)
use Node through an env shebang; the reviewed
[pnpm 10 entry](https://raw.githubusercontent.com/pnpm/pnpm/v10.0.0/pnpm/bin/pnpm.cjs)
is available as the finite-version fixture reference. This is source-confirmed,
not a Unix execution result on this Windows host.

Required fix: closed supported Node-script dispatch with the native interpreter
and script identities, or explicit unsupported status for unbound script
layouts. Add same-path interpreter replacement and unsupported-wrapper
regressions, with actual Unix execution in platform CI. The existing
cross-platform provider contract cannot silently treat unbound scripts as native.

### F4 — Python live marker list disagrees with discovery (P1)

`scan/project/mod.rs::find_python_marker` accepts `tox.ini`, while
`rules/generated.rs::validate_python_scope` omits it and includes `Pipfile`,
which discovery does not accept. A tox.ini-only project emits existing Python
rules and passes registry reconstruction, but execution now refuses all of
them as missing marker. A nearer tox.ini ownership boundary is also omitted by
the new live validator.

Required fix: share the existing five-marker catalogue between discovery and
live validation. Add real tox-only and nested-tox scan/plan/preview/fake-execute
fixtures and a nearer-marker drift refusal. Functional owner owns the fix.

### F5 — New tool providers erase typed probe failures (P1)

The added uv/Bun/Go-build provider loop in `scan/global/mod.rs` unconditionally
sets `Failed` when `output_path(command_output(...))` returns none. This
overwrites `Timeout`, `Canceled`, `OutputTruncated` or `Unsupported` already
recorded by `SystemProviderProbe`. Existing provider outcome distinctions must
also apply to these three rows.

Required fix: preserve an existing probe failure and use `Failed` only for
otherwise successful malformed/ambiguous path output. Test all three rules and
the individual typed statuses. Functional owner has the finding.

### F6 — Yarn custom configuration and delegated entry need binding (P1)

`execution/provider.rs` includes `YARN_RC_FILENAME` in the environment
fingerprint but reads only fixed home `.yarnrc`/`.yarnrc.yml` filenames. Relevant
same-path content changes in the selected custom rc are not hashed. The allowed
configuration keys also omit `yarnPath`; Yarn can redirect commands to another
entry script whose identity is not included in the closed shim binding.

The official [configuration documentation](https://yarnpkg.com/configuration/yarnrc)
describes `yarnPath` delegation. Its
[Configuration source](https://raw.githubusercontent.com/yarnpkg/berry/master/packages/yarnpkg-core/sources/Configuration.ts)
implements `getRcFilename` and `checkYarnPath`. These primary sources confirm
the relevant mechanisms; no user Yarn installation was invoked.

Required narrow resolution: fingerprint the selected supported rc and bind the
actual supported delegated entry, or visibly refuse unsupported delegation.
Do not interpret arbitrary wrapper text or add shell dispatch. Parent and
functional owner have this finding; final disposition/testing remains pending.

Follow-up review of the proposed closed rc parser found that skipping every
indented line also skips a valid uniformly indented YAML root `yarnPath` key.
The locally installed YAML 2.9.0 parser confirmed that
`  yarnPath: custom.cjs` parses as a root delegation entry. The owner was asked
to cover that case and Yarn's case-insensitive rc-filename environment lookup.
This validation only parsed a string; it did not invoke a user provider.
The same check confirmed indented roots with YAML directives or `...` / `---`
document markers, including trailing comments; minimum indentation alone is
not a sufficient refusal check. The official Configuration source also applies
normalized `YARN_YARN_PATH` environment settings before its path delegation
check, so that direct environment entry is part of the same refusal contract.

### F7 — Reviewed sizing can remove an inspect-only refusal explanation (P2)

The new generated-output eligibility proof stores its unavailable reason in a
`PathUnresolved` sizing warning. `rescan_target_size_with_probe` replaces the
entire warning vector with the new size estimate's warnings. The new desktop
row action offers reviewed rescan on every path-backed row, including
inspect-only rows. Thus a successful size rescan retains the inspect-only intent
but loses its only busy/unknown-owner/secret-proof explanation.

This does not grant cleanup authority. It violates useful refusal/recovery
presentation: the user sees inspection-only with no reason after explicitly
requesting more information. The parent accepted this fix for F06. Generated
proof failures do not currently create an exact-target typed health diagnostic
that could simply be reused. The agreed narrow direction is to rerun current
eligibility for inspect-only targets with a closed generated/Windows rule and
append its current refusal after replacing sizing warnings. A successful new
proof still requires a fresh scan; rescan must not promote the saved intent.
No arbitrary old warning retention, detail-string classification or new wire
field is needed.

## Findings fixed by this reviewer

- `.trellis/spec/backend/error-handling.md`: replaced the stale hard-error rule
  for selected-root access with the approved partial-report/healthy-sibling
  continuation contract; retained invalid empty project scope.
- `.trellis/spec/backend/quality-guidelines.md`: synchronized the missing-root
  matrix row with failed-root coverage and CLI partial exit semantics.

These are surgical documentation corrections. The stale inline scanner comment
was handed to the functional owner with its nearby code fixes.

## Verification and open acceptance

- Documentation `git diff --check`: pass for the two reviewer-edited specs.
- Production lint/type-check/native tests: main-session gates remain authoritative;
  no duplicate native build was started during the reserved build slot.
- Frontend full-check evidence handed to this review: 429 Vitest and 5 Node tests
  passing after pagination, with lint/type-check/build. This is implementation
  handoff evidence, not an independently repeated run in this review.
- The production findings require coordinated regression results and a
  refreshed corrected B before backend runtime optimization. Earlier B artifacts
  must be retained as superseded evidence, not renamed into a corrected result.
- Packaged native two-volume/unrelated-CWD scan, offline Earth, full-report
  CLI/Tauri parity, final root `just ci`, and forthcoming runtime/cancel/idle
  gates are owned by the parent programme and remain outside this focused check.

This file will be updated with actual regression results before this check is
reported complete; no unresolved production finding is waived here.

## Follow-up review and F6 correction — 2026-10-13

The resumed independent review traced F1-F7 through current production code
and the assertions recorded in [functional review regressions](functional-review-regressions.md).
F1, F2, F3, F4, F5 and F7 now match their requested corrections. In particular,
F4 previews before adding a nearer tox marker and proves execute reaches no
runner; F7 covers generated and Windows busy/unknown-owner rows without
promoting Inspect Only. Their broader executed test counts remain the owner's
recorded evidence, not a second independent run here.

F6 initially remained open: the rc collector missed Yarn Classic's additional
configuration sources. The pinned upstream
[rc search](https://raw.githubusercontent.com/yarnpkg/yarn/v1.22.22/src/util/rc.js),
[platform config directory](https://raw.githubusercontent.com/yarnpkg/yarn/v1.22.22/src/util/user-dirs.js),
and [startup delegation](https://raw.githubusercontent.com/yarnpkg/yarn/v1.22.22/src/cli/index.js)
show that an unseen `yarn-path` can redirect even a version query. Its
[rc argument handling](https://raw.githubusercontent.com/yarnpkg/yarn/v1.22.22/src/rc.js)
also accepts `--...` settings that can change cwd or load another rc file.
These are source-confirmed mechanisms; no installed Yarn or user cache was run.

After the coordinator released the B2 freeze, this reviewer made the following
focused corrections:

- `rules/configuration.rs` includes `YARN_CONFIG`, the Classic user rc paths,
  platform `CONFIG_DIRECTORY`, and Unix system paths. Relative selectors,
  unreadable/nonregular config files and rc argument injection are refused.
  Relevant source paths/content are hashed through the existing binding;
  unrelated credential values remain excluded.
- `provider_authority.rs` adds the real CLI scan -> saved v2 plan -> preview ->
  simulated execute regression for missed user/platform/explicit rc sources.
  It rejects same-path content drift. Each source's delegation/argument cases
  refuse both discovery and preview without any provider invocation.
- `process_fixture.rs` records all fixture provider calls, making the last
  assertion stronger than checking only that cleanup was not dispatched.
- The canonical provider-authority spec now records these same requirements.

The new CLI suite passes 5 tests (14.79 s test time) and core configuration
passes 3 tests (0.01 s), using `--locked --target-dir dist` on Windows.
Unix source branches are included but not executed on this host. F1's root
reparse evidence remains an injected no-follow probe, not a native link test.
B2 is superseded and preserved; a fresh corrected runtime reference is required.

README and both scan guides agree with current scope controls, coverage,
recheck-size and full-report pagination behavior. Provenance agrees with the
conversion script and packaged notices. The source archive and both derivative
SHA-256 hashes were rechecked; Natural Earth's official catalogue/terms confirm
the source identity and redistribution statement. No claim of native packaging,
offline runtime or Unix execution is inferred from this documentation review.

### Reviewer verification and handoff

| Command | Result |
| --- | --- |
| `cargo test --locked --target-dir dist -p devsweep-cli --test provider_authority -- --nocapture` | 5 passed, including the final relative-selector regression; 14.79 s test time |
| `cargo test --locked --target-dir dist -p devsweep-core --lib rules::configuration:: -- --nocapture` | 3 passed; 0.01 s test time |
| `cargo check --locked --target-dir dist -p devsweep-core -p devsweep-cli --all-targets` | Passed |
| `cargo clippy --locked --target-dir dist -p devsweep-core -p devsweep-cli --all-targets -- -D warnings` | Passed |
| `cargo fmt --all -- --check` and scoped `git diff --check` | Passed |

The focused functional review checkpoint is closed with no remaining F1-F7
production findings. Source/test files were frozen and the Cargo slot released
to the coordinator for the corrected B3 reference. This is not final programme
acceptance: full root `just ci`, packaged native/parity/offline checks, runtime
measurements and Unix CI remain their independently recorded gates.
