import { useEffect, useRef, useState } from "react";
import type { ScanPreviewTarget, UntrustedTarget } from "../api/types.gen";
import { AccessibleUserData, DestinationGlyph } from "../app-shell";
import { message, type PresentationLanguageTag } from "../i18n";
import { riskLabel, scopeLabel } from "../modes/clean/labels";
import { formatBytes, formatEvidence } from "./format";

function capacity(target: Pick<UntrustedTarget, "estimated_bytes" | "size_complete">, locale: PresentationLanguageTag): string {
  if (target.size_complete) return formatBytes(target.estimated_bytes);
  if (target.estimated_bytes > 0) return message(locale, "clean.v1.capacity.partial", { bytes: formatBytes(target.estimated_bytes) });
  return message(locale, "clean.v1.capacity.unknown");
}

interface PreviewProps {
  locale?: PresentationLanguageTag;
  mode: "preview";
  targets: ScanPreviewTarget[];
}

interface ReviewProps {
  locale?: PresentationLanguageTag;
  mode: "review";
  targets: UntrustedTarget[];
  selectedIds: Set<string>;
  allSelected: boolean;
  disabled: boolean;
  showSelectAll?: boolean;
  /** Rows protected in this review. They render as inspect-only. */
  protectedIds?: ReadonlySet<string>;
  /** Renders the collapsed Skipped group: rows offer only Restore. */
  skipped?: boolean;
  onSelect: (id: string, selected: boolean) => void;
  onSelectAll: (selected: boolean) => void;
  onSkip?: (id: string) => void;
  onRestore?: (id: string) => void;
  onProtect?: (target: UntrustedTarget) => void;
  onRescan?: (target: UntrustedTarget) => void;
  onAnalyze?: (target: UntrustedTarget) => void;
}

function RowActions({ locale, props, target, inspectOnly }: { locale: PresentationLanguageTag; props: ReviewProps; target: UntrustedTarget; inspectOnly: boolean }) {
  const name = target.path ?? target.id;
  const skipLabel = message(locale, "clean.v1.action.skip");
  const restoreLabel = message(locale, "clean.v1.action.restore");
  const protectLabel = message(locale, "clean.v1.action.protect");
  if (props.skipped) {
    return <td className="row-actions">{props.onRestore && <button type="button" className="secondary-button" aria-label={`${restoreLabel} ${name}`} disabled={props.disabled} onClick={() => props.onRestore?.(target.id)}>{restoreLabel}</button>}</td>;
  }
  const isProtected = props.protectedIds?.has(target.id) ?? false;
  return <td className="row-actions">
    {target.path && props.onRescan && <button type="button" className="secondary-button" disabled={props.disabled} onClick={() => props.onRescan?.(target)}>{message(locale, "clean.v1.rescan.action")}</button>}
    {target.path && inspectOnly && props.onAnalyze && <button type="button" className="secondary-button" disabled={props.disabled} onClick={() => props.onAnalyze?.(target)}>{message(locale, "clean.v1.rescan.analyze")}</button>}
    {!inspectOnly && props.onSkip && <button type="button" className="secondary-button" aria-label={`${skipLabel} ${name}`} disabled={props.disabled} onClick={() => props.onSkip?.(target.id)}>{skipLabel}</button>}
    {target.path && !isProtected && props.onProtect && <button type="button" className="secondary-button" aria-label={`${protectLabel} ${name}`} disabled={props.disabled} onClick={() => props.onProtect?.(target)}>{protectLabel}</button>}
  </td>;
}

function TargetEvidence({ target }: { target: ScanPreviewTarget | UntrustedTarget }) {
  const [open, setOpen] = useState(false);
  return <details onToggle={(event) => setOpen(event.currentTarget.open)}>
    <summary>{target.evidence.length}</summary>
    {open && <>
      <ul>{target.evidence.map((item, index) => <li key={`${item.type}-${index}`}>{formatEvidence(item)}</li>)}</ul>
      {target.sizing_warnings?.map((warning) => <p className="warning-text" key={warning.kind}>{warning.detail}</p>)}
    </>}
  </details>;
}

