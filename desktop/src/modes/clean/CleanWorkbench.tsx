import { Fragment, useEffect, useReducer, useRef, useState, type ReactNode } from "react";
import type { CleanMovedTotalsV1, CommandError, DesktopScanProgress, DesktopScanResult, DryRunOutcome, ExecutionReport, ScanTotals, UntrustedTarget } from "../../api/types.gen";
import type { DesktopBridge } from "../../api/bridge";
import { decodeCommandError } from "../../api/contract";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { formatBytes } from "../../components/format";
import { ExecutePage } from "../../pages/ExecutePage";
import { ReviewPage } from "../../pages/ReviewPage";
import { ScanPage } from "../../pages/ScanPage";
import { ScanPreviewPage } from "../../pages/ScanPreviewPage";
import { message, type PresentationLanguageTag } from "../../i18n";
import { DetailView, Stage, StageResult } from "../../stage";
import { hasIrreversibleSelection, selectedTotals } from "../../state/selectors";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { movedTotals } from "./result";
import { ProtectDialog } from "./ProtectDialog";
import { cleanReducer, initialCleanState } from "./reducer";
import { ScanCoverage } from "./ScanCoverage";
import { ScanRoots } from "./ScanRoots";
import { useScanScope } from "./useScanScope";
import "./styles.css";

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected desktop error" }; }
}

/** Localized capacity labels: verified, lower bound, and unknown stay separate. */
function capacityParts(locale: PresentationLanguageTag, totals: ScanTotals): string[] {
  const parts: string[] = [];
  if (totals.verified_bytes > 0) parts.push(message(locale, "clean.v1.capacity.verified", { bytes: formatBytes(totals.verified_bytes) }));
  if (totals.partial_lower_bound_bytes > 0) parts.push(message(locale, "clean.v1.capacity.partial", { bytes: formatBytes(totals.partial_lower_bound_bytes) }));
  if (totals.unknown_target_count > 0) parts.push(message(locale, "clean.v1.capacity.unknown"));
  return parts;
}

/** One muted line; each fact stays a separate text node. */
function MetaLine({ parts }: { parts: readonly string[] }) {
  return <p>{parts.filter(Boolean).map((part, index) => <Fragment key={index}>{index > 0 ? " · " : null}<span>{part}</span></Fragment>)}</p>;
}

let scanIdSequence = 0;
function createScanId(): string {
  scanIdSequence += 1;
  const randomId = globalThis.crypto?.randomUUID?.();
  return randomId ? `scan-${randomId}` : `scan-${Date.now()}-${scanIdSequence}`;
}

