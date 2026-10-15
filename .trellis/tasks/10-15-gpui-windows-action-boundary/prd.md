# C02 Windows action authority and elevation boundary

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md).

## Goal

Provide narrowly scoped Windows action authorization, privilege handoff, durable outcomes and reversible-state records.

## Requirements

C02-R1. Define finite operation identities and capability states for user-scope, elevated, unsupported and external-confirmation actions.

C02-R2. Bind live preview to exact provider/file/registry identity, selected scope, tool and action preconditions; no arbitrary path/argv or shell executor in an elevation request.

C02-R3. Run the normal app as standard user. Privileged actions use a short-lived, authenticated native helper only when a reviewed action requires it; no persistent service.

C02-R4. Audit before dispatch, preserve cancellation versus unknown-after-dispatch, version backups and verify read-back. Record no permission prompt as successful authorization.

## Acceptance criteria

- [ ] C02-AC1: Forged/replayed/stale/cross-user requests and tool replacement fail before dispatch; helper accepts only a closed typed operation set.
- [ ] C02-AC2: UAC decline, helper disconnect, audit failure, timeout and crash are truthful terminal outcomes with no automatic redispatch.
- [ ] C02-AC3: Isolated registry/file fixtures prove backup integrity, conditional restore and current identity verification; no host-wide registry or policy change.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
