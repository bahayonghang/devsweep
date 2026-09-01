# Bad fixture

This file exists only to fail the output checker.

```text
rm -rf node_modules
Remove-Item -Recurse D:\cache
software uninstall --plan software.json --preview-digest sha256:dead --confirm
optimize run --plan opt.json --preview-digest sha256:dead --confirm
```
