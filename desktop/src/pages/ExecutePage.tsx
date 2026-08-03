import type { ExecutionReport, TargetOutcome } from "../api/types.gen";
import { formatCapacity, formatTotals } from "../components/format";

function actionLabel(outcome: TargetOutcome): string {
  switch (outcome.action.type) {
    case "command": return outcome.action.irreversible ? "Irreversible command" : "Command";
    case "move_to_trash": return "Move to trash";
    case "inspect_only": return "Inspect only";
    case "permanent_delete": return "Permanent delete disabled";
  }
}
function statusLabel(outcome: TargetOutcome): string {
  switch (outcome.status.type) {
    case "succeeded": return outcome.action.type === "move_to_trash" ? "Moved to trash; capacity becomes available after trash is emptied" : "Succeeded";
    case "failed": return `Failed: ${outcome.status.message}`;
    case "skipped": return `Skipped: ${outcome.status.reason}`;
  }
}
function OutcomeTable({ report }: { report: ExecutionReport }) {
  return <div className="outcome-list">{report.outcomes.map((outcome) => <div className="outcome-row" key={outcome.target_id}><div><strong>{outcome.target_id}</strong><span className="secondary">{actionLabel(outcome)}</span></div><span className={`outcome-status status-${outcome.status.type}`}>{statusLabel(outcome)}</span><span className="capacity">{formatCapacity(outcome.estimated_recoverable)}</span></div>)}</div>;
}
export function ExecutePage(props: { report: ExecutionReport; final: boolean; onConfirm: () => void; onReturn: () => void }) {
  return <section className="execute-panel">
    <header className="section-heading"><div><h2>{props.final ? "Cleanup report" : "Dry-run preview"}</h2><p>{props.report.selected} selected · {props.report.succeeded} succeeded · {props.report.failed} failed · {props.report.skipped} skipped</p></div><div className="capacity-total"><span>Estimated recoverable</span><strong>{formatTotals(props.report.estimated_recoverable)}</strong></div></header>
    <OutcomeTable report={props.report} />
    <div className="digest-line"><span>Confirmation digest</span><code>{props.report.confirmation_digest}</code></div>
    <footer className="action-bar"><button className="secondary-button" onClick={props.onReturn}>{props.final ? "Back to targets" : "Change selection"}</button>{!props.final && <button className="danger-button wide-action" onClick={props.onConfirm}>Continue to confirmation</button>}</footer>
  </section>;
}
