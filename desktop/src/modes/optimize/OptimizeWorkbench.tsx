import { useEffect, useReducer, useRef } from "react";
import type { DesktopBridge } from "../../api/bridge";
import { decodeCommandError } from "../../api/contract";
import type {
  CommandError,
  DesktopOptimizeAuditResult,
  DesktopOptimizeListResult,
  DesktopOptimizePreviewResult,
  DesktopOptimizeRunResult,
  MaintenanceActionClass,
  MaintenanceCatalogueEntryV1,
  MaintenanceExecutionOutcome,
} from "../../api/types.gen";
import { AccessibleUserData, PageHeaderSlot } from "../../app-shell/AppShell";
import { DestinationGlyph, type GlyphName } from "../../app-shell/glyphs";
import { ErrorBanner } from "../../components/ErrorBanner";
import { message, type MessageKey, type PresentationLanguageTag } from "../../i18n";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { dispatchable, initialOptimizeState, optimizeReducer, selectedEntry, type OptimizeOperation } from "./state";
import "./styles.css";

interface OptimizeWorkbenchProps {
  readonly bridge: DesktopBridge;
  readonly coordinator: OperationCoordinator;
  readonly locale: PresentationLanguageTag;
}

let operationSequence = 0;
function createOperationId(kind: OptimizeOperation): string {
  operationSequence += 1;
  const randomId = globalThis.crypto?.randomUUID?.();
  return randomId ? `optimize-${kind}-${randomId}` : `optimize-${kind}-${Date.now()}-${operationSequence}`;
}

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected Optimize IPC error" }; }
}

function titleKey(id: string): MessageKey {
  switch (id) {
    case "dns.flush": return "optimize.v1.title.dns.flush";
    case "settings.storage_recommendations": return "optimize.v1.title.settings.storage_recommendations";
    case "settings.search": return "optimize.v1.title.settings.search";
    case "settings.energy_recommendations": return "optimize.v1.title.settings.energy_recommendations";
    case "guidance.drive_optimize": return "optimize.v1.title.guidance.drive_optimize";
    case "guidance.system_integrity": return "optimize.v1.title.guidance.system_integrity";
    case "guidance.filesystem_check": return "optimize.v1.title.guidance.filesystem_check";
    case "guidance.network_reset": return "optimize.v1.title.guidance.network_reset";
    default: return "optimize.v1.detail.identity";
  }
}

function badgeKey(actionClass: MaintenanceActionClass): MessageKey {
  switch (actionClass) {
    case "execute": return "optimize.v1.action.execute";
    case "settings_handoff": return "optimize.v1.action.settings";
    case "guidance": return "optimize.v1.action.guidance";
  }
}

function effectKey(actionClass: MaintenanceActionClass): MessageKey {
  switch (actionClass) {
    case "execute": return "optimize.v1.effect.dns.flush";
    case "settings_handoff": return "optimize.v1.effect.settings";
    case "guidance": return "optimize.v1.effect.guidance";
  }
}

function riskKey(actionClass: MaintenanceActionClass): MessageKey {
  switch (actionClass) {
    case "execute": return "optimize.v1.risk.dns.flush";
    case "settings_handoff": return "optimize.v1.risk.settings";
    case "guidance": return "optimize.v1.risk.guidance";
  }
}

function outcomeLabel(locale: PresentationLanguageTag, outcome: MaintenanceExecutionOutcome): string {
  return message(locale, `optimize.v1.outcome.${outcome}` as MessageKey);
}

function actionGlyph(actionClass: MaintenanceActionClass): GlyphName {
  switch (actionClass) {
    case "execute": return "optimize";
    case "settings_handoff": return "language";
    case "guidance": return "help";
  }
}