export function TargetTable(props: PreviewProps | ReviewProps) {
  const locale = props.locale ?? "en";
  const [requestedPage, setPage] = useState(0);
  const pageCount = Math.max(1, Math.ceil(props.targets.length / 100));
  const page = Math.min(requestedPage, pageCount - 1);
  const pageTargets = props.targets.slice(page * 100, (page + 1) * 100);
  const reviewTargets = props.mode === "review" ? props.targets : [];
  const protectedIds = props.mode === "review" ? props.protectedIds : undefined;
  const reviewInspectOnly = (target: UntrustedTarget) => target.intent.type === "inspect_only" || (protectedIds?.has(target.id) ?? false);
  let executableCount = 0;
  let selectedExecutableCount = 0;
  if (props.mode === "review" && props.showSelectAll !== false) {
    for (const target of reviewTargets) if (!reviewInspectOnly(target)) {
      executableCount += 1;
      if (props.selectedIds.has(target.id)) selectedExecutableCount += 1;
    }
  }
  const selectAllRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (selectAllRef.current) selectAllRef.current.indeterminate = selectedExecutableCount > 0 && selectedExecutableCount < executableCount;
  }, [executableCount, selectedExecutableCount]);
  return <div className="target-list">
    {pageCount > 1 && <div className="target-pagination">
      <p role="status">{message(locale, "clean.v1.list.page", { page: String(page + 1), pages: String(pageCount), start: String(page * 100 + 1), end: String(Math.min((page + 1) * 100, props.targets.length)), count: String(props.targets.length) })}</p>
      <div>
        <button type="button" className="secondary-button" disabled={page === 0} onClick={() => setPage(0)}>{message(locale, "clean.v1.list.first")}</button>
        <button type="button" className="secondary-button" disabled={page === 0} onClick={() => setPage(page - 1)}>{message(locale, "clean.v1.list.previous")}</button>
        <button type="button" className="secondary-button" disabled={page + 1 === pageCount} onClick={() => setPage(page + 1)}>{message(locale, "clean.v1.list.next")}</button>
        <button type="button" className="secondary-button" disabled={page + 1 === pageCount} onClick={() => setPage(pageCount - 1)}>{message(locale, "clean.v1.list.last")}</button>
      </div>
    </div>}
    <div className="table-frame">
    <table className={props.mode === "preview" ? "preview-table" : undefined}>
      <thead><tr>
        {props.mode === "review" && <th className="select-cell">{props.showSelectAll === false ? null : <input ref={selectAllRef} type="checkbox" aria-label="Select all executable targets" checked={props.allSelected} disabled={props.disabled || executableCount === 0} onChange={(event) => props.onSelectAll(event.target.checked)} />}</th>}
        <th>{message(locale, "clean.v1.table.target")}</th><th>{message(locale, "clean.v1.table.capacity")}</th><th>{message(locale, "clean.v1.table.risk")}</th><th>{props.mode === "preview" ? message(locale, "clean.v1.status.ready") : message(locale, "clean.v1.action.execute")}</th><th>{message(locale, "clean.v1.table.evidence")}</th>
        {props.mode === "review" && <th>{message(locale, "clean.v1.table.actions")}</th>}
      </tr></thead>
      <tbody>{pageTargets.map((target) => {
        const previewTarget = "disposition" in target;
        const isProtected = !previewTarget && (protectedIds?.has(target.id) ?? false);
        const inspectOnly = previewTarget ? target.disposition === "inspect_only" : reviewInspectOnly(target);
        const skippedRow = props.mode === "review" && props.skipped === true;
        return <tr key={target.id}>
          {props.mode === "review" && <td className="select-cell"><input type="checkbox" aria-label={`Select ${target.path ?? target.id}`} checked={props.selectedIds.has(target.id)} disabled={props.disabled || inspectOnly || skippedRow} title={inspectOnly ? message(locale, "clean.v1.error.inspect_only") : undefined} onChange={(event) => props.onSelect(target.id, event.target.checked)} /></td>}
          <td className="target-primary">
            <span className="glyph-tile"><DestinationGlyph name={target.ecosystem} /></span>
            <span className="target-copy">
              <strong className="target-name"><AccessibleUserData value={target.path ?? target.id} /></strong>
              <span className="secondary">{target.ecosystem} · {scopeLabel(locale, target.scope.type)}</span>
            </span>
          </td>
          <td className="capacity">{capacity(target, locale)}</td>
          <td><span className={`badge risk-${target.risk}`}>{riskLabel(locale, target.risk)}</span></td>
          <td>{isProtected ? <span className="badge neutral">{message(locale, "clean.v1.target.protected")}</span> : inspectOnly ? <span className="badge neutral">{message(locale, "clean.v1.target.inspect_only")}</span> : previewTarget ? <span className="badge neutral">{message(locale, "clean.v1.status.ready")}</span> : message(locale, target.intent.type === "run_built_in_action" ? "clean.v1.result.command_action" : "clean.v1.action.move_to_trash")}</td>
          <td><TargetEvidence target={target} /></td>
          {props.mode === "review" && !previewTarget && <RowActions locale={locale} props={props} target={target} inspectOnly={inspectOnly} />}
        </tr>;
      })}</tbody>
    </table>
    </div>
  </div>;
}