export function CleanWorkbench({ bridge, coordinator, locale, onAnalyzeRoot }: { bridge: DesktopBridge; coordinator: OperationCoordinator; locale: PresentationLanguageTag; onAnalyzeRoot?: (root: string) => void }) {
  const [state, dispatch] = useReducer(cleanReducer, initialCleanState);
  const scope = useScanScope(bridge, () => dispatch({ type: "scope_changed" }));
  const { options } = scope;
  const [query, setQuery] = useState("");
  const [view, setView] = useState<"stage" | "detail">("stage");
  const [protectTarget, setProtectTarget] = useState<UntrustedTarget | null>(null);
  const [protectBusy, setProtectBusy] = useState(false);
  const [protectError, setProtectError] = useState<CommandError | null>(null);
  const [cumulative, setCumulative] = useState<CleanMovedTotalsV1 | null>(null);
  const mounted = useRef(true);
  const scanRequest = useRef<{ id: string; started: boolean; canceled: boolean } | null>(null);

  // StrictMode replays this effect. Setup must restore the flag after cleanup.
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);

  const scan = async (targetId?: string) => {
    if (scanRequest.current || scope.loading || scope.saving || (options.include_projects && options.roots.length === 0) || state.phase === "scanning" || state.phase === "executing" || state.pending !== null) return;
    const scanId = createScanId();
    const request = { id: scanId, started: false, canceled: false };
    scanRequest.current = request;
    setView("stage");
    // Invalidates the digest before waiting for another mode's cancellation.
    dispatch({ type: "scan_requested", scanId });
    let progressError: CommandError | null = null;
    try {
      const lease = await coordinator.start<DesktopScanResult>({
        kind: "clean.scan",
        id: scanId,
        cancel: () => bridge.scanCancel(scanId),
        start: () => {
          if (request.canceled) return Promise.resolve({ type: "canceled" as const, scan_id: scanId });
          request.started = true;
          const onProgress = (progress: DesktopScanProgress) => {
              if (mounted.current) dispatch({ type: "scan_progressed", progress });
            };
          const onError = (error: unknown) => {
              if (progressError) return;
              progressError = commandError(error);
              if (mounted.current) {
                dispatch({ type: "scan_cancel_requested", scanId });
                dispatch({ type: "command_failed", error: progressError });
              }
              void bridge.scanCancel(scanId).catch(() => undefined);
            };
          return targetId
            ? bridge.scanRescanTarget(scanId, options, targetId, onProgress, onError)
            : bridge.scanStart(scanId, options, onProgress, onError);
        },
      });
      if (!lease) {
        if (mounted.current) dispatch({ type: "scan_failed", scanId, error: commandError(new Error("The desktop operation coordinator is closed.")) });
        return;
      }
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
      if (mounted.current) {
        dispatch({ type: "scan_failed", scanId, error: progressError ?? commandError(error) });
      }
    } finally {
      if (scanRequest.current === request) scanRequest.current = null;
    }
  };
  const cancel = async () => {
    const scanId = state.activeScan?.scanId;
    if (!scanId) return;
    dispatch({ type: "scan_cancel_requested", scanId });
    const request = scanRequest.current;
    if (request?.id === scanId) {
      request.canceled = true;
      if (!request.started) return;
    }
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
      if (!lease) {
        if (mounted.current) dispatch({ type: "command_failed", error: commandError(new Error("The desktop operation coordinator is closed.")) });
        return;
      }
      try {
        const outcome = await lease.result;
        if (mounted.current) {
          dispatch({ type: "dry_run_succeeded", outcome });
          setView("detail");
        }
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
          if (mounted.current) {
            dispatch({ type: "execute_requested" });
            setCumulative(null);
          }
          return bridge.planExecute(plan, selectedIds, digest);
        },
      });
      if (!lease) {
        if (mounted.current) dispatch({ type: "command_failed", error: commandError(new Error("The desktop operation coordinator is closed.")) });
        return;
      }
      try {
        const report = await lease.result;
        if (mounted.current) {
          dispatch({ type: "execute_succeeded", report });
          setView("stage");
          void bridge.historyCleanTotals().then((totals) => { if (mounted.current) setCumulative(totals); }).catch(() => undefined);
        }
      } finally { await lease.complete().catch(() => false); }
    } catch (error) {
      if (started && mounted.current) dispatch({ type: "command_failed", error: commandError(error) });
    }
  };

  const protect = async () => {
    const target = protectTarget;
    if (!target?.path || protectBusy) return;
    setProtectBusy(true);
    setProtectError(null);
    try {
      await bridge.protectionAdd(target.path, true);
      if (!mounted.current) return;
      dispatch({ type: "target_protected", targetId: target.id });
      setProtectTarget(null);
    } catch (error) {
      if (mounted.current) setProtectError(commandError(error));
    } finally {
      if (mounted.current) setProtectBusy(false);
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
  const stageVisible = showMain && !showStage && view === "stage";
  const detailVisible = showMain && !showStage && view === "detail";
  const busy = state.phase === "executing" || state.pending !== null;
  const errorBanner = state.error && <ErrorBanner error={state.error} onDismiss={() => dispatch({ type: "error_dismissed" })} />;
  const mainContent = showMain ? <div className="main-content">
    {reported && state.execution ? <ExecutePage locale={locale} report={state.execution} targets={state.scan?.plan.targets} final onConfirm={() => undefined} onReturn={() => dispatch({ type: "review_requested" })} />
      : showDryRun && state.dryRun ? <ExecutePage locale={locale} report={state.dryRun.report} targets={state.scan?.plan.targets} final={false} onConfirm={() => dispatch({ type: "confirmation_opened" })} onReturn={() => dispatch({ type: "review_requested" })} />
      : preview ? <ScanPreviewPage locale={locale} status={preview.status} progress={preview.progress} preview={preview.preview} canReturnToReport={state.scan !== null} onReturnToReport={() => dispatch({ type: "stopped_preview_dismissed" })} />
      : <ReviewPage
          locale={locale}
          state={state}
          query={query}
          onSelect={(targetId, selected) => dispatch({ type: "selection_changed", targetId, selected })}
          onSelectAll={(selected) => dispatch({ type: "select_all_changed", selected })}
          onDryRun={() => void dryRun()}
          onSkip={(targetId) => dispatch({ type: "target_skipped", targetId })}
          onRestore={(targetId) => dispatch({ type: "target_restored", targetId })}
          onProtect={(target) => { setProtectError(null); setProtectTarget(target); }}
          onRescan={(target) => void scan(target.id)}
          onAnalyze={onAnalyzeRoot ? (target) => { if (target.path) onAnalyzeRoot(target.path); } : undefined}
        />}
  </div> : null;
  const scanPage = <ScanPage locale={locale} stage={showStage} empty={emptyReport} incomplete={state.scan?.health.completeness === "partial"} activeScan={state.activeScan} busy={busy || scope.loading || scope.saving} options={options} onOptions={scope.changeOptions} onScan={() => void scan()} onCancel={() => void cancel()}
    rootControls={<ScanRoots locale={locale} roots={options.roots} suggestions={scope.suggestions} disabled={busy || scanning || scope.loading || scope.saving} onChange={scope.changeRoots} />} />;
  const rescanLink = <button type="button" className="stage-link" disabled={busy || (!options.include_projects && !options.include_global)} onClick={() => void scan()}>{message(locale, "clean.v1.action.scan")}</button>;
  const detailsAction = <button type="button" className="stage-action" onClick={() => setView("detail")}>{message(locale, "stage.v1.action.details")}</button>;
  let stage: ReactNode = null;
  if (stageVisible && reported && state.execution) {
    const moved = movedTotals(state.execution);
    const hasMoves = state.execution.outcomes.some((outcome) => outcome.status.type === "succeeded" && outcome.action.type === "move_to_trash");
    const commands = state.execution.outcomes.filter((outcome) => outcome.status.type === "succeeded" && outcome.action.type === "command").length;
    const counts = message(locale, "clean.v1.result.counts", { succeeded: String(state.execution.succeeded), skipped: String(state.execution.skipped), failed: String(state.execution.failed) });
    stage = <StageResult
      mode="clean"
      label={message(locale, "command.clean")}
      className="clean-stage clean-result"
      caption={message(locale, hasMoves ? "clean.v1.result.caption" : commands > 0 ? "clean.v1.result.command_action" : "clean.v1.result.no_changes")}
      value={hasMoves ? formatBytes(moved.verified_bytes + moved.partial_lower_bound_bytes) : String(commands)}
      meta={<>
        <MetaLine parts={[counts, ...capacityParts(locale, moved)]} />
        {hasMoves && <p>{message(locale, "clean.v1.trash.moved")}</p>}
        {commands > 0 && <p>{message(locale, "clean.v1.result.commands", { count: String(commands) })} {message(locale, "clean.v1.result.commands_caption")}</p>}
        {cumulative && <p className="clean-cumulative">{message(locale, cumulative.lower_bound ? "clean.v1.result.cumulative_at_least" : "clean.v1.result.cumulative", { bytes: formatBytes(cumulative.known_bytes) })}</p>}
      </>}
      action={detailsAction}
      secondary={rescanLink}
    />;
  } else if (stageVisible && preview) {
    const totals = preview.preview?.totals;
    stage = <Stage
      mode="clean"
      label={message(locale, "command.clean")}
      className="clean-stage clean-stopped"
      title={message(locale, preview.status === "failed" ? "clean.v1.stage.stopped_failed" : "clean.v1.scan.canceled")}
      meta={totals ? <MetaLine parts={[message(locale, "clean.v1.stage.found_meta", { count: String(totals.target_count) }), ...capacityParts(locale, totals)]} /> : undefined}
      primary={detailsAction}
      secondary={rescanLink}
    />;
  } else if (stageVisible && state.scan) {
    const totals = state.scan.health.totals;
    const selected = selectedTotals(state);
    const selectedBytes = formatBytes(selected.verified_bytes + selected.partial_lower_bound_bytes);
    const hasSelection = state.selectedIds.size > 0;
    const reviewAction = <button type="button" className={hasSelection ? "stage-link" : "stage-action"} onClick={() => setView("detail")}>{message(locale, "clean.v1.action.review")}</button>;
    stage = <StageResult
      mode="clean"
      label={message(locale, "command.clean")}
      className="clean-stage clean-found"
      caption={message(locale, "stage.v1.caption.estimated")}
      value={selectedBytes}
      busy={state.pending === "dry_run"}
      meta={<>
        <MetaLine parts={[message(locale, "clean.v1.stage.found_meta", { count: String(state.scan.plan.targets.length) }), ...capacityParts(locale, totals)]} />
        {hasSelection
          ? <p>{message(locale, "clean.v1.preview.selected", { count: String(state.selectedIds.size) })}</p>
          : <p role="status">{message(locale, "clean.v1.stage.none_selected")}</p>}
        {state.protectedIds.size > 0 && <p role="status">{message(locale, "clean.v1.review.rescan_hint")}</p>}
      </>}
      action={hasSelection
        ? <button type="button" className="stage-action" disabled={busy} onClick={() => void dryRun()}>{message(locale, "clean.v1.action.clean_selected", { bytes: selectedBytes })}</button>
        : reviewAction}
      secondary={hasSelection ? <>{reviewAction}{rescanLink}</> : rescanLink}
    />;
  }
  return <div className="clean-mode">
    {scope.sessionOnly && <p role="status">{message(locale, "clean.v1.scope.session_only")}</p>}
    {scope.error !== null && <ErrorBanner error={commandError(scope.error)} onDismiss={scope.dismissError} />}
    {showStage ? <>
      {scanPage}
      {errorBanner}
      {mainContent}
    </> : stage ? <>
      {stage}
      {errorBanner}
    </> : detailVisible ? <DetailView locale={locale} onBack={() => setView("stage")}>
      {scanPage}
      {state.phase !== "scanning" && state.scan && !emptyReport && !preview ? <div className="scan-toolbar"><label>{message(locale, "clean.v1.search.label")} <input value={query} onChange={(event) => setQuery(event.target.value)} /></label></div> : null}
      {errorBanner}
      {mainContent}
    </DetailView> : null}
    {state.scan && !scanning && !detailVisible && <ScanCoverage locale={locale} health={state.scan.health} />}
    <ConfirmDialog locale={locale} open={state.phase === "confirming" || state.phase === "executing"} digest={state.dryRun?.digest ?? ""} irreversible={hasIrreversibleSelection(state)} busy={state.phase === "executing"} onCancel={() => dispatch({ type: "confirmation_closed" })} onConfirm={() => void execute()} />
    <ProtectDialog locale={locale} path={protectTarget?.path ?? null} busy={protectBusy} error={protectError} onCancel={() => { setProtectTarget(null); setProtectError(null); }} onConfirm={() => void protect()} onDismissError={() => setProtectError(null)} />
  </div>;
}
