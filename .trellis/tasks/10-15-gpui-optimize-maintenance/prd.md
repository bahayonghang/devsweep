# C06 Bounded Windows maintenance through the CLI

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C02](../10-15-gpui-windows-action-boundary/prd.md).

## Goal

Extend the closed maintenance catalogue with preflighted native repairs and honest per-action outcomes.

## Requirements

C06-R1. Retain the eight existing catalogue entries and distinguish actual operation, Settings handoff and guidance.

C06-R2. Research and implement finite Start/Search refresh, orphan startup repair, due SSD retrim and Windows health-check actions where exact Windows APIs/providers are supportable; explorer icon/thumbnail caches remain owner-managed guidance until a separately justified safe action is reviewed.

C06-R3. Preflight privilege, AC/battery, update/reboot state, volume capability and operation cooldown; show cost, applicability and reason for skipped work.

C06-R4. Support a reviewed multi-action plan only after single-action semantics are proven; one UI button means review then execute, with per-action progress/results.

C06-R5. Separate health checks from repair: long-running or network-dependent repair is a distinct review/action, not an automatic follow-on to a health scan.

## Acceptance criteria

- [ ] C06-AC1: Closed operation catalogue documents executable versus guidance status and supported Windows builds; no silent service/security/power-policy tweaks.
- [ ] C06-AC2: UAC refusal, unsupported hardware, low battery, ongoing update, cooldown and changed preconditions prevent only the affected action and report the reason.
- [ ] C06-AC3: Cancel/close does not force-kill a system repair; already-dispatched work remains waited on or reported unknown/reboot-needed.
- [ ] C06-AC4: Fake runners plus disposable native Windows evidence prove side effects and requery; no productivity-machine repairs are used as tests.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
