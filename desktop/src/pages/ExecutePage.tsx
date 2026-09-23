import type { ExecutionReport, TargetOutcome } from "../api/types.gen";
import { formatCapacity, formatTotals } from "../components/format";
import { message, type PresentationLanguageTag } from "../i18n";

function actionLabel(locale: PresentationLanguageTag, outcome: TargetOutcome): string {
  switch (outcome.action.type) {
    case "command": return message(locale, "clean.v1.action.execute");
    case "move_to_trash": return message(locale, "clean.v1.trash.moved");
    case "inspect_only": return message(locale, "clean.v1.target.inspect_only");
    case "permanent_delete": return message(locale, "clean.v1.error.inspect_only");
  }
}
function statusLabel(locale: PresentationLanguageTag, outcome: TargetOutcome, final: boolean): string {
  switch (outcome.status.type) {
    case "succeeded": return outcome.action.type === "move_to_trash" ? message(locale, "clean.v1.trash.moved") : message(locale, "clean.v1.execute.completed");
    case "failed": return message(locale, "clean.v1.execute.failed");
    case "skipped": return final ? message(locale, "clean.v1.outcome.skipped", { reason: outcome.status.reason }) : message(locale, "clean.v1.outcome.dry_run");
  }
}
function OutcomeTable({ locale, report, final }: { locale: PresentationLanguageTag; report: ExecutionReport; final: boolean }) {
  return <div className="outcome-list">{report.outcomes.map((outcome) => <div className="outcome-row" key={outcome.target_id}><div><strong>{outcome.target_id}</strong><span className="secondary">{actionLabel(locale, outcome)}</span></div><span className={`outcome-status status-${outcome.status.type}`}>{statusLabel(locale, outcome, final)}</span><span className="capacity">{formatCapacity(outcome.estimated_recoverable)}</span></div>)}</div>;
}
export function ExecutePage(props: { locale?: PresentationLanguageTag; report: ExecutionReport; final: boolean; onConfirm: () => void; onReturn: () => void }) {
  const locale = props.locale ?? "en";
  const totals = formatTotals(props.report.estimated_recoverable);
  return <section className="card execute-panel">
    <header className="section-heading"><div><h2>{props.final ? message(locale, "clean.v1.execute.completed") : message(locale, "clean.v1.preview.title")}</h2><p>{message(locale, "clean.v1.preview.selected", { count: String(props.report.selected) })}</p></div><div className="capacity-total"><strong className="display-capacity">{totals}</strong><span>{message(locale, "clean.v1.preview.estimated", { bytes: totals })}</span>{props.final ? <span>{message(locale, "clean.v1.trash.moved")}</span> : null}</div></header>
    <OutcomeTable locale={locale} report={props.report} final={props.final} />
    <div className="digest-line"><span>{message(locale, "clean.v1.preview.digest", { digest: props.report.confirmation_digest })}</span></div>
    <footer className="action-bar"><button className="secondary-button" onClick={props.onReturn}>{message(locale, "clean.v1.action.cancel")}</button>{!props.final && <button className="danger-button wide-action" onClick={props.onConfirm}>{message(locale, "clean.v1.confirm.title")}</button>}</footer>
  </section>;
}
