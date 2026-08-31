# Implement - Software Umbrella Orchestration

This umbrella coordinates contracts and recursive evidence only. Do not start
it as a product implementation task. Child work requires a later explicit
approval of the revised task tree.

## Child order and handoffs

1. Complete `08-29-software-inventory-plan` and freeze tagged identities,
   inventory fingerprints, eligibility/refusal reasons, untrusted plan, preview
   digest, and hostile source fixtures.
2. Hand only validated identities and frozen DTOs to
   `08-29-software-execution-audit`; complete current-user MSIX-only strategy
   reconstruction, durable pre-side-effect `dispatch_started`, no-auto-retry
   restart recovery, post-state requery, the closed five terminal outcomes, and
   the locked redacted audit before presentation starts.
3. Complete `08-29-software-cli-tui-desktop-native` against both accepted
   contracts without parser changes, vendor-command parsing, guessed leftovers,
   any MSI/machine-scope execution, or elevation.

## Umbrella acceptance and rollback

- Perform this review only after all three implementation leaves are archived.
  Inspect their product commits plus moved task artifacts under
  `.trellis/tasks/archive/`; do not use archived status alone as acceptance.
- Compare CLI/TUI/Desktop identities, selection eligibility, stable reasons,
  preview/outcome/audit fixtures, redaction, and locale-invariant documents.
- Stop on identity/schema drift, executable registry text, guessed ownership,
  UAC/elevated children, or incomplete post-dispatch evidence; return defects to
  the owning child.
- Register or roll back Software atomically while preserving readable versioned
  audits. Keep missing native standard-user evidence `UNVERIFIED`; archive leaf
  children before this umbrella.
- Persist the review at `.trellis/tasks/08-29-software-mode/acceptance.md`.
  Record reviewed leaf product commit SHAs and archived task paths, every
  umbrella AC clause and its evidence, every command/exit code/log path, all
  manual/native `UNVERIFIED`, and exactly one overall `PASS` or `FAIL`. Overall
  `PASS` requires no completion-required `UNVERIFIED`. Keep this umbrella active
  after PASS; archive it only after `08-29-five-mode-native-integration` passes.
