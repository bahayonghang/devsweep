# Implement - Optimize Umbrella Orchestration

This umbrella coordinates contracts and recursive evidence only. Do not start
it as a product implementation task. Child work requires a later explicit
approval, and the TPR-01 OS-build mechanism must remain resolved in the accepted
plan.

## Child order and handoff

1. Complete `08-29-optimize-catalog-execution`: freeze all eight ids, action
   classes, manifest-independent `RtlGetVersion` preflight through
   `Wdk_System_SystemServices`, fixed DNS identity/argv, Settings URIs,
   authorizer, outcomes, and locked audit.
2. Review exhaustive catalogue/build-query/refusal fixtures and record the typed
   handoff before `08-29-optimize-cli-tui-desktop-native` starts.
3. Complete presentation/native work without querying the OS build again,
   manufacturing ids/argv, changing parser/catalogue decisions, or describing a
   Settings launch/guidance item as completed maintenance.

## Umbrella acceptance and rollback

- Perform this review only after both implementation leaves are archived.
  Inspect their product commits plus moved task artifacts under
  `.trellis/tasks/archive/`; do not use archived status alone as acceptance.
- Compare all surfaces for id, action class, capability/refusal, digest, terminal
  outcome, locale-invariant schemas, exact DNS process evidence, Settings launch
  evidence, audit, unsupported builds, and no-UAC behavior.
- Stop on catalogue/build/URI/path drift, fallback version helpers, privilege
  expansion, or an unresolved native gate; return defects to the owning child.
- Register or roll back Optimize atomically while preserving readable audits;
  archive leaf children before this umbrella.
- Persist the review at `.trellis/tasks/08-29-optimize-mode/acceptance.md`.
  Record reviewed leaf product commit SHAs and archived task paths, every
  umbrella AC clause and its evidence, every command/exit code/log path, all
  manual/native `UNVERIFIED`, and exactly one overall `PASS` or `FAIL`. Overall
  `PASS` requires no completion-required `UNVERIFIED`. Keep this umbrella active
  after PASS; archive it only after `08-29-five-mode-native-integration` passes.
