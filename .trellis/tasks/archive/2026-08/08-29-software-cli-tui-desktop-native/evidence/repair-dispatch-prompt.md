Active task: .trellis/tasks/08-29-software-cli-tui-desktop-native

MINIMAL REPAIR (test-only, single file). You are the trellis-implement sub-agent again; all standing constraints apply. Do not spawn sub-agents; no commit/push.

## The one defect
`desktop/src/App.test.tsx` test "loads and persists the shared language setting without registering unavailable modes" fails: it asserts `expect(screen.queryByRole("tab", { name: "软件" })).not.toBeInTheDocument()` (line ~269). Software is now legitimately registered by this task, so the assertion is stale. The intent of the test — unavailable modes have no placeholder tabs — remains valid.

## The fix
Update that test to assert the NEW truth: the Software tab (软件) IS present with its workbench, while modes that are genuinely still unregistered (e.g. 优化/optimize and 状态/status, plus any other MODE_IDS entries without registrations per `desktop/src/App.tsx`) are absent. Keep the rest of the test intact. Change nothing else in the repo.

## Validation
`npm --prefix desktop run lint` and `npm --prefix desktop run typecheck` must stay exit 0 (run them; note that Vitest is EPERM in your sandbox — the main session will run the full gate). Save logs under `.trellis/tasks/08-29-software-cli-tui-desktop-native/evidence/` with a `repair-` prefix. Append an `implement.jsonl` line. Report the diff and both exit codes.
