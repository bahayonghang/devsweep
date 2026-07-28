# Design: Real Cancellation and Worker Exit

## Token ownership

This task owns the shared `CancellationToken` interface.
It is cloneable, thread-safe, and its cancellation request is monotonic.
Scanner, size walker, provider probe, executor, and ProcessRunner receive its
read-only observer. No layer infers cancellation from UI text or job status.

Checks occur before a root or target starts, between traversal entries, around
provider probes, and before and after executor actions. ProcessRunner turns an
observed request into a real `Canceled` result after terminating interruptible
command trees. Trash and other non-interruptible actions finish their current
operation, then no later action may start.

## Worker confirmation

`Cancelling` means a request exists, not that work stopped. A worker emits one
terminal confirmation only after its current safe boundary has stopped and no
later action can start. The UI then becomes `Canceled`. Race-hardening ignores
late progress, finish, and cancel messages after that terminal transition.

Execution reports distinguish completed actions, canceled-before-start targets,
and partial execution. Scan cancellation returns scan-reliability partial
outcomes. Provider cancellation returns a typed ProcessRunner diagnostic while
retaining unrelated results already obtained.

## Runtime and quit policy

The runtime owns each worker token and `JoinHandle` in a bounded registry.
It releases the dispatch permit only after terminal event and join completion.
`Effect::CancelJob` requests the token and never fabricates `JobCanceled`.

During a mutation, `q` or Ctrl-C opens the D9 quit choice: continue waiting, or
request cancellation and wait. The latter keeps the overlay until the worker is
terminal and joined. There is no detach option, default detach, or exit that
leaves a deletion or command worker behind. Scan jobs use the same token/join
policy but may finish with a partial scan rather than cleanup report.

## Compatibility

Use a no-op adapter while individual consumers migrate. Keep the race task's
truthful boundary-cancel wording until workers report real confirmation.
Rollback removes token propagation and registry together; retaining only a UI
`Canceled` state would recreate the original false claim.
