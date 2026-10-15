import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { CommandError, DesktopScanResult, DryRunOutcome, ExecutionReport, UntrustedTarget } from "../../api/types.gen";
import type { DesktopBridge } from "../../api/bridge";
import { decodeCommandError } from "../../api/contract";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { ErrorBanner } from "../../components/ErrorBanner";
import { ExecutePage } from "../../pages/ExecutePage";
import { ReviewPage } from "../../pages/ReviewPage";
import { ScanPreviewPage } from "../../pages/ScanPreviewPage";
import { message, type PresentationLanguageTag } from "../../i18n";
import { hasIrreversibleSelection } from "../../state/selectors";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { ProtectDialog } from "./ProtectDialog";
import type { CleanAction } from "./reducer";
import { CleanSessionProvider, useCleanSession } from "./CleanSession";
import { CleanOverview } from "./CleanOverview";
import { useCleanAdvisory } from "./useCleanAdvisory";
import type { CleanScope } from "./session-state";
import "./styles.css";

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected desktop error" }; }
}
let sequence = 0;
const operationId = (kind: string) => `${kind}-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${++sequence}`}`;
interface Props { bridge: DesktopBridge; coordinator: OperationCoordinator; locale: PresentationLanguageTag; onAnalyzeRoot?: (root: string) => void }
type Session = NonNullable<ReturnType<typeof useCleanSession>>;

/** The shell retains this provider; standalone consumers get the same domain controller. */
export function CleanWorkbench(props: Props) {
  const session = useCleanSession();
  return session ? <Workbench {...props} session={session} /> : <CleanSessionProvider bridge={props.bridge}><CleanWorkbench {...props} /></CleanSessionProvider>;
}

function Workbench({ bridge, coordinator, locale, onAnalyzeRoot, session }: Props & { session: Session }) {
  const { slots, dispatch, scope, advisory } = session;
  const [detail, setDetail] = useState<CleanScope | null>(null);
  const [query, setQuery] = useState("");
  const [protectTarget, setProtectTarget] = useState<UntrustedTarget | null>(null);
  const [protectBusy, setProtectBusy] = useState(false);
  const [protectError, setProtectError] = useState<CommandError | null>(null);
  const mounted = useRef(true);
  const current = useRef(slots);
  useLayoutEffect(() => { current.current = slots; }, [slots]);
  // Locks immediately, before React renders disabled controls or the coordinator joins.
  const request = useRef<{ id: string; scope: CleanScope; kind: "scan" | "preview" | "execute"; started: boolean; canceled: boolean } | null>(null);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      const owned = request.current;
      if (owned) {
        owned.canceled = true;
        if (coordinator.activeIdentity()?.id === owned.id) void coordinator.cancelAndJoin().catch(() => undefined);
      }
      dispatch({ type: "hidden" });
    };
  }, [coordinator, dispatch]);
  const update = (name: CleanScope, revision: number, change: CleanAction) => dispatch({ type: "update", scope: name, revision, change, completedAt: change.type === "scan_completed" ? new Date().toISOString() : undefined });
  const busy = protectBusy || Object.values(slots).some(({ state }) => state.activeScan !== null || state.pending !== null || state.phase === "executing");
  const assessment = useCleanAdvisory(bridge, slots.project.state.scan, advisory.threshold);

  const scan = async (name: CleanScope, targetId?: string) => {
    if (request.current || busy || (name === "project" && (scope.loading || scope.saving || scope.options.roots.length === 0))) return;
    const options = name === "global" ? { include_projects: false, include_global: true, roots: [] }
      : { include_projects: true, include_global: false, roots: [...scope.options.roots] };
    const scanId = operationId("scan");
    const owned = { id: scanId, scope: name, kind: "scan" as const, started: false, canceled: false };
    request.current = owned;
    const revision = slots[name].revision + 1;
    update(name, revision - 1, { type: "scan_requested", scanId });
    setDetail(null);
    let progressError: CommandError | null = null;
    try {
      const lease = await coordinator.start<DesktopScanResult>({ kind: "clean.scan", id: scanId,
        cancel: () => { owned.canceled = true; return bridge.scanCancel(scanId); },
        start: () => {
          if (owned.canceled || !mounted.current) return Promise.resolve({ type: "canceled", scan_id: scanId });
          owned.started = true;
          const onProgress: Parameters<DesktopBridge["scanStart"]>[2] = progress => { if (mounted.current) update(name, revision, { type: "scan_progressed", progress }); };
          const onError = (error: unknown) => {
            if (progressError) return;
            progressError = commandError(error);
            if (mounted.current) {
              update(name, revision, { type: "scan_cancel_requested", scanId });
              update(name, revision, { type: "command_failed", error: progressError });
            }
            void bridge.scanCancel(scanId).catch(() => undefined);
          };
          return targetId ? bridge.scanRescanTarget(scanId, options, targetId, onProgress, onError) : bridge.scanStart(scanId, options, onProgress, onError);
        } });
      if (!lease) throw new Error("The desktop operation coordinator is closed.");
      try {
        const result = await lease.result;
        if (mounted.current) {
          if (progressError) update(name, revision, { type: "scan_failed", scanId, error: progressError });
          else if (result.type === "completed") update(name, revision, { type: "scan_completed", scanId, report: result.report });
          else update(name, revision, { type: "scan_canceled", scanId });
        }
      } finally { await lease.complete().catch(() => false); }
    } catch (error) { if (mounted.current) update(name, revision, { type: "scan_failed", scanId, error: progressError ?? commandError(error) }); }
    finally { if (request.current === owned) request.current = null; }
  };

  const cancel = async (name: CleanScope) => {
    const scanId = slots[name].state.activeScan?.scanId;
    if (!scanId) return;
    update(name, slots[name].revision, { type: "scan_cancel_requested", scanId });
    const owned = request.current;
    if (owned?.id === scanId) { owned.canceled = true; if (!owned.started) return; }
    try { await bridge.scanCancel(scanId); }
    catch (error) { update(name, slots[name].revision, { type: "command_failed", error: commandError(error) }); }
  };

  const dryRun = async (name: CleanScope) => {
    const { state, revision, stale } = slots[name];
    if (request.current || busy || stale || !state.scan || state.selectedIds.size === 0) return;
    const plan = state.scan.plan, selectedIds = [...state.selectedIds];
    const owned = { id: operationId("preview"), scope: name, kind: "preview" as const, started: false, canceled: false };
    request.current = owned;
    update(name, revision, { type: "dry_run_requested" });
    try {
      const lease = await coordinator.start<DryRunOutcome>({ kind: "clean.dry-run", id: owned.id,
        cancel: () => { owned.canceled = true; },
        start: () => {
          if (!mounted.current || owned.canceled || current.current[name].revision !== revision) throw new Error("Preview superseded.");
          owned.started = true;
          return bridge.planDryRun(plan, selectedIds);
        } });
      if (!lease) throw new Error("The desktop operation coordinator is closed.");
      try {
        const outcome = await lease.result;
        if (mounted.current && !owned.canceled) { update(name, revision, { type: "dry_run_succeeded", outcome }); setDetail(name); }
      } finally { await lease.complete().catch(() => false); }
    } catch (error) { if (mounted.current) update(name, revision, { type: "command_failed", error: commandError(error) }); }
    finally { if (request.current === owned) request.current = null; }
  };

  const execute = async (name: CleanScope) => {
    const { state, revision, stale } = slots[name];
    if (request.current || busy || stale || state.phase !== "confirming" || !state.scan || !state.dryRun) return;
    const plan = state.scan.plan, selectedIds = [...state.selectedIds], digest = state.dryRun.digest;
    const owned = { id: operationId("execute"), scope: name, kind: "execute" as const, started: false, canceled: false };
    request.current = owned;
    update(name, revision, { type: "execute_requested" });
    try {
      const lease = await coordinator.start<ExecutionReport>({ kind: "clean.execute", id: owned.id,
        cancel: () => { owned.canceled = true; },
        start: () => {
          if (!mounted.current || owned.canceled || current.current[name].revision !== revision) throw new Error("Execution superseded.");
          owned.started = true;
          return bridge.planExecute(plan, selectedIds, digest);
        } });
      if (!lease) throw new Error("The desktop operation coordinator is closed.");
      try { const report = await lease.result; if (mounted.current) { update(name, revision, { type: "execute_succeeded", report }); setDetail(name); } }
      finally { await lease.complete().catch(() => false); }
    } catch (error) { if (mounted.current) update(name, revision, { type: "command_failed", error: commandError(error) }); }
    finally { if (request.current === owned) request.current = null; }
  };

  const protect = async () => {
    if (!protectTarget?.path || protectBusy || request.current) return;
    setProtectBusy(true); setProtectError(null);
    try {
      await bridge.protectionAdd(protectTarget.path, true);
      for (const name of ["global", "project"] as const) update(name, current.current[name].revision, { type: "target_protected", targetId: protectTarget.id });
      dispatch({ type: "protections_changed" });
      if (mounted.current) setProtectTarget(null);
    }
    catch (error) { if (mounted.current) setProtectError(commandError(error)); }
    finally { if (mounted.current) setProtectBusy(false); }
  };

  const selectedScope = detail ?? "project";
  const slot = slots[selectedScope], state = slot.state;
  const change = (action: CleanAction) => update(selectedScope, slot.revision, action);
  const preview = state.activeScan ? { status: "active" as const, ...state.activeScan }
    : state.stoppedPreview ? { status: state.stoppedPreview.kind, ...state.stoppedPreview } : null;
  return <div className="clean-mode clean-workbench">
    {scope.error !== null && <ErrorBanner error={commandError(scope.error)} onDismiss={scope.dismissError} />}
    {detail === null ? <CleanOverview session={session} locale={locale} busy={busy} scan={name => void scan(name)} cancel={name => void cancel(name)} review={name => { setQuery(""); setDetail(name); }} preview={name => void dryRun(name)} dismiss={name => update(name, slots[name].revision, { type: "error_dismissed" })} assessment={assessment} /> : <>
      <header className="clean-detail-heading"><button type="button" className="secondary-button" onClick={() => setDetail(null)}>{message(locale, "clean.v2.back")}</button><h1>{message(locale, `clean.v2.${detail}`)}</h1></header>
      {state.error && <ErrorBanner error={state.error} onDismiss={() => change({ type: "error_dismissed" })} />}
      {slot.stale && <p className="warning-text" role="status">{message(locale, "clean.v2.stale")}</p>}
      {state.execution && state.phase === "reported" ? <ExecutePage locale={locale} report={state.execution} targets={state.scan?.plan.targets} final onConfirm={() => undefined} onReturn={() => change({ type: "review_requested" })} />
        : state.dryRun && ["dry_run", "confirming", "executing"].includes(state.phase) ? <ExecutePage locale={locale} report={state.dryRun.report} targets={state.scan?.plan.targets} final={false} onConfirm={() => change({ type: "confirmation_opened" })} onReturn={() => change({ type: "review_requested" })} />
          : preview ? <><ScanPreviewPage locale={locale} status={preview.status} progress={preview.progress} preview={preview.preview} canReturnToReport={state.scan !== null} onReturnToReport={() => change({ type: "stopped_preview_dismissed" })} />{state.activeScan && <button type="button" disabled={state.activeScan.cancelRequested} onClick={() => void cancel(selectedScope)}>{message(locale, "clean.v1.action.cancel")}</button>}</>
            : <><label className="clean-search">{message(locale, "clean.v1.search.label")}<input type="search" value={query} onChange={event => setQuery(event.target.value)} /></label><ReviewPage locale={locale} state={state} query={query} disabled={busy || slot.stale}
              onSelect={(targetId, selected) => { if (!busy && !slot.stale) change({ type: "selection_changed", targetId, selected }); }}
              onSelectAll={selected => { if (!busy && !slot.stale) change({ type: "select_all_changed", selected }); }}
              onDryRun={() => void dryRun(selectedScope)}
              onSkip={targetId => change({ type: "target_skipped", targetId })} onRestore={targetId => change({ type: "target_restored", targetId })}
              onProtect={target => { setProtectError(null); setProtectTarget(target); }}
              onRescan={target => void scan(selectedScope, target.id)}
              onAnalyze={onAnalyzeRoot ? target => { if (target.path) onAnalyzeRoot(target.path); } : undefined} /></>}
    </>}
    <ConfirmDialog locale={locale} open={state.phase === "confirming" || state.phase === "executing"} digest={state.dryRun?.digest ?? ""} irreversible={hasIrreversibleSelection(state)} busy={state.phase === "executing"} onCancel={() => change({ type: "confirmation_closed" })} onConfirm={() => void execute(selectedScope)} />
    <ProtectDialog locale={locale} path={protectTarget?.path ?? null} busy={protectBusy} error={protectError} onCancel={() => { setProtectTarget(null); setProtectError(null); }} onConfirm={() => void protect()} onDismissError={() => setProtectError(null)} />
  </div>;
}
