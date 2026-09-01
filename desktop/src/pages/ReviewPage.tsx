import type { AppState } from "../state/app-state";
import { allExecutableSelected, selectedTotals } from "../state/selectors";
import { formatTotals } from "../components/format";
import { TargetTable } from "../components/TargetTable";
import { message, type PresentationLanguageTag } from "../i18n";

export function ReviewPage(props: {
  locale?: PresentationLanguageTag;
  state: AppState;
  onSelect: (id: string, selected: boolean) => void;
  onSelectAll: (selected: boolean) => void;
  onDryRun: () => void;
}) {
  const locale = props.locale ?? "en";
  if (!props.state.scan) return <section className="empty-state"><h2>{message(locale, "clean.v1.review.empty_title")}</h2><p>{message(locale, "clean.v1.review.empty_hint")}</p></section>;
  const targets = props.state.scan.plan.targets;
  if (targets.length === 0) return <section className="empty-state completed-empty"><h2>{message(locale, "clean.v1.scan.empty")}</h2><p>{message(locale, "clean.v1.review.empty_hint")}</p></section>;
  const busy = props.state.pending !== null || props.state.phase === "executing";
  return <section className="workspace">
    <header className="section-heading"><div><h2>{message(locale, "command.clean")}</h2><p>{message(locale, "clean.v1.scan.complete", { count: String(targets.length) })}</p></div>{props.state.scan.health.diagnostics.length > 0 && <details className="diagnostics"><summary>{props.state.scan.health.diagnostics.length}</summary>{props.state.scan.health.diagnostics.map((item, index) => <p key={`${item.path}-${index}`}><strong>{item.stage}:</strong> {item.detail}</p>)}</details>}</header>
    <TargetTable locale={locale} mode="review" targets={targets} selectedIds={props.state.selectedIds} allSelected={allExecutableSelected(props.state)} disabled={busy} onSelect={props.onSelect} onSelectAll={props.onSelectAll} />
    <footer className="action-bar"><div><span className="summary-label">{message(locale, "clean.v1.preview.estimated", { bytes: formatTotals(selectedTotals(props.state)) })}</span><span className="secondary">{message(locale, "clean.v1.summary.selected", { count: String(props.state.selectedIds.size) })}</span></div><button className="primary-button wide-action" disabled={props.state.selectedIds.size === 0 || busy} onClick={props.onDryRun}>{message(locale, "clean.v1.action.preview")}</button></footer>
  </section>;
}
