# Research: Linux audit lock lifetime under concurrent process spawn

- Query: Can inherited Unix file descriptions explain the C16 Optimize suite lock failures, and what is the smallest correct repair?
- Scope: internal source/log review plus a bounded Linux kernel reproduction; no product changes or product tests.
- Date: 2026-10-15

## Findings

### Result and evidence boundary

The checker's proposed mechanism is **deterministically reproduced**: a forked child retaining a duplicate of a flock-held open file description keeps the lock after the parent's owning file is closed. Explicit `LOCK_UN` before closing releases it immediately, even while that child is held at a pipe barrier. Close-on-exec does not prevent the fork-to-exec lifetime window.

The source and failure locations are consistent with this mechanism. The historical suite logs do not identify the actual holder PID or file description, so those exact failures are not independently traced to a specific child. This report proves the OS mechanism and identifies an applicable release-lifetime defect; it does not relabel either failed suite as passed.

### Files found / source anchors

| File and anchor | Observation |
| --- | --- |
| `dist/c16/evidence/linux-rustc188-cargo198-test-isolated.log:723` | `recovery_never_redispatches_pending_operations` fails at `optimize/tests.rs:1172` with `Audit(LockUnavailable("/tmp/.tmpXOtvRb/optimize.lock"))`; total 455 passed, 1 failed. |
| `dist/c16/evidence/linux-rust198-test-reviewed.log:846` | `lock_conflict_fails_closed_and_preserves_bytes` unexpectedly fails while opening its own post-run journal at test line 1193; `one_operation_permit_serializes_concurrent_runs` fails reopening after both threads join at line 1247. Total 454 passed, 2 failed. |
| `crates/devsweep-core/src/optimize/audit.rs:260` | Journal opens fixed `optimize.lock` in its own parent directory before reading/appending the journal. |
| `crates/devsweep-core/src/optimize/audit.rs:572` | `SidecarLock` owns a `File`; Unix acquisition uses `flock(LOCK_EX | LOCK_NB)` at line 612. No explicit unlock/Drop implementation was present when inspected. |
| `crates/devsweep-core/src/software/execution/audit.rs:938` | The Software sibling has the same close-only lock lifetime and Unix `flock` acquisition at line 978. It is susceptible even though the cited failures are Optimize tests. |
| `crates/devsweep-core/src/process/tree.rs:16` | Unix process setup registers a `pre_exec` closure for `setpgid(0, 0)` at line 23. |
| `crates/devsweep-core/src/process/mod.rs:161` | The shared runner configures that backend before `command.spawn()` at line 163. |
| `crates/devsweep-core/src/optimize/execution.rs:233` | Execute holds the process-wide permit and opens a journal; recovery has the equivalent boundaries at line 269. |
| `crates/devsweep-core/src/optimize/tests.rs:1181` | The intentional lock-conflict test fails before its intended conflicting execution, immediately after an earlier successful execution. |
| `crates/devsweep-core/src/optimize/tests.rs:1205` | Concurrent operations both succeed and their threads join; the failure is the next independent journal open. |

Relevant specs: backend `quality-guidelines.md:1110-1113` requires retaining failing receipts even if isolated checks later pass; `:1098-1101` requires isolated Cargo output without exporting scanner-visible `CARGO_TARGET_DIR`; `:344-360` requires exclusive Software audit locking and fail-closed dispatch. The task asks to preserve the corresponding Optimize fail-closed behavior. No retry, assertion relaxation, lock-file deletion or test serialization is needed to correct descriptor lifetime.

### Why this fits whole-suite-only failures

1. Test A acquires an Optimize journal lock in a unique temporary directory.
2. Unrelated test B spawns a child through the Unix ProcessRunner. `fork` duplicates the process's file descriptors, including A's lock file, even though B does not know A's temporary path.
3. A finishes and drops its journal. Closing A's descriptor is not the last reference while B's pre-exec child still has the inherited descriptor.
4. A immediately opens a new journal. That fresh `open` creates a different open file description and its nonblocking exclusive flock fails.
5. B reaches exec (or exits), closing the inherited descriptor, so a later retry or isolated test can pass.

