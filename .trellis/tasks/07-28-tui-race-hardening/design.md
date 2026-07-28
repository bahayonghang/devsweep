# Design: TUI Race Hardening

## Scope and decisions

This phase-zero task hardens the existing TUI control plane without changing
the serialized cleanup-plan format or promising interruptible work before the
true-cancellation task lands. The selected product decision is D5: a scan
event received while a confirmation modal is open immediately invalidates that
modal. The modal remains visible with an explicit re-confirmation message;
Enter performs no side effect and asks the user to close and reopen it.

`07-28-plan-validation` remains the owner of the cross-process canonical
digest and `ActionFingerprint`. This task may normalize scanner paths only to
remove duplicate scan output; it must not create a second public digest or
plan-validation contract.

## TUI state and data flow

Add an app-private `ExecutionManifest` that owns the complete selected
`CleanTarget` snapshot and the display data derived from that same snapshot.
Opening a confirmation builds the manifest once. `ConfirmState` owns it and
has an `invalidated_by_scan` flag. Confirming consumes the frozen manifest to
build the clean effect; it never calls `selected_cleanup_plan()` or reads the
current target/selection state again.

`ScanProgress` and `ScanFinished` first mark an open confirmation invalid,
then apply the normal scan-state update. Rendering shows why confirmation is
disabled. Enter on an invalid confirmation adds a warning log entry and emits
no `StartClean`; Escape closes it. This keeps current scan state responsive
while making the displayed confirmation non-executable.

Selection persistence uses explicit per-target overrides, keyed by stable
`TargetId`. Toggling records `Selected` or `Deselected`; rebuilding a staged
scan preserves an override for a matching target and applies the normal
default only to targets with no prior override. Overrides for targets that no
longer exist are discarded with the old target.

## Job ownership and state machine

The app rejects `c` and scan-starting `s` while a mutation job is active. The
runtime owns a minimal clean-dispatch gate as a second line of defense around
`StartClean`; it rejects a second clean worker before spawning a thread and
releases its permit only after the first worker sends its terminal event. The
gate does not own cancellation tokens, handles, or process trees; those remain
the responsibility of `07-28-true-cancellation`.

All job updates pass through one transition function. Legal transitions are
`Running -> Cancelling`, `Running -> Succeeded|Failed`, and
`Cancelling -> Canceled|Succeeded|Failed`; terminal states remain terminal.
This task removes the runtime's fabricated `JobCanceled` response. Before
true cancellation is implemented, `x` means "stop at the next action
boundary" and the UI never claims an in-flight action has already been
canceled. Delayed worker events after a terminal state are logged and ignored.

## Scanner duplicate behavior

Scanner roots are normalized before coverage reduction. Candidate comparison
uses a private scanner dedupe key consisting of the normalized footprint and
the action identity. Exact duplicate candidates merge evidence into one target;
the same path with a different action remains distinct. Parent/child roots with
the same action keep the smallest covering root. This is a presentation and
scan-quality layer only; the validated-plan once guarantee remains owned by
`07-28-plan-validation`.

## Compatibility and rollback

No CLI or JSON schema changes occur. Existing confirmation text is extended
with an invalidation state, and existing `JobStatus::Canceled` remains usable
for the later real-cancellation implementation. Reverting this task removes
the private manifest/gate/override helpers and restores prior event handling;
no persisted state needs migration.

## Verification risks

The fail-red suite must drive app events directly and observe emitted effects,
not merely rendered text. Runtime tests need a blocking fake clean service so
they can prove a second `StartClean` did not spawn. Path-equivalence coverage
must run on Windows for case and separator behavior; absence of that evidence
is a no-go for declaring the dedupe portion complete, even if non-Windows unit
tests pass.
