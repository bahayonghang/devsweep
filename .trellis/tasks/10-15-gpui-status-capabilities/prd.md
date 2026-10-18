# C08 System status telemetry and process actions

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C02](../10-15-gpui-windows-action-boundary/prd.md).

## Goal

Expand truthful Windows telemetry, local process explanations and explicitly reviewed process control.

## Requirements

C08-R1. Retain bounded CPU/memory/volume/network/battery/process snapshots and lossless CLI live records; improve GPU/VRAM/disk activity and available thermal detail using read-only native sources.

C08-R2. Represent unavailable, unsupported, stale and permission-denied sensor values separately; no fabricated zeroes, temperatures or fan values.

C08-R3. Add a locally explainable health summary and process details; score evidence, weights and unknown coverage must be visible and testable.

C08-R4. If process termination is offered, use a distinct reviewed action bound to PID plus creation identity and user scope, protect critical/shared/system processes, and confirm possible data loss.

C08-R5. Own the Keep Screen On shared service and CLI before native UI integration: explicit finite lease, user confirmation, owner-bound session token, visible status and stop/expiry/owner-exit release. Never modify persistent power settings or release another owner's request.

## Acceptance criteria

- [x] C08-AC1: Counter reset, multiple adapters, missing battery/sensor, protected processes and sample aging are covered; no driver installation is required.
- [x] C08-AC2: Slow NDJSON reader, disconnect, cancellation and full-queue join retain the existing sequence/terminal contracts.
- [x] C08-AC3: PID reuse and process identity drift refuse termination; a fixture-owned process is the only native termination target.
- [x] C08-AC4: Health explanation distinguishes CPU load from a system fault; no opaque numeric score or always-on extra sampler.
- [x] C08-AC5: No-GUI keep-awake start/status/stop behavior proves lease expiry, token ownership, cancellation and owner-exit cleanup using an isolated short-lived request; unsupported platforms explain capability gaps. C13 owns native controls over this accepted service.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
