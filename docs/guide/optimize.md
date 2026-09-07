# Optimize catalogue and maintenance

Optimize mode presents a closed eight-operation Windows maintenance catalogue. It does not invent operations, request elevation, or treat opening Windows Settings as completed maintenance. Run it as a standard user.

The catalogue is the only rendering source. Each row has a stable id and one of three badges:

- **Runs here** — `dns.flush` executes native System32 `ipconfig.exe /flushdns` for at most 10 seconds. No other command is composed or searched on `PATH`.
- **Opens Windows Settings** — `settings.storage_recommendations`, `settings.search`, and `settings.energy_recommendations` open a frozen Settings page. A successful launch is recorded as **launched**, never as an optimization completion. DevSweep does not change the setting.
- **Guidance only** — `guidance.drive_optimize`, `guidance.system_integrity`, `guidance.filesystem_check`, and `guidance.network_reset` are display-only. They have no preview or run action.

Storage and Search Settings rows require Windows build 22000 or later. Energy recommendations require build 22624 or later. DevSweep consumes the core typed build query (`RtlGetVersion`). If that query fails, the Settings rows are unavailable; DevSweep does not guess a version.

Optimize follows the same authority chain as Clean: list the catalogue, plan one exact operation, preview for a live digest, then `--confirm`. Guidance cannot be planned. The list/catalogue document is not a runnable plan.

```powershell
devsweep optimize list --format json --output optimize-list.json
devsweep optimize plan --operation dns.flush --output dns-plan.json
devsweep optimize preview --plan dns-plan.json
```

Run requires the saved plan, the live `sha256:` digest from that preview, and `--confirm`. Confirm the grammar with `--help`; do not run `optimize run` as a documentation check:

```powershell
devsweep optimize run --help
devsweep optimize run --plan dns-plan.json --preview-digest sha256:DIGEST --confirm
```

Cancellation before dispatch, failure, timeout, and unknown after dispatch are distinct terminals. Recovery reads the Optimize audit journal and does not redispatch. The journal is stored only at `%LOCALAPPDATA%\DevSweep\audit\v1\optimize.jsonl`. Legacy `%APPDATA%\devsweep\audit.jsonl` is not used or imported.

TUI and Desktop follow the same staged states: checking, ready, selected, previewing, preview-ready, confirming, running (DNS only) or launching (Settings only), then terminal or unknown. The sticky summary never calls a Settings launch or guidance display a completed optimization.
