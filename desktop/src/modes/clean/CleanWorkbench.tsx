import { Fragment, useEffect, useReducer, useRef, useState, type ReactNode } from "react";
import type { CleanMovedTotalsV1, CommandError, DesktopScanResult, DryRunOutcome, ExecutionReport, ScanOptions, ScanTotals, UntrustedTarget } from "../../api/types.gen";
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
import { hasIrreversibleSelection } from "../../state/selectors";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { movedTotals } from "./result";
import { ProtectDialog } from "./ProtectDialog";
import { cleanReducer, initialCleanState } from "./reducer";
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

export function CleanWorkbench({ bridge, coordinator, locale }: { bridge: DesktopBridge; coordinator: OperationCoordinator; locale: PresentationLanguageTag }) {
  const [state, dispatch] = useReducer(cleanReducer, initialCleanState);
  const [options, setOptions] = useState<ScanOptions>({ include_projects: true, include_global: true, roots: ["."] });
  const [query, setQuery] = useState("");
  const [view, setView] = useState<"stage" | "detail">("stage");
  const [protectTarget, setProtectTarget] = useState<UntrustedTarget | null>(null);
  const [protectBusy, setProtectBusy] = useState(false);
  const [protectError, setProtectError] = useState<CommandError | null>(null);
  const [cumulative, setCumulative] = useState<CleanMovedTotalsV1 | null>(null);
  const mounted = useRef(true);

  useEffect(() => () => { mounted.current = false; }, []);

  const scan = async () => {
    if (state.phase === "scanning" || state.phase === "executing" || state.pending !== null) return;
    const scanId = createScanId();
    setView("stage");
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
          if (mounted.current) {
            dispatch({ type: "execute_requested" });
            setCumulative(null);
          }
          return bridge.planExecute(plan, selectedIds, digest);
        },
      });
      if (!lease) return;
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
  const stageVisible = showMain && !showStage && view === "stage";
  const detailVisible = showMain && !showStage && view === "detail";
  const busy = state.phase === "executing" || state.pending !== null;
  const errorBanner = state.error && <ErrorBanner error={state.error} onDismiss={() => dispatch({ type: "error_dismissed" })} />;
  const mainContent = showMain ? <div className="main-content">
    {reported && state.execution ? <ExecutePage locale={locale} report={state.execution} final onConfirm={() => undefined} onReturn={() => dispatch({ type: "review_requested" })} />
      : showDryRun && state.dryRun ? <ExecutePage locale={locale} report={state.dryRun.report} final={false} onConfirm={() => dispatch({ type: "confirmation_opened" })} onReturn={() => dispatch({ type: "review_requested" })} />
      : preview ? <ScanPreviewPage locale={locale} status={preview.status} progress={preview.progress} preview={preview.preview} canReturnToReport={state.scan !== null} onReturnToReport={() => dispatch({ type: "stopped_preview_dismissed" })} />
      : <ReviewPage
          locale={locale}
          state={filteredState}
          onSelect={(targetId, selected) => dispatch({ type: "selection_changed", targetId, selected })}
          onSelectAll={(selected) => dispatch({ type: "select_all_changed", selected })}
          onDryRun={() => void dryRun()}
          onSkip={(targetId) => dispatch({ type: "target_skipped", targetId })}
          onRestore={(targetId) => dispatch({ type: "target_restored", targetId })}
          onProtect={(target) => { setProtectError(null); setProtectTarget(target); }}
        />}
  </div> : null;
  const scanPage = <ScanPage locale={locale} stage={showStage} empty={emptyReport} activeScan={state.activeScan} busy={busy} options={options} onOptions={setOptions} onScan={() => void scan()} onCancel={() => void cancel()} />;
  const rescanLink = <button type="button" className="stage-link" disabled={busy || (!options.include_projects && !options.include_global)} onClick={() => void scan()}>{message(locale, "clean.v1.action.scan")}</button>;
  const detailsAction = <button type="button" className="stage-action" onClick={() => setView("detail")}>{message(locale, "stage.v1.action.details")}</button>;
  let stage: ReactNode = null;
  if (stageVisible && reported && state.execution) {
    const moved = movedTotals(state.execution);
    const counts = message(locale, "clean.v1.result.counts", { succeeded: String(state.execution.succeeded), skipped: String(state.execution.skipped), failed: String(state.execution.failed) });
    stage = <StageResult
      mode="clean"
      label={message(locale, "command.clean")}
      className="clean-stage clean-result"
      caption={message(locale, "clean.v1.result.caption")}
      value={formatBytes(moved.verified_bytes + moved.partial_lower_bound_bytes)}
      meta={<>
        <MetaLine parts={[counts, ...capacityParts(locale, moved)]} />
        <p>{message(locale, "clean.v1.trash.moved")}</p>
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
      meta={totals ? <MetaLine parts={[message(locale, "clean.v1.scan.complete", { count: String(totals.target_count) }), ...capacityParts(locale, totals)]} /> : undefined}
      primary={detailsAction}
      secondary={rescanLink}
    />;
  } else if (stageVisible && state.scan) {
    const totals = state.scan.health.totals;
    stage = <StageResult
      mode="clean"
      label={message(locale, "command.clean")}
      className="clean-stage clean-found"
      caption={message(locale, "clean.v1.stage.found_caption")}
      value={formatBytes(totals.verified_bytes + totals.partial_lower_bound_bytes)}
      meta={<>
        <MetaLine parts={[message(locale, "clean.v1.scan.complete", { count: String(state.scan.plan.targets.length) }), ...capacityParts(locale, totals)]} />
        {state.protectedIds.size > 0 && <p role="status">{message(locale, "clean.v1.review.rescan_hint")}</p>}
      </>}
      action={<button type="button" className="stage-action" onClick={() => setView("detail")}>{message(locale, "clean.v1.action.review")}</button>}
      secondary={rescanLink}
    />;
  }
  return <div className="clean-mode">
    {showStage ? <>
      {scanPage}
      {errorBanner}
      {mainContent}
    </> : stage ? <>
      {stage}
      {errorBanner}
    </> : detailVisible ? <DetailView locale={locale} onBack={() => setView("stage")}>
      {scanPage}
      {state.phase !== "scanning" && state.scan && !emptyReport ? <div className="scan-toolbar"><label>{message(locale, "clean.v1.search.label")} <input value={query} onChange={(event) => setQuery(event.target.value)} /></label></div> : null}
      {errorBanner}
      {mainContent}
    </DetailView> : null}
    <ConfirmDialog locale={locale} open={state.phase === "confirming" || state.phase === "executing"} digest={state.dryRun?.digest ?? ""} irreversible={hasIrreversibleSelection(state)} busy={state.phase === "executing"} onCancel={() => dispatch({ type: "confirmation_closed" })} onConfirm={() => void execute()} />
    <ProtectDialog locale={locale} path={protectTarget?.path ?? null} busy={protectBusy} error={protectError} onCancel={() => { setProtectTarget(null); setProtectError(null); }} onConfirm={() => void protect()} onDismissError={() => setProtectError(null)} />
  </div>;
}
