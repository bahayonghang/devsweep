import { useEffect, useRef } from "react";
import type { ScanPreviewTarget, UntrustedTarget } from "../api/types.gen";
import { message, type PresentationLanguageTag } from "../i18n";
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
  onSelect: (id: string, selected: boolean) => void;
  onSelectAll: (selected: boolean) => void;
}

export function TargetTable(props: PreviewProps | ReviewProps) {
  const locale = props.locale ?? "en";
  const reviewTargets = props.mode === "review" ? props.targets : [];
  const executableCount = reviewTargets.filter((target) => target.intent.type !== "inspect_only").length;
  const selectedExecutableCount = props.mode === "review"
    ? reviewTargets.filter((target) => target.intent.type !== "inspect_only" && props.selectedIds.has(target.id)).length
    : 0;
  const selectAllRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (selectAllRef.current) selectAllRef.current.indeterminate = selectedExecutableCount > 0 && selectedExecutableCount < executableCount;
  }, [executableCount, selectedExecutableCount]);
  return <div className="table-frame">
    <table className={props.mode === "preview" ? "preview-table" : undefined}>
      <thead><tr>
        {props.mode === "review" && <th className="select-cell">{props.showSelectAll === false ? null : <input ref={selectAllRef} type="checkbox" aria-label="Select all executable targets" checked={props.allSelected} disabled={props.disabled || executableCount === 0} onChange={(event) => props.onSelectAll(event.target.checked)} />}</th>}
        <th>{message(locale, "clean.v1.table.target")}</th><th>{message(locale, "clean.v1.table.category")}</th><th>{message(locale, "clean.v1.table.capacity")}</th><th>{message(locale, "clean.v1.table.risk")}</th><th>{props.mode === "preview" ? message(locale, "clean.v1.status.ready") : message(locale, "clean.v1.action.execute")}</th><th>{message(locale, "clean.v1.table.evidence")}</th>
      </tr></thead>
      <tbody>{props.targets.map((target) => {
        const previewTarget = "disposition" in target;
        const inspectOnly = previewTarget ? target.disposition === "inspect_only" : target.intent.type === "inspect_only";
        return <tr key={target.id}>
          {props.mode === "review" && <td className="select-cell"><input type="checkbox" aria-label={`Select ${target.path ?? target.id}`} checked={props.selectedIds.has(target.id)} disabled={props.disabled || inspectOnly} title={inspectOnly ? message(locale, "clean.v1.error.inspect_only") : undefined} onChange={(event) => props.onSelect(target.id, event.target.checked)} /></td>}
          <td><strong className="target-name">{target.path ?? target.id}</strong><span className="secondary">{target.ecosystem} · {target.scope.type}</span></td>
          <td>{target.kind.replaceAll("_", " ")}</td>
          <td className="capacity">{capacity(target, locale)}</td>
          <td><span className={`badge risk-${target.risk}`}>{target.risk}</span></td>
          <td>{inspectOnly ? <span className="badge neutral">{message(locale, "clean.v1.target.inspect_only")}</span> : previewTarget ? <span className="badge neutral">{message(locale, "clean.v1.status.ready")}</span> : message(locale, "clean.v1.trash.moved")}</td>
          <td><details><summary>{target.evidence.length}</summary><ul>{target.evidence.map((item, index) => <li key={`${item.type}-${index}`}>{formatEvidence(item)}</li>)}</ul>{target.sizing_warnings?.map((warning) => <p className="warning-text" key={warning.kind}>{warning.detail}</p>)}</details></td>
        </tr>;
      })}</tbody>
    </table>
  </div>;
}
