import { useEffect, useRef } from "react";
import type { UntrustedTarget } from "../api/types.gen";
import { formatBytes, formatEvidence } from "./format";

function capacity(target: UntrustedTarget): string {
  if (target.size_complete) return formatBytes(target.estimated_bytes);
  if (target.estimated_bytes > 0) return `At least ${formatBytes(target.estimated_bytes)}`;
  return "Unknown";
}

export function TargetTable(props: {
  targets: UntrustedTarget[];
  selectedIds: Set<string>;
  allSelected: boolean;
  disabled: boolean;
  onSelect: (id: string, selected: boolean) => void;
  onSelectAll: (selected: boolean) => void;
}) {
  const executableCount = props.targets.filter((target) => target.intent.type !== "inspect_only").length;
  const selectedExecutableCount = props.targets.filter((target) => target.intent.type !== "inspect_only" && props.selectedIds.has(target.id)).length;
  const selectAllRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (selectAllRef.current) selectAllRef.current.indeterminate = selectedExecutableCount > 0 && selectedExecutableCount < executableCount;
  }, [executableCount, selectedExecutableCount]);
  return <div className="table-frame">
    <table>
      <thead><tr>
        <th className="select-cell"><input ref={selectAllRef} type="checkbox" aria-label="Select all executable targets" checked={props.allSelected} disabled={props.disabled || executableCount === 0} onChange={(event) => props.onSelectAll(event.target.checked)} /></th>
        <th>Target</th><th>Category</th><th>Capacity</th><th>Risk</th><th>Action</th><th>Evidence</th>
      </tr></thead>
      <tbody>{props.targets.map((target) => {
        const inspectOnly = target.intent.type === "inspect_only";
        return <tr key={target.id}>
          <td className="select-cell"><input type="checkbox" aria-label={`Select ${target.path ?? target.id}`} checked={props.selectedIds.has(target.id)} disabled={props.disabled || inspectOnly} title={inspectOnly ? "Inspect-only targets cannot be cleaned" : undefined} onChange={(event) => props.onSelect(target.id, event.target.checked)} /></td>
          <td><strong className="target-name">{target.path ?? target.id}</strong><span className="secondary">{target.ecosystem} · {target.scope.type}</span></td>
          <td>{target.kind.replaceAll("_", " ")}</td>
          <td className="capacity">{capacity(target)}</td>
          <td><span className={`badge risk-${target.risk}`}>{target.risk}</span></td>
          <td>{inspectOnly ? <span className="badge neutral">Inspect only</span> : target.reversible ? "Move to trash" : <span className="badge warning">Irreversible</span>}</td>
          <td><details><summary>{target.evidence.length} items</summary><ul>{target.evidence.map((item, index) => <li key={`${item.type}-${index}`}>{formatEvidence(item)}</li>)}</ul>{target.sizing_warnings?.map((warning) => <p className="warning-text" key={warning.kind}>{warning.detail}</p>)}</details></td>
        </tr>;
      })}</tbody>
    </table>
  </div>;
}