The Optimize mutex coordinates Optimize calls in the parent process. It does not coordinate unrelated ProcessRunner forks. Unique temporary directories prevent shared-path collisions, but cannot prevent inheritance of the parent's already-open descriptors.

Linux documents that flock ownership follows the open file description; fork/dup references share that lock, explicit unlock can release it, and close releases it only after the last reference disappears. [Linux man-pages flock(2)](https://man7.org/linux/man-pages/man2/flock.2.html). Rust documents that `pre_exec` runs after fork with duplicated resources, and that standard-library close-on-exec descriptors close at exec. [Rust CommandExt](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.pre_exec). Sources accessed 2026-10-15; the fetched Rust page identifies std 1.99.0 and is used for the documented lifecycle, not as evidence that C16 compiled with that version.

## Bounded reproduction

Executed at `2026-10-15T06:42:22.067366+00:00` using existing WSL `Ubuntu-24.04`, Python `3.12.3`, kernel `6.18.54.1-microsoft-standard-WSL2`.

- Used only an anonymous `memfd` with `MFD_CLOEXEC`, an independent reopen through `/proc/self/fd`, and two pipes for ready/release barriers. No on-disk lock, product file, real audit store, cleanup action or host setting was touched.
- Asserted a real conflict while the owner remained alive.
- Waited for the child to acknowledge that its inherited descriptor was open, then tested parent release. No sleeps or scheduling-dependent loop.
- Ran 10 close-only and 10 explicit-unlock cases. All met their expected assertions, and all children were waited/reaped.
- Sandbox WSL discovery returned `Wsl/E_ACCESSDENIED`; the same read-only discovery and isolated reproduction worked with approved `require_escalated`. No installation was performed.

Captured output:

```json
{"utc":"2026-10-15T06:42:22.067366+00:00","python":"3.12.3","kernel":"6.18.54.1-microsoft-standard-WSL2","storage":"anonymous memfd; no filesystem output"}
{"iterations":10,"passed":10,"result":{"explicit_unlock":false,"close_on_exec":true,"acquire_while_inherited_fd_open":false,"acquire_after_child_exit":true}}
{"iterations":10,"passed":10,"result":{"explicit_unlock":true,"close_on_exec":true,"acquire_while_inherited_fd_open":true,"acquire_after_child_exit":true}}
```

Reproduction source, preserved here rather than creating another artifact. Run by piping it to `wsl.exe -d Ubuntu-24.04 --exec python3 -`:

```python
import errno, fcntl, json, os, platform, select, sys
from datetime import datetime, timezone

def lock_available(fd):
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return True
    except OSError as exc:
        if exc.errno not in (errno.EAGAIN, errno.EACCES):
            raise
        return False

def run_case(explicit_unlock):
    held = os.memfd_create('devsweep-c16-flock-proof', os.MFD_CLOEXEC)
    probe = os.open('/proc/self/fd/' + str(held), os.O_RDWR | os.O_CLOEXEC)
    fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
    assert not lock_available(probe), 'live owner must exclude independent opener'
    cloexec = bool(fcntl.fcntl(held, fcntl.F_GETFD) & fcntl.FD_CLOEXEC)
    ready_r, ready_w = os.pipe()
    release_r, release_w = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(ready_r)
        os.close(release_w)
        os.close(probe)
        os.write(ready_w, b'R')
        os.close(ready_w)
        os.read(release_r, 1)
        os._exit(0)
    os.close(ready_w)
    os.close(release_r)
    try:
        assert select.select([ready_r], [], [], 5)[0], 'child ready timeout'
        assert os.read(ready_r, 1) == b'R'
        if explicit_unlock:
            fcntl.flock(held, fcntl.LOCK_UN)
        os.close(held)
        held = -1
        available = lock_available(probe)
        assert available == explicit_unlock
        if available:
            fcntl.flock(probe, fcntl.LOCK_UN)
        os.write(release_w, b'X')
        os.close(release_w)
        release_w = -1
        _, status = os.waitpid(pid, 0)
        pid = -1
        assert status == 0
        assert lock_available(probe), 'all old references closed'
        fcntl.flock(probe, fcntl.LOCK_UN)
        return {'explicit_unlock': explicit_unlock, 'close_on_exec': cloexec,
                'acquire_while_inherited_fd_open': available,
                'acquire_after_child_exit': True}
    finally:
        if held != -1: os.close(held)
        os.close(ready_r)
        if release_w != -1: os.close(release_w)
        if pid != -1: os.waitpid(pid, 0)
        os.close(probe)

print(json.dumps({'utc': datetime.now(timezone.utc).isoformat(),
                  'python': sys.version.split()[0], 'kernel': platform.release(),
                  'storage': 'anonymous memfd; no filesystem output'}))
for mode in (False, True):
    rows = [run_case(mode) for _ in range(10)]
    print(json.dumps({'iterations': len(rows), 'passed': len(rows),
                      'result': rows[0]}))
```

## Smallest correct repair and tests

### Product repair recommendation

Add explicit Unix `flock(fd, LOCK_UN)` release to `SidecarLock` destruction while its `File` is still valid, in both Optimize and Software's matching implementation. Ordinary File destruction then closes the descriptor. Keep Windows share-mode behavior unchanged and retain current `LOCK_EX | LOCK_NB` acquisition/error classification. No new dependency or general lock abstraction is necessary for these two small owners.

The destructor must not panic. If the implementation handles interrupted unlock, retry only `EINTR`, without sleeping; do not convert acquisition conflicts into waits/retries. Document that release happens when the owning guard ends even if an unrelated fork temporarily retains a duplicate descriptor. Do not remove the sidecar path: unlink/recreate could create a different inode and let two writers each hold a supposedly exclusive lock.

The owning journal still covers audit validation, durable transitions and dispatch. Unlocking at guard destruction therefore does not shorten the intended critical section; it removes an unintended lifetime extension beyond the owner. The guard is not a cloneable authority object, and no child-side Drop should run inside a `pre_exec` closure.

Replacing the process-group `pre_exec` with `CommandExt::process_group(0)` may separately reduce manual unsafe process setup, but is not the minimal lock repair: another fork/dup elsewhere can retain descriptors. Do not broaden this bounded defect repair into process-runner changes solely to hide the trigger.

### Deterministic Rust regression

Use a test-private duplicate of the lock's `File` via `try_clone()`. This duplicates the same open file description and reproduces the essential lifetime without unsafe `fork` inside a multithreaded test harness. Rust documents the shared underlying handle. [Rust File::try_clone](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_clone), accessed 2026-10-15.

For each affected lock owner:

1. Acquire the original guard; clone only its underlying file into a test-held descriptor.
2. Assert an independently opened second guard fails with `LockUnavailable` while the owner is alive, and journal bytes are unchanged.
3. Drop the original guard, keep the duplicate alive, and assert an independent new guard succeeds immediately. This deterministically fails with the inspected close-only implementation.
4. Drop the old duplicate while the new guard is still alive; assert a third independent opener remains refused. This guards against accidentally releasing someone else's newer lock.
5. Drop the new guard; assert the next open succeeds. Check no journal bytes or dispatch counts changed merely from refused acquisition.

Keep any descriptor accessor under `cfg(test)` or place the test inside the lock-owning module; do not expose production lock internals. Preserve existing conflict/recovery/serialization assertions. The existing 50 ms fake-dispatch delay in one unrelated concurrency test is not needed by this new regression and should not be increased to mask failure.

After the checker implements and proves the regression, rerun the affected audit/execution tests and complete core/CLI suite under both required Linux compiler configurations, retaining the original failure logs. Exact-test success alone is insufficient. Native Windows checks remain separate; this mechanism test does not validate Windows file sharing.

## Caveats / Not Found

- No strace/proc-fd trace of the historical failing test processes is available; the particular concurrent child responsible remains unidentified.
- The isolated mechanism test uses Linux shmem-backed anonymous files, not the original `/tmp` filesystem. Its observed fork/open-file-description semantics are the identified defect; full source-level regression and suite verification remain checker's responsibility.
- No product edits, Cargo builds/tests, native cleanup, installations, persistent WSL changes or generated `dist/` scripts/logs were performed by this researcher. The only written file is this report.
- Source anchors were recorded before the checker repair and may shift. Two guessed version-pinned Rust source URLs returned 404; no findings rely on them.
