import { useEffect, useRef } from "react";
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
    {!inspectOnly && props.onSkip && <button type="button" className="secondary-button" aria-label={`${skipLabel} ${name}`} disabled={props.disabled} onClick={() => props.onSkip?.(target.id)}>{skipLabel}</button>}
    {target.path && !isProtected && props.onProtect && <button type="button" className="secondary-button" aria-label={`${protectLabel} ${name}`} disabled={props.disabled} onClick={() => props.onProtect?.(target)}>{protectLabel}</button>}
  </td>;
}

export function TargetTable(props: PreviewProps | ReviewProps) {
  const locale = props.locale ?? "en";
  const reviewTargets = props.mode === "review" ? props.targets : [];
  const protectedIds = props.mode === "review" ? props.protectedIds : undefined;
  const reviewInspectOnly = (target: UntrustedTarget) => target.intent.type === "inspect_only" || (protectedIds?.has(target.id) ?? false);
  const executableCount = reviewTargets.filter((target) => !reviewInspectOnly(target)).length;
  const selectedExecutableCount = props.mode === "review"
    ? reviewTargets.filter((target) => !reviewInspectOnly(target) && props.selectedIds.has(target.id)).length
    : 0;
  const selectAllRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (selectAllRef.current) selectAllRef.current.indeterminate = selectedExecutableCount > 0 && selectedExecutableCount < executableCount;
  }, [executableCount, selectedExecutableCount]);
  return <div className="table-frame">
    <table className={props.mode === "preview" ? "preview-table" : undefined}>
      <thead><tr>
        {props.mode === "review" && <th className="select-cell">{props.showSelectAll === false ? null : <input ref={selectAllRef} type="checkbox" aria-label="Select all executable targets" checked={props.allSelected} disabled={props.disabled || executableCount === 0} onChange={(event) => props.onSelectAll(event.target.checked)} />}</th>}
        <th>{message(locale, "clean.v1.table.target")}</th><th>{message(locale, "clean.v1.table.capacity")}</th><th>{message(locale, "clean.v1.table.risk")}</th><th>{props.mode === "preview" ? message(locale, "clean.v1.status.ready") : message(locale, "clean.v1.action.execute")}</th><th>{message(locale, "clean.v1.table.evidence")}</th>
        {props.mode === "review" && <th>{message(locale, "clean.v1.table.actions")}</th>}
      </tr></thead>
      <tbody>{props.targets.map((target) => {
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
          <td>{isProtected ? <span className="badge neutral">{message(locale, "clean.v1.target.protected")}</span> : inspectOnly ? <span className="badge neutral">{message(locale, "clean.v1.target.inspect_only")}</span> : previewTarget ? <span className="badge neutral">{message(locale, "clean.v1.status.ready")}</span> : message(locale, "clean.v1.trash.moved")}</td>
          <td><details><summary>{target.evidence.length}</summary><ul>{target.evidence.map((item, index) => <li key={`${item.type}-${index}`}>{formatEvidence(item)}</li>)}</ul>{target.sizing_warnings?.map((warning) => <p className="warning-text" key={warning.kind}>{warning.detail}</p>)}</details></td>
          {props.mode === "review" && !previewTarget && <RowActions locale={locale} props={props} target={target} inspectOnly={inspectOnly} />}
        </tr>;
      })}</tbody>
    </table>
  </div>;
}
