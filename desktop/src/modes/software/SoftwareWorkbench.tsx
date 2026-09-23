import { useEffect, useMemo, useReducer, useRef, useState } from "react";
import type { DesktopBridge } from "../../api/bridge";
import { decodeCommandError } from "../../api/contract";
import type {
  CommandError,
  DesktopSoftwareAuditResult,
  DesktopSoftwareInventoryResult,
  DesktopSoftwareLeftoversPreviewResult,
  DesktopSoftwareLeftoversResult,
  DesktopSoftwarePreviewResult,
  DesktopSoftwareUninstallResult,
  DesktopSoftwareUpdatesResult,
  SoftwareEntryV1,
  SoftwareExecutionOutcome,
  SoftwareLeftoverAppV1,
  SoftwareLeftoverPreviewV1,
  SoftwareSizeEvidence,
  SoftwareStartupEntryV1,
} from "../../api/types.gen";
import { AccessibleUserData } from "../../app-shell/AppShell";
import { DestinationGlyph } from "../../app-shell/glyphs";
import { formatBinaryBytes, message, type MessageKey, type PresentationLanguageTag } from "../../i18n";
import { DetailView, Stage, StageResult } from "../../stage";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { selectSoftwareEntries, selectedSoftwareEntries, softwareIdentityText, softwareSource, summarizeSoftwareSize } from "./selectors";
import { initialSoftwareState, softwareReducer, uninstallSucceeded, type SoftwareOperation } from "./state";
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

function sizeEvidenceLabel(locale: PresentationLanguageTag, size: SoftwareSizeEvidence): string {
  if (size.state === "unknown") return message(locale, "software.v1.size.unknown");
  if (size.state === "partial") return message(locale, "software.v1.size.lower_bound", { bytes: formatBinaryBytes(size.lower_bound_bytes) });
  const key = size.basis === "reported_estimate" ? "software.v1.size.estimated"
    : size.basis === "measured_directory" ? "software.v1.size.measured_directory" : "software.v1.size.measured";
  return message(locale, key, { bytes: formatBinaryBytes(size.value_bytes) });
}

function sizeLabel(locale: PresentationLanguageTag, entry: SoftwareEntryV1): string {
  return sizeEvidenceLabel(locale, entry.size);
}

function outcomeLabel(locale: PresentationLanguageTag, outcome: SoftwareExecutionOutcome): string {
  return message(locale, `software.v1.outcome.${outcome}` as MessageKey);
}