function summaryText(
  locale: PresentationLanguageTag,
  selected: MaintenanceCatalogueEntryV1 | null,
  previewDigest: string | null,
  reportOutcome: MaintenanceExecutionOutcome | null,
): string {
  if (reportOutcome === "launched") return message(locale, "optimize.v1.summary.launched");
  if (reportOutcome === "unknown_after_dispatch") return message(locale, "optimize.v1.summary.unknown");
  if (reportOutcome === "canceled_before_start") return message(locale, "optimize.v1.summary.canceled");
  if (reportOutcome === "failed") return message(locale, "optimize.v1.summary.failed");
  if (reportOutcome === "succeeded") return message(locale, "optimize.v1.summary.succeeded");
  if (selected?.action_class === "guidance") return message(locale, "optimize.v1.summary.guidance");
  if (selected && previewDigest) {
    return message(locale, "optimize.v1.summary.preview", {
      id: selected.id,
      action: message(locale, badgeKey(selected.action_class)),
      digest: previewDigest,
    });
  }
  if (selected) {
    return message(locale, "optimize.v1.summary.selected", {
      id: selected.id,
      action: message(locale, badgeKey(selected.action_class)),
    });
  }
  return message(locale, "optimize.v1.summary.catalogue");
}

function openConfirmDialog(node: HTMLDialogElement): void {
  if (node.open) return;
  if (typeof node.showModal === "function") {
    try {
      node.showModal();
      return;
    } catch {
      // jsdom and already-open native dialogs fall through to the open attribute.
    }
  }
  node.setAttribute("open", "");
}

function closeConfirmDialog(node: HTMLDialogElement): void {
  if (!node.open) return;
  if (typeof node.close === "function") {
    try {
      node.close();
      return;
    } catch {
      // jsdom close may throw when the dialog was opened via the attribute.
    }
  }
  node.removeAttribute("open");
}

