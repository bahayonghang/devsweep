# Status and terminal worker delivery

## 1. Scope / Trigger

Apply when changing Status live production, CLI/Tauri Status consumption or the
TUI worker mailbox/event loop. Replaceable scan observations and lossless live
records have different delivery requirements; neither may grow an unbounded
application queue behind a slow consumer.

## 2. Signatures

- `spawn_live(LiveRequest) -> Result<(StatusEventReceiver, LiveControl), StatusError>`.
- `StatusEventReceiver::recv_timeout(Duration)` preserves the standard
  `RecvTimeoutError::{Timeout, Disconnected}` results.
- `LiveControl::request_cancel()` requests cooperative stop;
  `LiveControl::join()` requests stop and joins, including on `Drop`.
- TUI-private `runtime::mailbox::{WorkerSender, WorkerReceiver}` transports
  `WorkerEvent`; it is not a public generic channel framework.
- TUI `draw_if_dirty` renders changed state/size. `drain_worker_events` processes
  at most 32 events before returning control to input handling.

## 3. Contracts

### Status

- Retain at most 16 ordinary queued records. A canceled producer may retain its
  one already-generated in-flight record plus one terminal record, for a hard
  maximum of 18. Never drop a generated record to make cancellation fast.
- Full-queue waits release the mutex and observe cancellation at intervals no
  longer than 25 ms. After a canceled send, the producer emits the terminal
  record directly; it must not generate another snapshot or tick-skipped record
  that consumes the terminal reserve.
- Sequence and terminal semantics are unchanged. A slow consumer receives all
  generated records in order, with one terminal record. This is backpressure,
  not snapshot coalescing.
- `LiveControl::join()` must finish with a full queue even while the receiver
  remains alive and undrained. Dropping the receiver wakes any blocked sender;
  dropping the sender wakes the receiver as disconnected after queued records
  are drained.
- CLI broken pipe cancels and joins the producer. Named output remains
  create-new, JSON remains one V1 envelope, and NDJSON retains its closed record
  schema. This change does not relax normal output or cleanup audit durability.

### TUI

- Keep one replaceable latest cumulative scan observation plus a FIFO of at
  most 32 ordered lifecycle records. A single lock orders observation replacement
  and lifecycle/phase flushes; a phase/final event cannot overtake the last
  preceding observation.
- A burst of same-job, same-phase scan snapshots replaces the pending slot.
  Do not drop lifecycle events or coalesce records from different job/phase
  identities as though they were one observation.
- Synchronous dispatch refusals and preference-save results return directly to
  the reducer. The UI must never block sending into its own full worker queue.
- Drain at most 32 events, then service input. Redraw only after visible state,
  input or terminal-size changes, or an actual future animation deadline; idle
  polling alone must not trigger a steady 10 Hz full redraw.
- A terminal event is the worker's last event, before its thread returns.
  In particular, canceled inventory must not send a later ignored finish event
  that could block UI join behind a full queue.
- Drop the mailbox receiver before teardown joins so blocked sends can settle.
  Cooperative read-only work may be canceled and joined; implicit teardown must
  wait for in-flight cleanup/uninstall/maintenance work without introducing a
  new cancellation request. Preserve the existing explicit user-cancel policy.
- Join before replacing a worker; job-id rejection and final-only promotion of
  cleanup facts remain unchanged.

## 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Status consumer stalls with full queue | Bounded retention; producer waits without dropping records |
| Cancel plus join while receiver remains alive | In-flight record and terminal fit; join completes before drain |
| Receiver closes during blocked send | Sender wakes and producer settles |
| Producer closes or panics | Receiver drains retained records, then observes disconnect |
| TUI burst of 10k same-phase observations | Only latest pending observation retained |
| Phase/lifecycle/final follows pending observation | Preserve ordering and one worker terminal |
| Idle event-loop polls with no changed state | No additional draws |
| Teardown during fake mutating work | Wait for completion; do not set an implicit cancel flag |

## 5. Good / Base / Bad Cases

- Good: a stalled Status sink resumes and receives every consecutive sequence,
  including its canceled terminal, after the producer has already joined.
- Good: a large scan cannot make the TUI drain forever before reading a key.
- Base: a resize marks the terminal dirty and redraws once.
- Bad: replacing `mpsc::channel` with a bounded blocking send while shutdown
  joins the producer without dropping/draining the receiver.
- Bad: using the scan's replaceable-slot policy for contractual Status NDJSON.
- Bad: canceling every worker indiscriminately after a terminal-render error.

## 6. Tests Required

- Actual `LiveControl` full-queue join while the receiver is alive, exact retained
  bound, lossless slow-consumer sequences, single terminal, receiver-close wakeup
  and sender disconnect. Preserve real broken-OS-pipe and create-new regressions.
- TUI 10k observation coalescing, phase/final ordering, full FIFO receiver close,
  finite drain/input service and dirty rendering with a `TestBackend`.
- Teardown tests distinguishing read-only cancellation from wait-only mutation,
  plus a canceled inventory worker emitting exactly one terminal.
- Native same-PID Status stop cycles and post-stop resource samples are separate
  from unit tests. A passing queue test is not a CPU or latency measurement.

## 7. Wrong vs Correct

Wrong: block a terminal send behind a full queue, then ask the UI thread to join
the producer before the UI drains that queue.

Correct: reserve the Status cancellation/terminal capacity, or close the TUI
receiver before teardown joins. Preserve ordered records while a consumer is
active, and keep mutation teardown wait-only.
