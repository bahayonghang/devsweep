# Current-user MSIX desktop review

Reviewed 2026-11-20 in the integration worktree `D:\ds-integrate-1120`, based on local `264e7ac` and remote `324e72b`, with uncommitted Step 10 changes.

## Execution and evidence

- The Software row supplies one selected ID and exact package full name. The worker rebuilds the plan from the current-user MSIX inventory. Core validates the saved plan, TTL, live digest, action class and identity again under the process permit and audit lock. The production dispatch uses the existing `SoftwareExecutor` shared with the CLI. JSON inventory fixtures have no route to this production executor.
- The Review row can prepare another preview. Root Enter and search Enter cannot dispatch MSIX. The labelled Confirm uninstall control checks its captured generation for both click and accessibility activation. Navigation, refresh, selection changes and dismissal discard review authority. Active jobs continue to own the execution slot until terminal delivery.
- DispatchStarted is flushed before the adapter call. The core writes adapter evidence, queries the same package full name, applies the existing MSIX classifier and persists the terminal. The desktop retains the core outcome, install state, restart and error codes. A post-dispatch audit failure projects uncertainty. A subsequent reconciliation never calls the removal adapter again.
- Checked the production and fake paths in `host_review/uninstall.rs`, `host_review.rs`, `software_review.rs`, `shell.rs`, the native review control and the core Software plan, executor, audit and MSIX adapter. The GUI tests call shared preview, dispatch request and presentation code with a fake host. Core tests cover the private authorization and real journal with injected platform boundaries.

## Corrections

1. Preview and execution originally reused one generation. Delivery of an old preview during execution could clear the running job slot and prevent its real result from being delivered. Confirmation now advances the generation. The new `msix_delayed_preview_cannot_unlock_a_running_removal` test checks the slot, exact call count and delivered terminal.
2. Routing all host uninstall rows to MSIX eligibility removed the existing read-only MSI plan review. Other source types now retain their previous read-only path. Repeated activation cannot confirm them. The new `msix_integration_preserves_read_only_msi_plans` test exercises both activations with an eligible isolated MSI and checks for zero host work. This is the Step 10 boundary until a subsequent MSI implementation.

## Checks

- Desktop Cargo format and `git diff --check` passed after these corrections.
- The initial integration GPUI gate logged 248 product and 4 gallery tests passing, with five Step 10 tests, followed by a successful release build. The build had already collected the test targets when the two new regression tests were added.
- The parent owns the serialized post-correction Rust gates. Focused filters are `msix` in the desktop suite and `msix_durable_dispatch_and_terminal_write_failures_preserve_recovery_evidence` in root `devsweep-core`. The final gate record takes precedence for executable results.
- Native desktop review, screenshots, actual Windows deployment and restart behavior were outside this source check. No installed software was changed.