export function OptimizeWorkbench({ bridge, coordinator, locale }: OptimizeWorkbenchProps) {
  const [state, dispatch] = useReducer(optimizeReducer, initialOptimizeState);
  const mounted = useRef(true);
  const confirmDialog = useRef<HTMLDialogElement>(null);
  useEffect(() => () => { mounted.current = false; }, []);
  useEffect(() => {
    const node = confirmDialog.current;
    if (!node) return;
    if (state.status === "confirming") openConfirmDialog(node);
    else closeConfirmDialog(node);
  }, [state.status]);
  const selected = selectedEntry(state);
  const active = state.operationId !== null;
  const canPreview = Boolean(selected && dispatchable(selected.action_class) && !active);
  const settingsHandoff = selected?.action_class === "settings_handoff";

  const runOperation = async <T,>(
    operation: OptimizeOperation,
    start: (operationId: string) => Promise<T>,
    completed: (operationId: string, result: T) => void,
  ) => {
    const operationId = createOperationId(operation);
    let started = false;
    try {
      const lease = await coordinator.start<T>({
        kind: "optimize",
        id: operationId,
        cancel: async () => {
          if (mounted.current) dispatch({ type: "cancel_requested", operationId });
          await bridge.optimizeCancel(operationId);
        },
        start: () => {
          started = true;
          if (mounted.current) dispatch({ type: "operation_requested", operationId, operation });
          return start(operationId);
        },
      });
      if (!lease) return;
      try {
        const result = await lease.result;
        if (mounted.current) completed(operationId, result);
      } finally {
        await lease.complete().catch(() => false);
      }
    } catch (error) {
      if (started && mounted.current) dispatch({ type: "operation_failed", operationId, error: commandError(error) });
    }
  };

  const refreshCatalogue = () => void runOperation<DesktopOptimizeListResult>(
    "list",
    (operationId) => bridge.optimizeListStart(operationId),
    (operationId, result) => {
      if (result.type === "completed") dispatch({ type: "list_completed", operationId, entries: result.entries });
      else dispatch({ type: "operation_canceled", operationId });
    },
  );

  const reviewPreview = () => {
    if (!selected || !dispatchable(selected.action_class)) return;
    const catalogueId = selected.id;
    void runOperation<DesktopOptimizePreviewResult>(
      "preview",
      (operationId) => bridge.optimizePreview(operationId, catalogueId),
      (operationId, result) => dispatch({ type: "preview_completed", operationId, result }),
    );
  };

  const dispatchConfirmed = () => {
    if (!state.plan || !state.preview || state.status !== "confirming") return;
    const plan = state.plan;
    const previewDigest = state.preview.digest;
    void runOperation<DesktopOptimizeRunResult>(
      "run",
      (operationId) => bridge.optimizeRun(operationId, plan, previewDigest, true),
      (operationId, result) => dispatch({ type: "run_completed", operationId, result }),
    );
  };

  const refreshAudit = () => void runOperation<DesktopOptimizeAuditResult>(
    "audit",
    (operationId) => bridge.optimizeAudit(operationId),
    (operationId, result) => dispatch({ type: "audit_completed", operationId, result }),
  );

  const cancelActive = async () => {
    const operationId = state.operationId;
    if (!operationId) return;
    dispatch({ type: "cancel_requested", operationId });
    try { await bridge.optimizeCancel(operationId); }
    catch (error) { dispatch({ type: "operation_failed", operationId, error: commandError(error) }); }
  };

  const statusText = state.status === "checking" ? message(locale, "optimize.v1.state.checking")
    : state.status === "previewing" ? message(locale, "optimize.v1.state.previewing")
      : state.status === "running" ? message(locale, "optimize.v1.state.running")
        : state.status === "launching" ? message(locale, "optimize.v1.state.launching")
          : state.status === "canceling" ? message(locale, "optimize.v1.state.canceling")
            : null;
  const reportOutcome = state.report?.outcomes[0]?.outcome ?? null;

  return <div className="optimize-mode" data-status={state.status}>
    <PageHeaderSlot>
      {statusText ? <span className="status-chip"><span className="status-chip-dot" aria-hidden="true" />{statusText}</span> : null}
    </PageHeaderSlot>
    <section className="optimize-toolbar" aria-label={message(locale, "command.optimize")}>
      {state.entries ? <button type="button" className="primary-button" disabled={active} onClick={refreshCatalogue}>
        {message(locale, "optimize.v1.action.refresh")}
      </button> : null}
      <button type="button" className="secondary-button" disabled={active} onClick={refreshAudit}>
        {message(locale, "optimize.v1.action.audit")}
      </button>
      {active ? <button type="button" className="danger-button" disabled={state.status === "canceling"} onClick={() => void cancelActive()}>
        {message(locale, "optimize.v1.action.cancel")}
      </button> : null}
      {statusText ? <p className="optimize-live-status" role="status" aria-live="polite">{statusText}</p> : null}
    </section>

    {state.error ? <ErrorBanner error={state.error} onDismiss={() => dispatch({ type: "error_dismissed" })} /> : null}

    {!state.entries ? <section className="card mode-empty">
      <span className="glyph-tile"><DestinationGlyph name="optimize" /></span>
      <h2>{message(locale, "optimize.v1.state.empty.title")}</h2>
      <p>{message(locale, "optimize.v1.state.empty.detail")}</p>
      <button type="button" className="primary-button" disabled={active} onClick={refreshCatalogue}>
        {message(locale, "optimize.v1.action.refresh")}
      </button>
    </section> : <section className="card optimize-catalogue" aria-busy={active}>
      <header className="optimize-catalogue-header">
        <p>{message(locale, "optimize.v1.list.summary", {
          count: String(state.entries.length),
          version: "1",
        })}</p>
      </header>
      <ul className="optimize-list" aria-label={message(locale, "command.optimize")}>
        {state.entries.map((entry) => {
          const title = message(locale, titleKey(entry.id));
          const badge = message(locale, badgeKey(entry.action_class));
          return <li key={entry.id} className="tile-row optimize-row" data-action-class={entry.action_class} data-selected={state.selectedId === entry.id}>
            <label className="optimize-selection">
              <input
                type="radio"
                name="optimize-selection"
                checked={state.selectedId === entry.id}
                disabled={active}
                aria-label={`${entry.id}: ${badge}`}
                onChange={() => dispatch({ type: "selection_changed", catalogueId: entry.id })}
              />
            </label>
            <span className="glyph-tile"><DestinationGlyph name={actionGlyph(entry.action_class)} /></span>
            <div className="tile-row-text">
              <strong><AccessibleUserData value={entry.id} /></strong>
              <span className="secondary optimize-row-facts">
                {title}
                {entry.build_floor !== null ? ` · ${message(locale, "optimize.v1.predicate.settings", { floor: String(entry.build_floor) })}` : ""}
                {entry.action_class === "guidance" ? <> · <span className="optimize-no-run">{message(locale, "optimize.v1.guidance.no_run")}</span></> : null}
              </span>
            </div>
            <div className="tile-row-actions">
              <span className={`optimize-badge optimize-badge-${entry.action_class}`}>{badge}</span>
            </div>
            <details open={state.expandedId === entry.id} onToggle={(event) => dispatch({ type: "expanded_changed", catalogueId: event.currentTarget.open ? entry.id : null })}>
              <summary>{message(locale, "optimize.v1.detail.capability")}</summary>
              <dl>
                <dt>{message(locale, "optimize.v1.detail.identity")}</dt><dd><AccessibleUserData value={entry.id} /></dd>
                <dt>{message(locale, badgeKey(entry.action_class))}</dt><dd>{message(locale, effectKey(entry.action_class))}</dd>
                <dt>{message(locale, "optimize.v1.detail.capability")}</dt><dd>{message(locale, riskKey(entry.action_class))}</dd>
              </dl>
            </details>
          </li>;
        })}
      </ul>
    </section>}

    {state.preview ? <section className="card optimize-preview" aria-labelledby="optimize-preview-title">
      <h2 id="optimize-preview-title">{message(locale, "optimize.v1.action.preview")}</h2>
      <p className="optimize-handoff-note">{message(locale, "optimize.v1.preview.note")}</p>
      <p><code>{state.preview.digest}</code></p>
    </section> : null}

    {state.report ? <section className="card optimize-results" aria-live="polite">
      <h2>{state.status === "unknown" ? message(locale, "optimize.v1.outcome.unknown_after_dispatch") : message(locale, "optimize.v1.results.title")}</h2>
      <ul>{state.report.outcomes.map((item) => <li key={item.operation_id} data-outcome={item.outcome}>
        <AccessibleUserData value={item.catalogue_id} /> — {outcomeLabel(locale, item.outcome)}
      </li>)}</ul>
    </section> : null}

    {state.audit ? <section className="card optimize-audit" aria-live="polite">
      <p>{message(locale, "optimize.v1.audit.summary", { records: String(state.audit.records.length), recovered: String(state.audit.recovered.length) })}</p>
      <details><summary>{message(locale, "optimize.v1.audit.transitions")}</summary>
        <ol>{state.audit.records.map((record, index) => <li key={`${record.operation_id}-${index}`}><AccessibleUserData value={`${record.operation_id} · ${record.status_code}`} /></li>)}</ol>
      </details>
    </section> : null}

    <footer className="optimize-summary-bar">
      <p>{summaryText(locale, selected, state.preview?.digest ?? null, reportOutcome)}</p>
      <div className="optimize-summary-actions">
        {selected?.action_class === "guidance" ? <span className="optimize-no-run">{message(locale, "optimize.v1.guidance.no_run")}</span> : <>
          <button type="button" className="primary-button" disabled={!canPreview} onClick={reviewPreview}>
            {message(locale, "optimize.v1.action.preview")}
          </button>
          <button type="button" className="danger-button" disabled={active || state.status !== "preview_ready"} onClick={() => dispatch({ type: "confirmation_opened" })}>
            {message(locale, "optimize.v1.action.confirm")}
          </button>
        </>}
      </div>
    </footer>

    <dialog
      ref={confirmDialog}
      className="optimize-confirm-dialog"
      aria-labelledby="optimize-confirm-title"
      onCancel={(event) => {
        event.preventDefault();
        dispatch({ type: "confirmation_closed" });
      }}
    >
      <h2 id="optimize-confirm-title">{message(locale, settingsHandoff ? "optimize.v1.confirm.title.settings" : "optimize.v1.confirm.title.execute")}</h2>
      <p>{message(locale, settingsHandoff ? "optimize.v1.confirm.detail.settings" : "optimize.v1.confirm.detail.execute")}</p>
      {state.preview ? <p><strong>{message(locale, "optimize.v1.confirm.digest", { digest: state.preview.digest })}</strong></p> : null}
      <p>{message(locale, "optimize.v1.preview.note")}</p>
      <div className="optimize-confirm-actions">
        <button type="button" className="secondary-button" onClick={() => dispatch({ type: "confirmation_closed" })}>{message(locale, "optimize.v1.action.close")}</button>
        <button type="button" className="danger-button" onClick={dispatchConfirmed}>
          {message(locale, settingsHandoff ? "optimize.v1.action.open" : "optimize.v1.action.run")}
        </button>
      </div>
    </dialog>
  </div>;
}
