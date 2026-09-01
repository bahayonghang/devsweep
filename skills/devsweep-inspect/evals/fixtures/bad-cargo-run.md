# Bad fixture

This file exists only to fail the output checker.

```text
cargo run --locked -p devsweep-cli --bin devsweep -- clean scan --root .
target\debug\devsweep.exe clean execute --plan plan.json --confirm
target\release\devsweep.exe clean execute --plan plan.json --confirm
```
