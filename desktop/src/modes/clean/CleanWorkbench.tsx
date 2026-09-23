import { useEffect, useReducer, useRef, useState } from "react";
import type { CommandError, DesktopScanResult, DryRunOutcome, ExecutionReport, ScanOptions } from "../../api/types.gen";
import type { DesktopBridge } from "../../api/bridge";
import { decodeCommandError } from "../../api/contract";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { formatTotals } from "../../components/format";
import { ExecutePage } from "../../pages/ExecutePage";
import { ReviewPage } from "../../pages/ReviewPage";
import { ScanPage } from "../../pages/ScanPage";
import { ScanPreviewPage } from "../../pages/ScanPreviewPage";
import { message, type PresentationLanguageTag } from "../../i18n";
import { DetailView, StageResult } from "../../stage";
import { hasIrreversibleSelection, selectedTotals } from "../../state/selectors";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { cleanReducer, initialCleanState } from "./reducer";
import "./styles.css";

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

export function CleanWorkbench({ bridge, coordinator, locale }: { bridge: DesktopBridge; coordinator: OperationCoordinator; locale: PresentationLanguageTag }) {
  const [state, dispatch] = useReducer(cleanReducer, initialCleanState);
  const [options, setOptions] = useState<ScanOptions>({ include_projects: true, include_global: true, roots: ["."] });
  const [query, setQuery] = useState("");
  const [overview, setOverview] = useState(false);
  const mounted = useRef(true);

  useEffect(() => () => { mounted.current = false; }, []);

  const scan = async () => {
    if (state.phase === "scanning" || state.phase === "executing" || state.pending !== null) return;
    const scanId = createScanId();
    setOverview(false);
    let progressError: CommandError | null = null;
    let started = false;
    try {
      const lease = await coordinator.start<DesktopScanResult>({
        kind: "clean.scan",
        id: scanId,
        cancel: () => bridge.scanCancel(scanId),
        start: () => {
          started = true;
          if (mounted.current) dispatch({ type: "scan_requested", scanId });
          return bridge.scanStart(
            scanId,
            options,
            (progress) => {
              if (mounted.current) dispatch({ type: "scan_progressed", progress });
            },
            (error) => {
              if (progressError) return;
              progressError = commandError(error);
              if (mounted.current) {
                dispatch({ type: "scan_cancel_requested", scanId });
                dispatch({ type: "command_failed", error: progressError });
              }
              void bridge.scanCancel(scanId).catch(() => undefined);
            },
          );
        },
      });
      if (!lease) return;
      try {
        const result = await lease.result;
        if (!mounted.current) return;
        if (progressError) dispatch({ type: "scan_failed", scanId, error: progressError });
        else if (result.type === "completed") dispatch({ type: "scan_completed", scanId, report: result.report });
        else dispatch({ type: "scan_canceled", scanId });
      } finally {
        await lease.complete().catch(() => false);
      }
    } catch (error) {
      if (started && mounted.current) {
        dispatch({ type: "scan_failed", scanId, error: progressError ?? commandError(error) });
      }
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
    const operationId = `dry-run-${Date.now()}`;
    let started = false;
    try {
      const lease = await coordinator.start<DryRunOutcome>({
        kind: "clean.dry-run",
        id: operationId,
        cancel: () => undefined,
        start: () => {
          started = true;
          if (mounted.current) dispatch({ type: "dry_run_requested" });
          return bridge.planDryRun(plan, selectedIds);
        },
      });
      if (!lease) return;
      try {
        const outcome = await lease.result;
        if (mounted.current) dispatch({ type: "dry_run_succeeded", outcome });
      } finally { await lease.complete().catch(() => false); }
    } catch (error) {
      if (started && mounted.current) dispatch({ type: "command_failed", error: commandError(error) });
    }
  };
  const execute = async () => {
    if (state.phase !== "confirming" || !state.scan || !state.dryRun) return;
    const plan = state.scan.plan;
    const selectedIds = [...state.selectedIds];
    const digest = state.dryRun.digest;
    const operationId = `execute-${Date.now()}`;
    let started = false;
    try {
      const lease = await coordinator.start<ExecutionReport>({
        kind: "clean.execute",
        id: operationId,
        cancel: () => undefined,
        start: () => {
          started = true;
          if (mounted.current) dispatch({ type: "execute_requested" });
          return bridge.planExecute(plan, selectedIds, digest);
        },
      });
      if (!lease) return;
      try {
        const report = await lease.result;
        if (mounted.current) dispatch({ type: "execute_succeeded", report });
      } finally { await lease.complete().catch(() => false); }
    } catch (error) {
      if (started && mounted.current) dispatch({ type: "command_failed", error: commandError(error) });
    }
  };

  const showDryRun = state.dryRun && ["dry_run", "confirming", "executing"].includes(state.phase);
  const preview = state.activeScan
    ? { status: "active" as const, progress: state.activeScan.progress, preview: state.activeScan.preview }
    : state.stoppedPreview
      ? { status: state.stoppedPreview.kind, progress: state.stoppedPreview.progress, preview: state.stoppedPreview.preview }
      : null;
  const reported = state.execution !== null && state.phase === "reported";
  const scanning = state.activeScan !== null;
  const emptyReport = Boolean(state.scan && state.scan.plan.targets.length === 0 && !scanning && !state.stoppedPreview);
  const idleHome = state.scan === null && state.stoppedPreview === null && !scanning;
  const showStage = scanning || idleHome || emptyReport;
  const showMain = reported || Boolean(showDryRun && state.dryRun) || preview !== null || (state.scan !== null && !emptyReport);
  const filteredState = query.trim() && state.scan
    ? {
        ...state,
        scan: {
          ...state.scan,
          plan: {
            ...state.scan.plan,
            targets: state.scan.plan.targets.filter((target) =>
              `${target.path ?? ""} ${target.id}`.toLowerCase().includes(query.trim().toLowerCase()),
            ),
          },
        },
      }
    : state;
  const detailVisible = showMain && !showStage && !overview;
  const overviewVisible = showMain && !showStage && overview;
  const errorBanner = state.error && <ErrorBanner error={state.error} onDismiss={() => dispatch({ type: "error_dismissed" })} />;
  const mainContent = showMain ? <div className="main-content">
    {reported && state.execution ? <ExecutePage locale={locale} report={state.execution} final onConfirm={() => undefined} onReturn={() => dispatch({ type: "review_requested" })} />
      : showDryRun && state.dryRun ? <ExecutePage locale={locale} report={state.dryRun.report} final={false} onConfirm={() => dispatch({ type: "confirmation_opened" })} onReturn={() => dispatch({ type: "review_requested" })} />
      : preview ? <ScanPreviewPage locale={locale} status={preview.status} progress={preview.progress} preview={preview.preview} canReturnToReport={state.scan !== null} onReturnToReport={() => dispatch({ type: "stopped_preview_dismissed" })} />
      : <ReviewPage locale={locale} state={filteredState} onSelect={(targetId, selected) => dispatch({ type: "selection_changed", targetId, selected })} onSelectAll={(selected) => dispatch({ type: "select_all_changed", selected })} onDryRun={() => void dryRun()} />}
  </div> : null;
  const scanPage = <ScanPage locale={locale} stage={showStage} empty={emptyReport} activeScan={state.activeScan} busy={state.phase === "executing" || state.pending !== null} options={options} onOptions={setOptions} onScan={() => void scan()} onCancel={() => void cancel()} />;
  return <div className="clean-mode">
    {showStage ? <>
      {scanPage}
      {errorBanner}
      {mainContent}
    </> : overviewVisible ? <>
      <StageResult
        mode="clean"
        label={message(locale, "command.clean")}
        caption={message(locale, "stage.v1.caption.estimated")}
        value={formatTotals(selectedTotals(state))}
        meta={state.scan ? <p>{message(locale, "clean.v1.scan.complete", { count: String(state.scan.plan.targets.length) })}</p> : undefined}
        action={<button type="button" className="stage-action" onClick={() => setOverview(false)}>{message(locale, "stage.v1.action.details")}</button>}
      />
      {errorBanner}
    </> : detailVisible ? <DetailView locale={locale} onBack={() => setOverview(true)}>
      {scanPage}
      {state.phase !== "scanning" && state.scan && !emptyReport ? <div className="scan-toolbar"><label>{message(locale, "clean.v1.search.label")} <input value={query} onChange={(event) => setQuery(event.target.value)} /></label></div> : null}
      {errorBanner}
      {mainContent}
    </DetailView> : null}
    <ConfirmDialog locale={locale} open={state.phase === "confirming" || state.phase === "executing"} digest={state.dryRun?.digest ?? ""} irreversible={hasIrreversibleSelection(state)} busy={state.phase === "executing"} onCancel={() => dispatch({ type: "confirmation_closed" })} onConfirm={() => void execute()} />
  </div>;
}
