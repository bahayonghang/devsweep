import type { AppState } from "../state/app-state";
import { allExecutableSelected, selectedTotals } from "../state/selectors";
import { formatTotals } from "../components/format";
import { TargetTable } from "../components/TargetTable";

export function ReviewPage(props: { state: AppState; onSelect: (id: string, selected: boolean) => void; onSelectAll: (selected: boolean) => void; onDryRun: () => void }) {
  if (!props.state.scan) return <section className="empty-state"><h2>No scan results</h2><p>Run a scan to review cleanup targets.</p></section>;
  const targets = props.state.scan.plan.targets;
  if (targets.length === 0) return <section className="empty-state completed-empty"><h2>Scan complete; no cleanup targets found</h2><p>The requested scopes finished without producing cleanup candidates.</p></section>;
  const busy = props.state.pending !== null || props.state.phase === "executing";
  return <section className="workspace">
    <header className="section-heading"><div><h2>Cleanup targets</h2><p>{targets.length} found · scan {props.state.scan.health.completeness}</p></div>{props.state.scan.health.diagnostics.length > 0 && <details className="diagnostics"><summary>{props.state.scan.health.diagnostics.length} scan notices</summary>{props.state.scan.health.diagnostics.map((item, index) => <p key={`${item.path}-${index}`}><strong>{item.stage}:</strong> {item.detail}</p>)}</details>}</header>
    <TargetTable mode="review" targets={targets} selectedIds={props.state.selectedIds} allSelected={allExecutableSelected(props.state)} disabled={busy} onSelect={props.onSelect} onSelectAll={props.onSelectAll} />
    <footer className="action-bar"><div><span className="summary-label">Estimated recoverable</span><strong>{formatTotals(selectedTotals(props.state))}</strong><span className="secondary">{props.state.selectedIds.size} selected</span></div><button className="primary-button wide-action" disabled={props.state.selectedIds.size === 0 || busy} onClick={props.onDryRun}>{props.state.pending === "dry_run" ? "Preparing dry run…" : "Review dry run"}</button></footer>
  </section>;
}
