# Terminal UI

Start the interactive terminal UI with:

```powershell
just dev
```

The UI is an interactive view over the same scanning, inventory, rule, plan,
and execution contracts used by the CLI. It does not create a separate cleanup
policy.

## Use it for review

Use the UI to inspect scan results, size completeness, safety diagnostics, rule
information, and selected cleanup targets before deciding whether a saved plan
should be executed. Follow the key hints shown in the running UI; they are the
authoritative controls for the current build.

## Keep the safety boundary

The UI does not make cleanup implicit. Scanning and inventory remain
observational. Any execution path still depends on a validated plan, selected
targets, and the same live authorization checks used by `clean --execute`.

For a scriptable or auditable workflow, use [scan](/guide/scan) to save JSON,
then follow the [saved-plan cleanup](/guide/clean) flow.
