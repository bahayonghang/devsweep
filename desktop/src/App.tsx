import { useReducer, useState } from "react";
import type { CommandError, ScanOptions } from "./api/types.gen";
import { tauriBridge, type DesktopBridge } from "./api/bridge";
import { decodeCommandError } from "./api/contract";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { ErrorBanner } from "./components/ErrorBanner";
import { ExecutePage } from "./pages/ExecutePage";
import { ReviewPage } from "./pages/ReviewPage";
import { ScanPage } from "./pages/ScanPage";
import { ScanPreviewPage } from "./pages/ScanPreviewPage";
import { appReducer, initialState } from "./state/app-state";
import { hasIrreversibleSelection } from "./state/selectors";

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected desktop error" }; }
}

let scanIdSequence = 0;

function createScanId(): string {
  scanIdSequence += 1;
  const randomId = globalThis.crypto?.randomUUID?.();
  return randomId ? `scan-${randomId}` : `scan-${Date.now()}-${scanIdSequence}`;
}

export function App({ bridge = tauriBridge }: { bridge?: DesktopBridge }) {
  const [state, dispatch] = useReducer(appReducer, initialState);
  const [options, setOptions] = useState<ScanOptions>({ include_projects: true, include_global: true, roots: ["."] });

  const scan = async () => {
    if (state.phase === "scanning" || state.phase === "executing" || state.pending !== null) return;
    const scanId = createScanId();
    let progressError: CommandError | null = null;
    dispatch({ type: "scan_requested", scanId });
    try {
      const result = await bridge.scanStart(
        scanId,
        options,
        (progress) => dispatch({ type: "scan_progressed", progress }),
        (error) => {
          if (progressError) return;
          progressError = commandError(error);
          dispatch({ type: "scan_cancel_requested", scanId });
          dispatch({ type: "command_failed", error: progressError });
          void bridge.scanCancel(scanId).catch(() => undefined);
        },
      );
      if (progressError) dispatch({ type: "scan_failed", scanId, error: progressError });
      else if (result.type === "completed") dispatch({ type: "scan_completed", scanId, report: result.report });
      else dispatch({ type: "scan_canceled", scanId });
    } catch (error) {
      dispatch({ type: "scan_failed", scanId, error: progressError ?? commandError(error) });
    }
  };
  const cancel = async () => {
    const scanId = state.activeScan?.scanId;
    if (!scanId) return;
    dispatch({ type: "scan_cancel_requested", scanId });
    try { await bridge.scanCancel(scanId); }
    catch (error) { dispatch({ type: "command_failed", error: commandError(error) }); }
  };
  const dryRun = async () => {
    if (!state.scan || state.selectedIds.size === 0) return;
    const plan = state.scan.plan;
    const selectedIds = [...state.selectedIds];
    dispatch({ type: "dry_run_requested" });
    try { dispatch({ type: "dry_run_succeeded", outcome: await bridge.planDryRun(plan, selectedIds) }); }
    catch (error) { dispatch({ type: "command_failed", error: commandError(error) }); }
  };
  const execute = async () => {
    if (state.phase !== "confirming" || !state.scan || !state.dryRun) return;
    const plan = state.scan.plan;
    const selectedIds = [...state.selectedIds];
    const digest = state.dryRun.digest;
    dispatch({ type: "execute_requested" });
    try { dispatch({ type: "execute_succeeded", report: await bridge.planExecute(plan, selectedIds, digest) }); }
    catch (error) { dispatch({ type: "command_failed", error: commandError(error) }); }
  };

  const showDryRun = state.dryRun && ["dry_run", "confirming", "executing"].includes(state.phase);
  const preview = state.activeScan
    ? { status: "active" as const, progress: state.activeScan.progress, preview: state.activeScan.preview }
    : state.stoppedPreview
      ? { status: state.stoppedPreview.kind, progress: state.stoppedPreview.progress, preview: state.stoppedPreview.preview }
      : null;
  return <div className="app-shell">
    <header className="app-header"><div className="brand-mark" aria-hidden="true">D</div><div><h1>devsweep</h1><p>Cleanup workbench</p></div><span className="safety-status">Dry-run first</span></header>
    <ScanPage activeScan={state.activeScan} busy={state.phase === "executing" || state.pending !== null} options={options} onOptions={setOptions} onScan={() => void scan()} onCancel={() => void cancel()} />
    {state.error && <ErrorBanner error={state.error} onDismiss={() => dispatch({ type: "error_dismissed" })} />}
    <main className="main-content">
      {state.execution && state.phase === "reported" ? <ExecutePage report={state.execution} final onConfirm={() => undefined} onReturn={() => dispatch({ type: "review_requested" })} />
        : showDryRun && state.dryRun ? <ExecutePage report={state.dryRun.report} final={false} onConfirm={() => dispatch({ type: "confirmation_opened" })} onReturn={() => dispatch({ type: "review_requested" })} />
        : preview ? <ScanPreviewPage status={preview.status} progress={preview.progress} preview={preview.preview} canReturnToReport={state.scan !== null} onReturnToReport={() => dispatch({ type: "stopped_preview_dismissed" })} />
        : <ReviewPage state={state} onSelect={(targetId, selected) => dispatch({ type: "selection_changed", targetId, selected })} onSelectAll={(selected) => dispatch({ type: "select_all_changed", selected })} onDryRun={() => void dryRun()} />}
    </main>
    <ConfirmDialog open={state.phase === "confirming" || state.phase === "executing"} digest={state.dryRun?.digest ?? ""} irreversible={hasIrreversibleSelection(state)} busy={state.phase === "executing"} onCancel={() => dispatch({ type: "confirmation_closed" })} onConfirm={() => void execute()} />
  </div>;
}
