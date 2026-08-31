# Implement - Optimize Catalogue, Execution, and Audit

Start only after later approval and the frozen CLI contract. Do not add catalogue
entries or dependencies during implementation.

## 1. Add the exhaustive catalogue and authorizer

1. Encode exactly eight stable ids: DNS execution, three Settings handoffs, and
   four guidance entries from `design.md`.
2. Add V1 plan, live preflight, non-deserializable validated action, digest,
   one-operation permit, and locked durable audit.
3. Snapshot every id/action identity/build predicate/refusal and prove unknown or
   rejected categories cannot resolve.

Focused validation:

```powershell
rtk cargo test -p devsweep-core optimize::catalogue
rtk cargo test -p devsweep-core optimize::authorizer
```

Rollback point: disable the catalogue version/constructors together and retain
readable audits; never leave ids backed by a generic command adapter.

## 2. Implement fixed Windows adapters

1. Add the exact `IsWow64Process2` -> `Sysnative`/`GetSystemDirectoryW` resolver
   and fail-closed path identity checks.
2. Dispatch only exact `ipconfig.exe` plus `/flushdns` through the bounded process
   runner for 10 seconds.
3. Enable `Wdk_System_SystemServices`, resolve build with `RtlGetVersion`, and
   fail unavailable on query error without `GetVersionExW` or environment-text
   fallback. Launch only the three literal Settings URIs through
   `ShellExecuteExW`, and report only `launched`.
4. Keep all four guidance ids out of every dispatch enum.

Focused validation:

```powershell
rtk cargo test -p devsweep-core optimize::windows
rtk cargo test -p devsweep-core optimize::execution
```

Rollback point: remove DNS/Settings adapters and mark their ids unavailable;
never fall back to PATH, shell, environment variables, SysWOW64, registry tweaks,
or administrator commands.

## 3. Frozen CLI, full, and native gates

Implement only `optimize list|plan|preview|run` in the owned handler.

```powershell
rtk cargo test -p devsweep-cli optimize
rtk git diff --check
rtk just ci
```

Native x64 and, where buildable, x86-on-x64 evidence records resolved path,
program/argv, process tree, output bounds, timeout/cancel/unknown, exact Settings
page launch, unsupported build, policy/launch failure, audit, and no UAC. Native
side effects are limited to the separately approved fixed
`ipconfig.exe /flushdns` action and opening the three frozen Settings URIs; do
not change a Windows setting or treat an opened page as completed maintenance.
Stop for any URI/id/build/path change or request to claim Settings work completed.
