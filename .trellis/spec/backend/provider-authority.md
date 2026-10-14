# Global owner-command authority

## 1. Scope and trigger

Read this contract when changing global command discovery, tool resolution,
configured cache roots, plan reconstruction, preview digests or dispatch.
`rules/definitions.rs` remains the sole catalogue; this guide is not a second
source of rule IDs or supported-version ranges.

## 2. Boundaries

- `rules/configuration.rs` resolves the closed configured-root policy and
  validates reviewed provider versions.
- `process/executable.rs` and `process/provider_dispatch.rs` resolve the same
  executable path for discovery and execution.
- `execution/provider.rs` creates a private, live `ProviderBinding` and extends
  the core confirmation digest. Bindings are not serialized plan authority.
- `Executor` obtains fresh bindings before comparing the preview digest and
  rechecks the matching binding immediately before dispatch.

## 3. Required contracts

The v2 observation/plan continues to carry typed intent, never caller-supplied
program, argv or cwd. The trusted registry reconstructs the owner command.
Discovery runs bounded read-only queries only and reports an explicit coverage
outcome for failed, missing, unsupported, ambiguous or truncated probes.

For each selected global command, preview must bind all of:

- The normalized, live provider cache footprint matching the observation.
- The resolved executable and its content hash.
- A launcher/interpreter or fixed support script and its content hash when the
  command requires one.
- A fully validated version/capability response within a reviewed range.
- Relevant allowlisted nonsecret cache configuration, including its contents
  when changing a same-path configuration file could alter the command.

Do not retain full environment dumps, credentials or entire config contents in
evidence, logs or reports. Hash relevant configuration privately. A failed
binding is a refusal with rescan/review guidance, never a fallback to stale
authority. A legacy pathless provider observation cannot acquire a new live
footprint merely by being loaded as a saved plan.

Windows package-manager shims require a closed supported layout. A recognized
Node package entry executes native `node.exe` with the fixed CLI script as a
separate argv element; bind the shim, script and native binary. Never execute
untrusted `.cmd` text or compose a shell string. Unsupported layouts, including
unrecognized wrappers, remain explicitly unavailable.

On Unix, a supported Node entry must also use the closed package path and exact
`#!/usr/bin/env node` declaration. Resolve a native interpreter explicitly and
bind both its identity and the entry script; do not let an implicit shebang
interpreter escape the preview digest. Corepack and arbitrary wrappers remain
unsupported. Cross-platform source/tests do not substitute for native platform
execution evidence.

Yarn discovery and execution share `rules::yarn_configuration()`. Include the
selected supported rc file, including case-insensitive `YARN_RC_FILENAME`
selection, in the relevant configuration fingerprint. `yarnPath`/`yarn-path`
and environment delegation are explicitly refused rather than followed. Closed
parsing must reject ambiguous YAML forms and detect indented or quoted root keys
even around directives/document markers. An unchanged rc filename is not proof
that its relevant contents are unchanged.

The Yarn Classic 1.22.22 search policy also includes `YARN_CONFIG`,
`~/.config/yarn/config`, `~/.config/yarn`, `~/.yarn/config`, the platform
`CONFIG_DIRECTORY` (Windows `%LOCALAPPDATA%/Yarn/Config`, Unix
`$XDG_CONFIG_HOME/yarn` or the upstream home/root fallback), and Unix
`/etc/yarn/config` and `/etc/yarnrc`. Inspect these before any provider query.
Relative configuration selectors, unreadable/nonregular files and rc `--...`
arguments are unsupported. Such arguments can change cwd or load another rc;
never silently ignore them or expand them into command authority. Ordinary
directories and absent rc files remain valid missing-file cases. Fingerprint
only relevant nonsecret values and the selected source paths.

Gradle, NuGet and inspect-only Go-directory overrides use the same trusted
resolver in discovery, registry reconstruction and live safety. Relative,
protected, reparse or otherwise unsafe paths do not become valid merely because
an environment variable names them. Cargo home remains inspect-only.

Live safety, explicit selection, confirmation and durable execution audit still
apply. An owner command is irreversible and not selected by default. Observed
store occupancy is not a promise of its eventual pruning yield.

## 4. Errors and drift

| Condition | Required outcome |
| --- | --- |
| Missing tool | Explicit absent coverage; no cleanup authority |
| Unsupported version or wrapper | Explicit unsupported/refusal reason |
| Yarn delegated entry or unsupported rc syntax | Refuse before provider dispatch; do not ignore the setting |
| Ambiguous/truncated/failed path probe | Incomplete coverage; no inferred path |
| Tool, interpreter or same-path script content changes after preview | Reject stale confirmation before cleanup |
| Relevant config content or footprint changes after preview | Reject or require rescan before cleanup |
| Identity changes between initial binding and dispatch | Refuse that dispatch through the executor outcome/audit path |
| Empty but successfully inspected cache | Empty coverage, distinct from failure |

## 5. Examples

A pip launcher binds both its resolved launcher and reported interpreter.
A standard npm Node shim binds the `.cmd`, `npm-cli.js` and `node.exe` contents
while executing no command-shell text. An unknown shim receives a supported
layout explanation instead of silently acquiring equivalent authority.

## 6. Required verification

Use separate-process fixtures with private PATH/home/config to avoid mutating
global test environment. Exercise real scan, saved report, v2 plan validation,
live preview and simulated owner-command execution. Include spaces, Unicode,
Python fallback, supported/unsupported version boundaries and same-path binary,
script and relevant config replacement. Sentinel files must remain intact.
Do not replace this chain with a test-only validated-plan constructor.
Cover custom Yarn rc selection, indented/quoted delegation, document markers,
environment case variants and an unchanged entry script with a replaced native
Node interpreter. Unrelated credential content must not affect the binding.
Classic fixtures also cover the extra user/platform/explicit config sources,
same-path content changes, relative selectors and rc argument refusal. Record
all simulated provider calls so discovery and preview tests prove refusal
before even a version/cache query, not merely before simulated cleanup.

## 7. Wrong and correct

Wrong: execute a saved `program` string, trust an old cache path, infer shell
arguments from wrapper text, or treat an unchanged file path as unchanged tool
identity.

Correct: reconstruct a closed command, query and hash current authority, compare
the current confirmation digest, then perform the same live checks at dispatch.
