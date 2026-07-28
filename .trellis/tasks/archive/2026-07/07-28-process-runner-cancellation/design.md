# Design: Bounded Process Runner

## Port and result contract

Introduce a single `ProcessRunner` port used by provider probes and executor
commands. A request carries program and argv as separate `OsString` values,
an explicit cwd policy, per-command timeout, inherited whole-job deadline,
per-stream byte cap, and a cancellation observer supplied by the future
true-cancellation owner. The runner returns a typed result that distinguishes
successful output from `NotFound`, `Timeout`, `Exit { code }`,
`InvalidOutput`, and `Canceled`. It does not collapse failures into `None`.

The runner owns process lifecycle, bounded capture, neutral working directory,
and display/audit sanitization. Provider-specific interpretation of a successful
output remains in `07-28-rule-registry-providers`; executor-specific audit
outcomes remain in `07-28-durable-audit`.

## Bounded I/O and deadlines

Spawn stdout and stderr readers immediately and retain only the tail of each
stream in a bounded ring buffer (default 1 MiB per stream, configurable through
the runner policy). Readers drain pipes even after the cap so a noisy child
cannot block on a full OS pipe. Output records total observed bytes and a
truncation flag. Non-UTF-8 bytes are converted lossily only at the presentation
boundary, never through `unwrap`.

The effective deadline is the earlier of the request timeout and job deadline.
The supervisor checks cancellation and deadline while readers drain. On timeout
or cancellation it starts tree termination, waits a bounded grace period, then
returns the typed status with retained sanitized tail output. A nonzero exit is
not interpreted as invalid output until the caller's parser applies its own
format rule.

## Process-tree backends

Use a small private `ProcessTreeBackend` abstraction. Unix starts each command
in a dedicated process group and signals the group for timeout/cancel so child
and grandchild processes are included. Windows assigns the child to a Job
Object configured to kill processes on close; timeout/cancel closes or
terminates that job. The platform implementation must use native APIs rather
than shelling out to `kill`, `taskkill`, or a command string.

The expected direct dependencies are `windows-sys` for Job Object APIs and a
small Unix FFI surface such as `libc` for group signaling. They are production
dependencies and require maintainer approval before activation. The design
must be validated with real child/grandchild fixtures on Windows and Unix;
compile-only evidence is insufficient.

## Cwd, cancellation, and sanitization

The default cwd is a created neutral temporary directory, not the caller's
project directory. A caller must explicitly request a cwd and its reason.
This prevents global provider discovery from inheriting `.npmrc` or equivalent
project-local configuration by accident.

`07-28-true-cancellation` owns the named cancellation-token type. Until it is
implemented, runner integration uses an agreed no-op observer adapter. This
task must not define a competing shared token module. The runner checks the
observer before spawn, while waiting, and before returning success.

`sanitize_process_output` is the only display/audit conversion: it removes or
escapes terminal control sequences, bounds output to the configured tail, and
marks truncation. Durable audit consumes this function instead of retaining
raw stderr. It is not a parser and does not change stdout bytes before a
provider parser has validated them.

## Migration and rollback

Migrate `ProcessCommandRunner` and provider `command_output` through the port
without changing their policy decisions. Global sweeps aggregate per-provider
typed diagnostics and obey a 30-second phase deadline; a timed-out provider
does not suppress other providers. Rollback restores existing call adapters but
must retain tree, timeout, output, and neutral-cwd fixtures for the next fix.