export function SoftwareWorkbench({ bridge, coordinator, locale }: SoftwareWorkbenchProps) {
  const [state, dispatch] = useReducer(softwareReducer, initialSoftwareState);
  const [detail, setDetail] = useState<"inventory" | "updates" | "startup" | null>("inventory");
  const [startupPending, setStartupPending] = useState<string | null>(null);
  const mounted = useRef(true);
  useEffect(() => () => { mounted.current = false; }, []);
  // Startup entries are a light read-only registry and folder listing.
  useEffect(() => {
    let current = true;
    bridge.softwareStartupList().then(
      (list) => { if (current) dispatch({ type: "startup_listed", list }); },
      (error: unknown) => { if (current) dispatch({ type: "support_failed", error: commandError(error) }); },
    );
    return () => { current = false; };
  }, [bridge]);
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
  ): Promise<boolean> => {
    const operationId = createOperationId(operation);
    let started = false;
    let finished = false;
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
      if (!lease) return false;
      try {
        const result = await lease.result;
        if (mounted.current) {
          completed(operationId, result);
          finished = true;
        }
      } finally {
        await lease.complete().catch(() => false);
      }
    } catch (error) {
      if (started && mounted.current) dispatch({ type: "operation_failed", operationId, error: commandError(error) });
    }
    return finished;
  };

  const refreshInventory = () => void runOperation<DesktopSoftwareInventoryResult>(
    "inventory",
    (operationId) => bridge.softwareInventoryStart(operationId),
    (operationId, result) => {
      if (result.type === "completed") dispatch({ type: "inventory_completed", operationId, inventory: result.inventory });
      else dispatch({ type: "operation_canceled", operationId });
    },
  );

  // Leftover discovery is read-only and runs inside the preview operation, so
  // the preview stays usable when discovery fails.
  const reviewPreview = () => {
    if (!state.inventory || state.selectedIds.size === 0) return;
    const inventory = state.inventory;
    const selectedIds = [...state.selectedIds];
    let discoveryError: CommandError | null = null;
    void runOperation<{ result: DesktopSoftwarePreviewResult; leftovers?: SoftwareLeftoverPreviewV1 }>(
      "preview",
      async (operationId) => {
        const result = await bridge.softwarePreview(operationId, inventory, selectedIds);
        try {
          const discovered = await bridge.softwareLeftoversPreview(operationId, inventory, selectedIds, null);
          return discovered.type === "discovered" ? { result, leftovers: discovered.preview } : { result };
        } catch (error) {
          discoveryError = commandError(error);
          return { result };
        }
      },
      (operationId, { result, leftovers }) => {
        dispatch({ type: "preview_completed", operationId, result, leftovers });
        if (discoveryError) dispatch({ type: "support_failed", error: discoveryError });
      },
    );
  };

  const checkUpdates = () => {
    const inventory = state.inventory;
    void runOperation<DesktopSoftwareUpdatesResult>(
      "updates",
      (operationId) => bridge.softwareUpdatesCheck(operationId, inventory),
      (operationId, result) => dispatch({ type: "updates_completed", operationId, result }),
    );
  };

  const refreshStartup = async () => {
    setStartupPending("list");
    try {
      const list = await bridge.softwareStartupList();
      if (mounted.current) dispatch({ type: "startup_listed", list });
    } catch (error) {
      if (mounted.current) dispatch({ type: "support_failed", error: commandError(error) });
    } finally {
      if (mounted.current) setStartupPending(null);
    }
  };

  // The switch click is the explicit user confirmation for one current-user value.
  const toggleStartup = async (entry: SoftwareStartupEntryV1, enabled: boolean) => {
    setStartupPending(entry.id);
    try {
      const report = await bridge.softwareStartupSet(entry.id, enabled, true);
      if (!mounted.current) return;
      dispatch({ type: "startup_toggled", report });
      if (report.outcome !== "succeeded") {
        dispatch({ type: "support_failed", error: { code: "software_failed", message: message(locale, "software.v1.startup.toggle_failed") } });
      }
    } catch (error) {
      if (mounted.current) dispatch({ type: "support_failed", error: commandError(error) });
    } finally {
      if (mounted.current) setStartupPending(null);
    }
  };

  const planLeftovers = (app: SoftwareLeftoverAppV1, uninstallOperationId: string) => {
    if (!state.inventory) return;
    const inventory = state.inventory;
    const selection = {
      software_id: app.software_id,
      uninstall_operation_id: uninstallOperationId,
      selected_candidate_ids: app.candidates.filter((candidate) => state.leftoverSelection.has(candidate.id)).map((candidate) => candidate.id),
    };
    if (selection.selected_candidate_ids.length === 0) return;
    void runOperation<DesktopSoftwareLeftoversPreviewResult>(
      "leftover_plan",
      (operationId) => bridge.softwareLeftoversPreview(operationId, inventory, [app.software_id], selection),
      (operationId, result) => {
        if (result.type === "planned") dispatch({ type: "leftovers_planned", operationId, result });
        else dispatch({ type: "operation_failed", operationId, error: { code: "software_stale_authority", message: "Leftover plan response has no plan" } });
      },
    );
  };

  const moveLeftovers = () => {
    if (!state.leftoverPlan || !state.leftoverConfirming) return;
    const { plan, preview } = state.leftoverPlan;
    void runOperation<DesktopSoftwareLeftoversResult>(
      "leftover_move",
      (operationId) => bridge.softwareLeftoversExecute(operationId, plan, preview.digest, true),
      (operationId, result) => dispatch({ type: "leftovers_moved", operationId, result }),
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

  const statusText = state.status === "canceling" ? message(locale, "software.v1.state.canceling")
    : state.operation === "updates" ? message(locale, "software.v1.state.checking_updates")
    : state.operation === "leftover_plan" ? message(locale, "software.v1.state.leftovers")
    : state.operation === "leftover_move" ? message(locale, "software.v1.state.moving_leftovers")
    : state.status === "loading" ? message(locale, "software.v1.state.loading")
    : state.status === "previewing" ? message(locale, "software.v1.state.previewing")
      : state.status === "uninstalling" ? message(locale, "software.v1.state.uninstalling")
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
  const notChecked = message(locale, "software.v1.stage.not_checked");
  const stageFacts = <p>{message(locale, "software.v1.stage.facts", {
    installed: state.inventory ? String(state.inventory.entries.length) : notChecked,
    updates: state.updates === null ? notChecked
      : state.updates.state === "available" ? String(state.updates.rows.length) : message(locale, "software.v1.stage.unavailable"),
    startup: state.startup ? String(state.startup.entries.length) : notChecked,
  })}</p>;
  const leftoverApp = state.leftoverReport ? state.leftovers?.apps.find((app) => app.software_id === state.leftoverReport?.software_id) : undefined;
  const leftoverResult = state.leftoverReport ? <p>{message(locale, "software.v1.leftovers.result", {
    app: leftoverApp ? sizeEvidenceLabel(locale, leftoverApp.app_size) : message(locale, "software.v1.size.unknown"),
    leftovers: state.leftoverReport.lower_bound
      ? message(locale, "software.v1.size.lower_bound", { bytes: formatBinaryBytes(state.leftoverReport.moved_known_bytes) })
      : formatBinaryBytes(state.leftoverReport.moved_known_bytes),
  })}</p> : null;
  const viewLinks = <>
    <button type="button" className="stage-link" onClick={() => setDetail("updates")}>{message(locale, "software.v1.action.updates")}</button>
    <button type="button" className="stage-link" onClick={() => setDetail("startup")}>{message(locale, "software.v1.action.startup")}</button>
  </>;
  const cancelButton = active ? <button type="button" className="danger-button" disabled={state.status === "canceling"} onClick={() => void cancelActive()}>
    {message(locale, "software.v1.action.cancel")}
  </button> : null;
  const auditCard = state.audit ? <section className="card software-audit" aria-live="polite">
    <p>{message(locale, "software.v1.audit.summary", { records: String(state.audit.records.length), recovered: String(state.audit.recovered.length) })}</p>
    <details><summary>{message(locale, "software.v1.audit.transitions")}</summary>
      <ol>{state.audit.records.map((record, index) => <li key={`${record.operation_id}-${index}`}><AccessibleUserData value={`${record.operation_id} · ${record.status_code}`} /></li>)}</ol>
    </details>
  </section> : null;

  const updates = state.updates;
  const updatesView = <DetailView locale={locale} onBack={() => setDetail(null)} status={statusChip}>
    <section className="card software-updates" aria-busy={state.operation === "updates"} aria-labelledby="software-updates-title">
      <header className="software-inventory-header">
        <h2 id="software-updates-title">{message(locale, "software.v1.updates.title")}</h2>
        <div className="software-summary-actions">
          <button type="button" className="primary-button" disabled={active} onClick={checkUpdates}>{message(locale, "software.v1.action.updates_check")}</button>
          {cancelButton}
        </div>
      </header>
      <p>{message(locale, "software.v1.updates.detail")}</p>
      {liveStatus}
      {errorBanner}
      {updates === null ? <p>{message(locale, "software.v1.updates.not_checked")}</p>
        : updates.state === "unavailable" ? <div className="software-updates-unavailable" data-reason={updates.reason_code}>
          <strong>{message(locale, "software.v1.updates.unavailable")}</strong>
          <p>{message(locale, `software.v1.updates.reason.${updates.reason_code}` as MessageKey)}</p>
        </div>
          : updates.rows.length === 0 ? <p>{message(locale, "software.v1.updates.empty")}</p>
            : <>
              <p>{message(locale, "software.v1.updates.summary", { count: String(updates.rows.length) })}</p>
              <table className="software-table">
                <thead><tr>
                  <th scope="col">{message(locale, "software.v1.updates.column.name")}</th>
                  <th scope="col">{message(locale, "software.v1.updates.column.id")}</th>
                  <th scope="col">{message(locale, "software.v1.updates.column.installed")}</th>
                  <th scope="col">{message(locale, "software.v1.updates.column.available")}</th>
                  <th scope="col">{message(locale, "software.v1.updates.column.source")}</th>
                </tr></thead>
                <tbody>{updates.rows.map((row) => <tr key={row.id}>
                  <td>
                    <AccessibleUserData value={row.name} />
                    {row.name_truncated ? <span className="software-note"> · {message(locale, "software.v1.updates.truncated")}</span> : null}
                    {row.matched_software_ids.length > 0 ? <span className="eligible"> · {message(locale, "software.v1.updates.matched")}</span> : null}
                  </td>
                  <td><AccessibleUserData value={row.id} /></td>
                  <td><AccessibleUserData value={row.installed_version} /></td>
                  <td><AccessibleUserData value={row.available_version} /></td>
                  <td><AccessibleUserData value={row.source} /></td>
                </tr>)}</tbody>
              </table>
            </>}
    </section>
  </DetailView>;

  const startup = state.startup;
  const unreadableSources = startup?.sources.filter((source) => source.state !== "available").length ?? 0;
  const startupView = <DetailView locale={locale} onBack={() => setDetail(null)} status={statusChip}>
    <section className="card software-startup" aria-busy={startupPending !== null} aria-labelledby="software-startup-title">
      <header className="software-inventory-header">
        <h2 id="software-startup-title">{message(locale, "software.v1.startup.title")}</h2>
        <button type="button" className="secondary-button" disabled={startupPending !== null} onClick={() => void refreshStartup()}>
          {message(locale, "software.v1.action.startup_refresh")}
        </button>
      </header>
      <p>{message(locale, "software.v1.startup.detail")}</p>
      {errorBanner}
      {unreadableSources > 0 ? <p className="manual">{message(locale, "software.v1.startup.sources_unavailable", { count: String(unreadableSources) })}</p> : null}
      {startup === null ? <p>{notChecked}</p>
        : startup.entries.length === 0 ? <p>{message(locale, "software.v1.startup.empty")}</p>
          : <table className="software-table">
            <thead><tr>
              <th scope="col">{message(locale, "software.v1.startup.column.name")}</th>
              <th scope="col">{message(locale, "software.v1.startup.column.location")}</th>
              <th scope="col">{message(locale, "software.v1.startup.column.state")}</th>
              <th scope="col">{message(locale, "software.v1.startup.column.toggle")}</th>
            </tr></thead>
            <tbody>{startup.entries.map((entry) => <tr key={entry.id} data-state={entry.state}>
              <td><AccessibleUserData value={entry.name} /></td>
              <td>{message(locale, `software.v1.startup.location.${entry.location}` as MessageKey)}</td>
              <td>{message(locale, `software.v1.startup.state.${entry.state}` as MessageKey)}</td>
              <td>
                <input
                  type="checkbox"
                  role="switch"
                  checked={entry.state === "enabled"}
                  disabled={entry.toggle !== "allowed" || startupPending !== null}
                  aria-label={message(locale, "software.v1.startup.toggle", { name: entry.name })}
                  onChange={(event) => void toggleStartup(entry, event.target.checked)}
                />
                {entry.toggle === "requires_administrator" ? <span className="manual"> {message(locale, "software.v1.startup.requires_administrator")}</span> : null}
              </td>
            </tr>)}</tbody>
          </table>}
    </section>
  </DetailView>;

  const leftoverCard = state.leftovers ? <section className="card software-leftovers" aria-labelledby="software-leftovers-title">
    <h2 id="software-leftovers-title">{message(locale, "software.v1.leftovers.title")}</h2>
    <p>{message(locale, "software.v1.leftovers.detail")}</p>
    {state.leftovers.apps.map((app) => {
      const uninstallOperationId = uninstallSucceeded(state, app.software_id);
      const selectedCount = app.candidates.filter((candidate) => state.leftoverSelection.has(candidate.id)).length;
      return <div key={app.software_id} className="software-leftover-app">
        <h3><AccessibleUserData value={app.display_name ?? app.software_id} /></h3>
        <p>{message(locale, "software.v1.leftovers.app_size", { size: sizeEvidenceLabel(locale, app.app_size) })}</p>
        {app.candidates.length === 0 ? <p>{message(locale, "software.v1.leftovers.none")}</p>
          : <ul className="software-list">{app.candidates.map((candidate) => {
            const certainty = message(locale, `software.v1.leftovers.certainty.${candidate.certainty}` as MessageKey);
            return <li key={candidate.id} className="tile-row software-row" data-certainty={candidate.certainty}>
              <label className="software-selection">
                <input
                  type="checkbox"
                  checked={state.leftoverSelection.has(candidate.id)}
                  disabled={active}
                  aria-label={`${candidate.path}: ${certainty}`}
                  onChange={(event) => dispatch({ type: "leftover_selection_changed", candidateId: candidate.id, selected: event.target.checked })}
                />
              </label>
              <div className="tile-row-text">
                <strong><AccessibleUserData value={candidate.path} /></strong>
                <span className="secondary">
                  {`${message(locale, `software.v1.leftovers.origin.${candidate.origin}` as MessageKey)} · ${certainty} · ${sizeEvidenceLabel(locale, candidate.size)}`}
                </span>
              </div>
            </li>;
          })}</ul>}
        {uninstallOperationId ? <button type="button" className="secondary-button" disabled={active || selectedCount === 0} onClick={() => planLeftovers(app, uninstallOperationId)}>
          {message(locale, "software.v1.action.leftovers_review")}
        </button> : null}
      </div>;
    })}
    {state.leftoverPlan ? <div className="software-leftover-plan">
      <p><code>{message(locale, "software.v1.leftovers.plan", { digest: state.leftoverPlan.preview.digest })}</code></p>
      <button type="button" className="danger-button" disabled={active} onClick={() => dispatch({ type: "leftover_confirmation_opened" })}>
        {message(locale, "software.v1.action.leftovers_confirm")}
      </button>
    </div> : null}
    {state.leftoverReport ? <div className="software-results" aria-live="polite">
      {leftoverResult}
      <ul>{state.leftoverReport.outcomes.map((item) => <li key={item.candidate_id} data-outcome={item.outcome}>
        <AccessibleUserData value={leftoverApp?.candidates.find((candidate) => candidate.id === item.candidate_id)?.path ?? item.candidate_id} />
        {` — ${message(locale, `software.v1.leftovers.outcome.${item.outcome}` as MessageKey)}`}
      </li>)}</ul>
    </div> : null}
  </section> : null;

  const showStage = detail === null || (detail === "inventory" && !state.inventory);

  return <div className="software-mode" data-status={state.status}>
    {showStage ? <>
      {!state.inventory ? <Stage
        mode="software"
        label={message(locale, "command.software")}
        busy={active}
        title={message(locale, "software.v1.state.empty.title")}
        meta={<><p>{message(locale, "software.v1.state.empty.detail")}</p>{stageFacts}{liveStatus}</>}
        primary={active
          ? <button type="button" className="stage-action" disabled={state.status === "canceling"} onClick={() => void cancelActive()}>{message(locale, "software.v1.action.cancel")}</button>
          : <button type="button" className="stage-action" onClick={() => { setDetail("inventory"); refreshInventory(); }}>{message(locale, "software.v1.action.inventory")}</button>}
        secondary={<>
          <button type="button" className="stage-link" disabled={active} onClick={refreshAudit}>{message(locale, "software.v1.action.audit")}</button>
          {viewLinks}
        </>}
      /> : <StageResult
        mode="software"
        label={message(locale, "command.software")}
        caption={message(locale, "stage.v1.caption.software")}
        value={String(state.inventory.entries.length)}
        meta={<><p>{inventorySummary}</p>{stageFacts}{leftoverResult}{liveStatus}</>}
        action={<button type="button" className="stage-action" onClick={() => setDetail("inventory")}>{message(locale, "stage.v1.action.details")}</button>}
        secondary={viewLinks}
      />}
      {errorBanner}
      {!state.inventory ? auditCard : null}
    </> : detail === "updates" ? updatesView : detail === "startup" ? startupView : <DetailView locale={locale} onBack={() => setDetail(null)} status={statusChip}>
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

    {leftoverCard}

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

    {state.leftoverPlan ? <dialog className="software-confirm-dialog" open={state.leftoverConfirming} aria-labelledby="software-leftover-confirm-title">
      <h2 id="software-leftover-confirm-title">{message(locale, "software.v1.leftovers.confirm.title")}</h2>
      <p>{message(locale, "software.v1.leftovers.confirm.detail")}</p>
      <p><strong>{message(locale, "software.v1.leftovers.plan", { digest: state.leftoverPlan.preview.digest })}</strong></p>
      <ul>{state.leftoverPlan.preview.items.map((item) => <li key={item.id}><AccessibleUserData value={item.path} /></li>)}</ul>
      <div className="software-confirm-actions">
        <button type="button" className="secondary-button" onClick={() => dispatch({ type: "leftover_confirmation_closed" })}>{message(locale, "software.v1.action.close")}</button>
        <button type="button" className="danger-button" disabled={active} onClick={moveLeftovers}>{message(locale, "software.v1.action.leftovers_move")}</button>
      </div>
    </dialog> : null}
  </div>;
}
