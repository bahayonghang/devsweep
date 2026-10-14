# Release CLI output acceptance

2026-10-13. Candidate C1 CLI SHA-256:
`a5881ebf98eed803581619db2cf0cb967093cf0151887c67b274511047701277`.
Driver: `tools/status-sink-smoke.py`, SHA-256
`5e572a2b0f0622d2367395491180cef4537376ed247f45cfdc28ccc27f24ff94`.
Raw records and exact argv remain in ignored
`dist/performance/cleanup-earth/C1-status-sink-01/`.

## Observed results

- Actual `status live --format ndjson --interval 1` used a fresh isolated
  profile and an unread OS pipe for 22.03 s. The driver then consumed five
  complete records at 500 ms intervals. All were valid V1 records for the same
  operation, with consecutive sequences 0–4: started, snapshot, tick-skipped,
  snapshot, snapshot.
- Closing the reader triggered the existing graceful broken-pipe path. The
  owned CLI exited 0 naturally; observed pipe-close-to-exit was 5.52 ms. This
  single observation is not a universal cancel deadline or performance claim.
- `status snapshot --format json --output <fresh-file>` emitted exactly one
  valid V1 envelope and returned the existing partial exit class 5. The second
  attempt returned refusal 6; original bytes and SHA-256 remained unchanged.
- All three owned CLI processes exited naturally. No forced termination,
  cleanup, uninstall or maintenance was invoked. Real process observations
  remain in ignored raw output; this tracked record contains aggregate facts.

Command from the repository root:

```text
python tools/status-sink-smoke.py dist/performance/cleanup-earth/C1-runtime/devsweep.exe dist/performance/cleanup-earth/C1-status-sink-01
```

## Evidence boundaries

This checks a real slow reader and broken pipe, plus named JSON create-new
behavior. It does not observe a terminal on the pipe after the reader closes,
measure an exact live queue size, artificially slow filesystem writes, or
establish portable timing. The deterministic production-channel regressions
separately prove the 16+1+1 retained-record bound and lossless terminal/cancel/join
behavior with the receiver alive and undrained. Output writer/create-new/error
regressions and the actual broken-pipe integration test passed in `just ci`.
The blocked synchronous OS write remains non-interruptible until the reader
progresses or closes; no durability or per-event sync policy was relaxed.

The independent checker hardened the driver with bounded pipe polling,
natural-exit deadlines, exact owned PID/creation records and unconditional
failure artifacts. A timeout reports the owned process still running and stops
subsequent measurements; it cannot become a passing cancellation record.
