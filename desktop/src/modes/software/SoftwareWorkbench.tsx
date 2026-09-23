import { useEffect, useMemo, useReducer, useRef, useState } from "react";
import type { DesktopBridge } from "../../api/bridge";
import { decodeCommandError } from "../../api/contract";
import type {
  CommandError,
  DesktopSoftwareAuditResult,
  DesktopSoftwareInventoryResult,
  DesktopSoftwarePreviewResult,
  DesktopSoftwareUninstallResult,
  SoftwareEntryV1,
  SoftwareExecutionOutcome,
} from "../../api/types.gen";
import { AccessibleUserData } from "../../app-shell/AppShell";
import { DestinationGlyph } from "../../app-shell/glyphs";
import { formatBinaryBytes, message, type MessageKey, type PresentationLanguageTag } from "../../i18n";
import { DetailView, Stage, StageResult } from "../../stage";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { selectSoftwareEntries, selectedSoftwareEntries, softwareIdentityText, softwareSource, summarizeSoftwareSize } from "./selectors";
import { initialSoftwareState, softwareReducer, type SoftwareOperation } from "./state";
import "./styles.css";

interface SoftwareWorkbenchProps {
  readonly bridge: DesktopBridge;
  readonly coordinator: OperationCoordinator;
  readonly locale: PresentationLanguageTag;
}

let operationSequence = 0;
function createOperationId(kind: SoftwareOperation): string {
  operationSequence += 1;
  const randomId = globalThis.crypto?.randomUUID?.();
  return randomId ? `software-${kind}-${randomId}` : `software-${kind}-${Date.now()}-${operationSequence}`;
}

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected Software IPC error" }; }
}

function sourceLabel(locale: PresentationLanguageTag, entry: SoftwareEntryV1): string {
  return message(locale, `software.v1.source.${softwareSource(entry)}` as MessageKey);
}

function eligibilityLabel(locale: PresentationLanguageTag, entry: SoftwareEntryV1): string {
  return message(locale, `software.v1.eligibility.${entry.eligibility.reason}` as MessageKey);
}

function scopeLabel(locale: PresentationLanguageTag, entry: SoftwareEntryV1): string {
  return message(locale, `software.v1.scope.${entry.scope}` as MessageKey);
}

function sizeLabel(locale: PresentationLanguageTag, entry: SoftwareEntryV1): string {
  if (entry.size.state === "unknown") return message(locale, "software.v1.size.unknown");
  if (entry.size.state === "partial") return message(locale, "software.v1.size.lower_bound", { bytes: formatBinaryBytes(entry.size.lower_bound_bytes) });
  return message(locale, entry.size.basis === "reported_estimate" ? "software.v1.size.estimated" : "software.v1.size.measured", {
    bytes: formatBinaryBytes(entry.size.value_bytes),
  });
}

function outcomeLabel(locale: PresentationLanguageTag, outcome: SoftwareExecutionOutcome): string {
  return message(locale, `software.v1.outcome.${outcome}` as MessageKey);
}

