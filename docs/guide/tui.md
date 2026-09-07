# Terminal UI

Start the interactive terminal UI with bare `devsweep` from an interactive
stdin and stdout TTY:

```powershell
devsweep
```

From this repository:

```powershell
cargo run --locked --bin devsweep
```

There is no `tui` subcommand. `devsweep tui` is rejected (exit 2). `just dev`
currently still passes `tui`; that recipe is a later release-contract fix and is
not the current tutorial.

The UI is an interactive view over the same scanning, planning, rule, and
execution contracts used by the CLI. It does not create a separate cleanup
policy. Bare invocation requires both stdin and stdout to be TTYs; redirected
streams exit 2 with `tty_required`.

## Use it for review

Use the UI to inspect scan results, size completeness, safety diagnostics, rule
information, and selected cleanup targets before deciding whether a saved plan
should be executed. Follow the key hints shown in the running UI; they are the
authoritative controls for the current build.

## Keep the safety boundary

The UI does not make cleanup implicit. Scanning remains observational. Any
execution path still depends on a validated plan, selected targets, a live
preview digest, and `--confirm` (or the matching in-UI confirmation).

For a scriptable or auditable workflow, use [scan](/guide/scan) to save an
observation, then follow the [saved-plan cleanup](/guide/clean) flow: exact
select, new plan, live preview digest, then confirm.
