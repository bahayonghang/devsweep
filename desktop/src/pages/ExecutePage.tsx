import type { ExecutionReport, TargetOutcome, UntrustedTarget } from "../api/types.gen";
import { formatCapacity, formatTotals } from "../components/format";
import { message, type PresentationLanguageTag } from "../i18n";
import { kindLabel } from "../modes/clean/labels";
import { movedTotals } from "../modes/clean/result";

function actionLabel(locale: PresentationLanguageTag, outcome: TargetOutcome): string {
  switch (outcome.action.type) {
    case "command": return message(locale, "clean.v1.result.command_action");
    case "move_to_trash": return message(locale, "clean.v1.action.move_to_trash");
    case "inspect_only": return message(locale, "clean.v1.target.inspect_only");
    case "permanent_delete": return message(locale, "clean.v1.error.inspect_only");
  }
}
function statusLabel(locale: PresentationLanguageTag, outcome: TargetOutcome, final: boolean): string {
  if (!final && outcome.status.type !== "failed") return message(locale, "clean.v1.outcome.dry_run");
  switch (outcome.status.type) {
    case "succeeded": return outcome.action.type === "move_to_trash" ? message(locale, "clean.v1.trash.moved") : message(locale, "clean.v1.execute.completed");
    case "failed": return message(locale, "clean.v1.result.failed_detail", { detail: outcome.status.message });
    case "skipped": return message(locale, "clean.v1.outcome.skipped", { reason: outcome.status.reason });
  }
}
function OutcomeTable({ locale, report, targets, final }: { locale: PresentationLanguageTag; report: ExecutionReport; targets: readonly UntrustedTarget[]; final: boolean }) {
  const byId = new Map(targets.map((target) => [target.id, target]));
  return <div className="outcome-list">{report.outcomes.map((outcome) => {
    const target = byId.get(outcome.target_id);
    return <div className="outcome-row" key={outcome.target_id} title={outcome.target_id}><div><strong>{target ? target.path ?? kindLabel(locale, target.kind) : outcome.target_id}</strong><span className="secondary">{target ? `${kindLabel(locale, target.kind)} · ` : ""}{actionLabel(locale, outcome)}</span></div><span className={`outcome-status status-${outcome.status.type}`}>{statusLabel(locale, outcome, final)}</span><span className="capacity">{formatCapacity(outcome.estimated_recoverable, locale)}</span></div>;
  })}</div>;
}
export function ExecutePage(props: { locale?: PresentationLanguageTag; report: ExecutionReport; targets?: readonly UntrustedTarget[]; final: boolean; onConfirm: () => void; onReturn: () => void }) {
  const locale = props.locale ?? "en";
  const totals = formatTotals(props.final ? movedTotals(props.report) : props.report.estimated_recoverable, locale);
  const moved = props.report.outcomes.some((outcome) => outcome.status.type === "succeeded" && outcome.action.type === "move_to_trash");
  const commands = props.report.outcomes.filter((outcome) => outcome.status.type === "succeeded" && outcome.action.type === "command").length;
  return <section className="card execute-panel">
    <header className="section-heading"><div><h2>{props.final ? message(locale, props.report.succeeded === 0 ? "clean.v1.result.no_changes" : "clean.v1.execute.completed") : message(locale, "clean.v1.preview.title")}</h2><p>{message(locale, "clean.v1.preview.selected", { count: String(props.report.selected) })}</p></div>{(!props.final || moved) && <div className="capacity-total"><strong className="display-capacity">{totals}</strong><span>{message(locale, "clean.v1.preview.estimated", { bytes: totals })}</span>{props.final ? <span>{message(locale, "clean.v1.trash.moved")}</span> : null}</div>}</header>
    {props.final && <p>{message(locale, "clean.v1.result.counts", { succeeded: String(props.report.succeeded), skipped: String(props.report.skipped), failed: String(props.report.failed) })}</p>}
    {props.final && commands > 0 && <p>{message(locale, "clean.v1.result.commands", { count: String(commands) })} {message(locale, "clean.v1.result.commands_caption")}</p>}
    <OutcomeTable locale={locale} report={props.report} targets={props.targets ?? []} final={props.final} />
    <div className="digest-line"><span>{message(locale, "clean.v1.preview.digest", { digest: props.report.confirmation_digest })}</span></div>
    {!props.final && <footer className="action-bar"><button className="secondary-button" onClick={props.onReturn}>{message(locale, "clean.v1.action.back_review")}</button><button className="danger-button wide-action" onClick={props.onConfirm}>{message(locale, "clean.v1.confirm.title")}</button></footer>}
  </section>;
}