export function SoftwareWorkbench({ bridge, coordinator, locale }: SoftwareWorkbenchProps) {
  const [state, dispatch] = useReducer(softwareReducer, initialSoftwareState);
  const [overview, setOverview] = useState(false);
  const mounted = useRef(true);
  useEffect(() => () => { mounted.current = false; }, []);
  const visibleEntries = useMemo(
    () => selectSoftwareEntries(state.inventory?.entries ?? [], state.query, state.sort),
    [state.inventory, state.query, state.sort],
  );
  const selectedEntries = useMemo(() => selectedSoftwareEntries(state), [state]);
  const sizeSummary = useMemo(() => summarizeSoftwareSize(selectedEntries), [selectedEntries]);
  const active = state.operationId !== null;

  const runOperation = async <T,>(
    operation: SoftwareOperation,
    start: (operationId: string) => Promise<T>,
    completed: (operationId: string, result: T) => void,
  ) => {
    const operationId = createOperationId(operation);
    let started = false;
    try {
      const lease = await coordinator.start<T>({
        kind: "software",
        id: operationId,
        cancel: async () => {
          if (mounted.current) dispatch({ type: "cancel_requested", operationId });
          await bridge.softwareCancel(operationId);
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

  const refreshInventory = () => void runOperation<DesktopSoftwareInventoryResult>(
    "inventory",
    (operationId) => bridge.softwareInventoryStart(operationId),
    (operationId, result) => {
      if (result.type === "completed") dispatch({ type: "inventory_completed", operationId, inventory: result.inventory });
      else dispatch({ type: "operation_canceled", operationId });
    },
  );

  const reviewPreview = () => {
    if (!state.inventory || state.selectedIds.size === 0) return;
    const inventory = state.inventory;
    const selectedIds = [...state.selectedIds];
    void runOperation<DesktopSoftwarePreviewResult>(
      "preview",
      (operationId) => bridge.softwarePreview(operationId, inventory, selectedIds),
      (operationId, result) => dispatch({ type: "preview_completed", operationId, result }),
    );
  };

  const uninstall = () => {
    if (!state.plan || !state.preview || state.status !== "confirming") return;
    const plan = state.plan;
    const previewDigest = state.preview.digest;
    void runOperation<DesktopSoftwareUninstallResult>(
      "uninstall",
      (operationId) => bridge.softwareUninstall(operationId, plan, previewDigest, true),
      (operationId, result) => dispatch({ type: "uninstall_completed", operationId, result }),
    );
  };

  const refreshAudit = () => void runOperation<DesktopSoftwareAuditResult>(
    "audit",
    (operationId) => bridge.softwareAudit(operationId),
    (operationId, result) => dispatch({ type: "audit_completed", operationId, result }),
  );

  const cancelActive = async () => {
    const operationId = state.operationId;
    if (!operationId) return;
    dispatch({ type: "cancel_requested", operationId });
    try { await bridge.softwareCancel(operationId); }
    catch (error) { dispatch({ type: "operation_failed", operationId, error: commandError(error) }); }
  };

  const statusText = state.status === "loading" ? message(locale, "software.v1.state.loading")
    : state.status === "previewing" ? message(locale, "software.v1.state.previewing")
      : state.status === "uninstalling" ? message(locale, "software.v1.state.uninstalling")
        : state.status === "canceling" ? message(locale, "software.v1.state.canceling")
          : null;

  const statusChip = statusText ? <span className="status-chip"><span className="status-chip-dot" aria-hidden="true" />{statusText}</span> : null;
  const liveStatus = statusText ? <p className="software-live-status" role="status" aria-live="polite">{statusText}</p> : null;
  const inventorySummary = state.inventory ? message(locale, "software.v1.inventory.summary", {
    count: String(state.inventory.entries.length),
    selectable: String(state.inventory.entries.filter((entry) => entry.eligibility.state === "selectable").length),
    manual: String(state.inventory.entries.filter((entry) => entry.eligibility.state === "manual").length),
  }) : "";
  const errorBanner = state.error ? <div className="error-banner" role="alert">
    <span>{"message" in state.error ? state.error.message : state.error.code}</span>
    <button type="button" onClick={() => dispatch({ type: "error_dismissed" })}>×</button>
  </div> : null;
  const auditCard = state.audit ? <section className="card software-audit" aria-live="polite">
    <p>{message(locale, "software.v1.audit.summary", { records: String(state.audit.records.length), recovered: String(state.audit.recovered.length) })}</p>
    <details><summary>{message(locale, "software.v1.audit.transitions")}</summary>
      <ol>{state.audit.records.map((record, index) => <li key={`${record.operation_id}-${index}`}><AccessibleUserData value={`${record.operation_id} · ${record.status_code}`} /></li>)}</ol>
    </details>
  </section> : null;

  return <div className="software-mode" data-status={state.status}>
    {!state.inventory || overview ? <>
      {!state.inventory ? <Stage
        mode="software"
        label={message(locale, "command.software")}
        busy={active}
        title={message(locale, "software.v1.state.empty.title")}
        meta={<><p>{message(locale, "software.v1.state.empty.detail")}</p>{liveStatus}</>}
        primary={active
          ? <button type="button" className="stage-action" disabled={state.status === "canceling"} onClick={() => void cancelActive()}>{message(locale, "software.v1.action.cancel")}</button>
          : <button type="button" className="stage-action" onClick={refreshInventory}>{message(locale, "software.v1.action.inventory")}</button>}
        secondary={<button type="button" className="stage-link" disabled={active} onClick={refreshAudit}>{message(locale, "software.v1.action.audit")}</button>}
      /> : <StageResult
        mode="software"
        label={message(locale, "command.software")}
        caption={message(locale, "stage.v1.caption.software")}
        value={String(state.inventory.entries.length)}
        meta={<><p>{inventorySummary}</p>{liveStatus}</>}
        action={<button type="button" className="stage-action" onClick={() => setOverview(false)}>{message(locale, "stage.v1.action.details")}</button>}
      />}
      {errorBanner}
      {!state.inventory ? auditCard : null}
    </> : <DetailView locale={locale} onBack={() => setOverview(true)} status={statusChip}>
    <section className="software-toolbar" aria-label={message(locale, "command.software")}>
      {state.inventory ? <button type="button" className="primary-button" disabled={active} onClick={refreshInventory}>
        {message(locale, "software.v1.action.inventory")}
      </button> : null}
      <label>
        <span>{message(locale, "software.v1.search.label")}</span>
        <input type="search" value={state.query} onChange={(event) => dispatch({ type: "query_changed", query: event.target.value })} />
      </label>
      <label>
        <span>{message(locale, "software.v1.sort.label")}</span>
        <select value={state.sort} onChange={(event) => dispatch({ type: "sort_changed", sort: event.target.value as typeof state.sort })}>
          {(["name", "size", "eligibility", "source"] as const).map((sort) => <option key={sort} value={sort}>{message(locale, `software.v1.sort.${sort}` as MessageKey)}</option>)}
        </select>
      </label>
      <button type="button" className="secondary-button" disabled={active} onClick={refreshAudit}>
        {message(locale, "software.v1.action.audit")}
      </button>
      {active ? <button type="button" className="danger-button" disabled={state.status === "canceling"} onClick={() => void cancelActive()}>
        {message(locale, "software.v1.action.cancel")}
      </button> : null}
      {liveStatus}
    </section>

    {errorBanner}

    <section className="card software-inventory" aria-busy={active}>
      <header className="software-inventory-header">
        <p>{inventorySummary}</p>
        <button type="button" className="secondary-button" disabled={active} onClick={() => dispatch({ type: "select_all", selected: state.selectedIds.size === 0 })}>
          {state.selectedIds.size === 0 ? message(locale, "software.v1.action.select_all") : message(locale, "software.v1.action.clear_selection")}
        </button>
      </header>
      <ul className="software-list" aria-label={message(locale, "command.software")}>
        {visibleEntries.map((entry) => {
          const selectable = entry.eligibility.state === "selectable";
          const name = entry.display_name ?? entry.id;
          const source = sourceLabel(locale, entry);
          return <li key={entry.id} className="tile-row software-row" data-selectable={selectable}>
            <label className="software-selection">
              <input
                type="checkbox"
                checked={state.selectedIds.has(entry.id)}
                disabled={!selectable || active}
                aria-label={`${name}: ${eligibilityLabel(locale, entry)}`}
                onChange={(event) => dispatch({ type: "selection_changed", softwareId: entry.id, selected: event.target.checked })}
              />
            </label>
            <span className="glyph-tile"><DestinationGlyph name="software" /></span>
            <div className="tile-row-text">
              <strong><AccessibleUserData value={name} /></strong>
              <span className="secondary">
                {entry.publisher ? <AccessibleUserData value={entry.publisher} /> : source}
                {entry.publisher ? ` · ${source}` : ""}
                {` · ${scopeLabel(locale, entry)} · ${sizeLabel(locale, entry)} · ${message(locale, "software.v1.last_used.unknown")}`}
              </span>
            </div>
            <div className="tile-row-actions software-row-facts">
              <span className={selectable ? "eligible" : "manual"}>{eligibilityLabel(locale, entry)}</span>
            </div>
            <details open={state.expandedId === entry.id} onToggle={(event) => dispatch({ type: "expanded_changed", softwareId: event.currentTarget.open ? entry.id : null })}>
              <summary>{message(locale, "software.v1.detail.identity")}</summary>
              <dl>
                <dt>ID</dt><dd><AccessibleUserData value={entry.id} /></dd>
                <dt>{message(locale, "software.v1.detail.identity")}</dt><dd><AccessibleUserData value={softwareIdentityText(entry.identity)} /></dd>
                <dt>Publisher</dt><dd><AccessibleUserData value={entry.publisher ?? "—"} /></dd>
                <dt>Version</dt><dd><AccessibleUserData value={entry.version ?? "—"} /></dd>
                <dt>{message(locale, "software.v1.detail.last_used")}</dt><dd>{message(locale, "software.v1.last_used.unknown")}</dd>
              </dl>
            </details>
          </li>;
        })}
      </ul>
    </section>

    {state.preview ? <section className="card software-preview" aria-labelledby="software-preview-title">
      <h2 id="software-preview-title">{message(locale, "software.v1.action.preview")}</h2>
      <p className="software-irrevocable">{message(locale, "software.v1.confirm.detail")}</p>
      <p><code>{state.preview.digest}</code></p>
      <ul>{state.preview.selected.map((item) => <li key={item.id}><AccessibleUserData value={softwareIdentityText(item.identity)} /></li>)}</ul>
    </section> : null}

    {state.report ? <section className="card software-results" aria-live="polite">
      <h2>{state.status === "unknown" ? message(locale, "software.v1.outcome.unknown_after_dispatch") : message(locale, "software.v1.results.title")}</h2>
      <ul>{state.report.outcomes.map((item) => <li key={item.operation_id} data-outcome={item.outcome}>
        <AccessibleUserData value={item.software_id} /> — {outcomeLabel(locale, item.outcome)}
      </li>)}</ul>
    </section> : null}

    {auditCard}

    <footer className="software-summary-bar">
      <p>{message(locale, "software.v1.summary", {
        count: String(state.selectedIds.size),
        estimated: formatBinaryBytes(sizeSummary.estimatedBytes),
        measured: formatBinaryBytes(sizeSummary.measuredBytes),
        lower: formatBinaryBytes(sizeSummary.lowerBoundBytes),
        unknown: String(sizeSummary.unknownCount),
      })}</p>
      <div className="software-summary-actions">
        <button type="button" className="primary-button" disabled={active || state.selectedIds.size === 0} onClick={reviewPreview}>
          {message(locale, "software.v1.action.preview")}
        </button>
        <button type="button" className="danger-button" disabled={active || state.status !== "preview_ready"} onClick={() => dispatch({ type: "confirmation_opened" })}>
          {message(locale, "software.v1.action.confirm")}
        </button>
      </div>
    </footer>
    </DetailView>}

    <dialog className="software-confirm-dialog" open={state.status === "confirming"} aria-labelledby="software-confirm-title">
      <h2 id="software-confirm-title">{message(locale, "software.v1.confirm.title")}</h2>
      <p>{message(locale, "software.v1.confirm.detail")}</p>
      {state.preview ? <>
        <p><strong>{message(locale, "software.v1.confirm.digest", { digest: state.preview.digest })}</strong></p>
        <ul>{selectedEntries.map((entry) => <li key={entry.id}><AccessibleUserData value={`${entry.display_name ?? entry.id} — ${softwareIdentityText(entry.identity)} — ${scopeLabel(locale, entry)}`} /></li>)}</ul>
      </> : null}
      <div className="software-confirm-actions">
        <button type="button" className="secondary-button" onClick={() => dispatch({ type: "confirmation_closed" })}>{message(locale, "software.v1.action.close")}</button>
        <button type="button" className="danger-button" onClick={uninstall}>{message(locale, "software.v1.action.uninstall")}</button>
      </div>
    </dialog>
  </div>;
}
