import { useState } from "react";
import type { CleanAdvisoriesV1, ProjectCapacitySubtotalV1 } from "../../api/types.gen";
import { DestinationGlyph } from "../../app-shell";
import { formatBytes, formatTotals } from "../../components/format";
import { message, type PresentationLanguageTag } from "../../i18n";
import { selectedTotals } from "../../state/selectors";
import { ErrorBanner } from "../../components/ErrorBanner";
import { ScanCoverage } from "./ScanCoverage";
import { ScanRoots } from "./ScanRoots";
import { useCleanSession } from "./CleanSession";
import type { CleanScope } from "./session-state";
import { validThreshold } from "./useCleanAdvisory";
import { movedTotals } from "./result";

type Session = NonNullable<ReturnType<typeof useCleanSession>>;
function subtotal(locale: PresentationLanguageTag, value: ProjectCapacitySubtotalV1) {
  return value.confidence === "unknown" ? message(locale, "clean.v1.capacity.unknown")
    : value.confidence === "lower_bound" ? message(locale, "clean.v1.capacity.partial", { bytes: formatBytes(value.known_bytes) })
      : formatBytes(value.known_bytes);
}

function ProjectAdvisories({ value, locale }: { value: CleanAdvisoriesV1; locale: PresentationLanguageTag }) {
  const [requestedPage, setPage] = useState(0);
  const pages = Math.max(1, Math.ceil(value.projects.length / 10));
  const page = Math.min(requestedPage, pages - 1);
  return <div className="clean-owner-list">
    {value.projects.slice(page * 10, (page + 1) * 10).map(project => <article key={project.owner_root} className="clean-owner">
      <h3>{project.owner_root}</h3>
      <span className={`clean-assessment ${project.assessment}`}><DestinationGlyph name={project.assessment === "oversized" ? "risk" : "build_artifacts"} />{message(locale, `clean.v2.${project.assessment}`)}</span>
      <dl><div><dt>{message(locale, "clean.v2.artifacts")}</dt><dd>{subtotal(locale, project.artifacts)}</dd></div>
        <div><dt>{message(locale, "clean.v2.dependencies")}</dt><dd>{subtotal(locale, project.dependencies)}</dd></div></dl>
      {project.excluded_target_ids.length > 0 && <p className="secondary">{message(locale, "clean.v2.excluded", { count: String(project.excluded_target_ids.length) })}</p>}
    </article>)}
    {pages > 1 && <div className="target-pagination"><span>{page + 1} / {pages}</span><button type="button" disabled={page === 0} onClick={() => setPage(page - 1)}>{message(locale, "clean.v1.list.previous")}</button><button type="button" disabled={page + 1 === pages} onClick={() => setPage(page + 1)}>{message(locale, "clean.v1.list.next")}</button></div>}
  </div>;
}

export function CleanOverview({ session, locale, busy, scan, cancel, review, preview, dismiss, assessment }: {
  session: Session; locale: PresentationLanguageTag; busy: boolean;
  scan: (scope: CleanScope) => void; cancel: (scope: CleanScope) => void;
  review: (scope: CleanScope) => void; preview: (scope: CleanScope) => void; dismiss: (scope: CleanScope) => void;
  assessment: { value: CleanAdvisoriesV1 | null; failed: boolean } | null;
}) {
  const [thresholdInvalid, setThresholdInvalid] = useState(false);
  const { scope, advisory } = session;
  return <>
    <header className="clean-workbench-heading"><div><p className="clean-eyebrow">DevSweep</p><h1>{message(locale, "clean.v2.heading")}</h1><p>{message(locale, "clean.v2.intro")}</p></div></header>
    <div className="clean-scope-grid">
      {(["global", "project"] as const).map(name => {
        const slot = session.slots[name];
        const state = slot.state;
        const scanning = state.activeScan !== null;
        const selected = selectedTotals(state);
        const rootControls = <ScanRoots locale={locale} roots={scope.options.roots} suggestions={scope.suggestions} disabled={busy || scope.loading || scope.saving} onChange={scope.changeRoots} />;
        const result = state.execution;
        const hasMoves = result?.outcomes.some(outcome => outcome.status.type === "succeeded" && outcome.action.type === "move_to_trash") ?? false;
        const commands = result?.outcomes.filter(outcome => outcome.status.type === "succeeded" && outcome.action.type === "command").length ?? 0;
        return <section key={name} className={`clean-scope-panel clean-scope-${name}`} aria-labelledby={`clean-${name}-heading`} aria-busy={scanning || state.pending !== null || state.phase === "executing"}>
          <header className="clean-scope-heading"><span className="clean-scope-icon"><DestinationGlyph name={name === "global" ? "package_cache" : "build_artifacts"} /></span><div><h2 id={`clean-${name}-heading`}>{message(locale, `clean.v2.${name}`)}</h2><p>{message(locale, `clean.v2.${name}_hint`)}</p></div></header>
          {name === "project" && <>
            {state.scan ? <details className="clean-folders"><summary>{message(locale, "clean.v2.folders_count", { count: String(scope.options.roots.length) })}</summary>{rootControls}</details> : rootControls}
            {scope.sessionOnly && <p role="status">{message(locale, "clean.v1.scope.session_only")}</p>}
            <details className="clean-threshold-settings"><summary>{message(locale, "clean.v2.advisory_settings", { threshold: String(advisory.threshold) })}</summary><form className="clean-threshold" onSubmit={event => {
              event.preventDefault(); const value = Number(new FormData(event.currentTarget).get("threshold"));
              setThresholdInvalid(!validThreshold(value)); if (validThreshold(value)) void advisory.save(value);
            }} noValidate>
              <div><label htmlFor="clean-threshold">{message(locale, "clean.v2.advisory_label")}</label><div className="clean-threshold-controls"><input key={advisory.threshold} id="clean-threshold" name="threshold" type="number" min="1" max="8388607" step="1" defaultValue={advisory.threshold} aria-invalid={thresholdInvalid} aria-describedby="clean-threshold-hint" disabled={advisory.saving} /><button type="submit" className="secondary-button" disabled={advisory.saving}>{message(locale, "clean.v2.advisory_save")}</button></div></div>
              <p id="clean-threshold-hint" className="secondary">{message(locale, "clean.v2.advisory_hint")}</p>
              {thresholdInvalid && <p role="alert">{message(locale, "clean.v2.advisory_invalid")}</p>}
            </form></details>
            {advisory.sessionOnly && <p role="status">{message(locale, "clean.v2.advisory_session")}</p>}
          </>}
          <div className="clean-scope-actions">
            <button type="button" className="primary-button" disabled={busy || (name === "project" && (scope.loading || scope.saving || scope.options.roots.length === 0))} onClick={() => scan(name)}>{message(locale, `clean.v2.${name}_scan`)}</button>
            {scanning && <button type="button" className="secondary-button" disabled={state.activeScan?.cancelRequested} onClick={() => cancel(name)}>{message(locale, "clean.v1.action.cancel")}</button>}
          </div>
          {busy && !scanning && state.pending === null && state.phase !== "executing" && <p className="secondary" role="status">{message(locale, "clean.v2.busy_other")}</p>}
          <div className="clean-scope-status" role="status">
            {scanning ? message(locale, "clean.v1.stage.scanning_headline") : state.pending ? message(locale, "clean.v2.pending") : state.stoppedPreview ? message(locale, state.stoppedPreview.kind === "failed" ? "clean.v1.stage.stopped_failed" : "clean.v1.scan.canceled") : slot.completedAt ? message(locale, "clean.v2.completed", { time: new Date(slot.completedAt).toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" }) }) : message(locale, "clean.v2.not_scanned")}
          </div>
          {state.error && <ErrorBanner error={state.error} onDismiss={() => dismiss(name)} />}
          {slot.stale && <p role="status" className="warning-text">{message(locale, "clean.v2.stale")}</p>}
          {state.scan && !scanning && !state.stoppedPreview && <div className="clean-report-summary">
            {result ? <><p>{message(locale, "clean.v1.result.counts", { succeeded: String(result.succeeded), skipped: String(result.skipped), failed: String(result.failed) })}</p><strong className="clean-capacity">{hasMoves ? formatTotals(movedTotals(result), locale) : String(commands)}</strong>{hasMoves && <p>{message(locale, "clean.v1.trash.moved")}</p>}{commands > 0 && <p>{message(locale, "clean.v1.result.commands", { count: String(commands) })} {message(locale, "clean.v1.result.commands_caption")}</p>}{!hasMoves && commands === 0 && <p>{message(locale, "clean.v1.result.no_changes")}</p>}</> : <>{name === "global" && <p className="clean-summary-label">{message(locale, "clean.v2.occupancy")}</p>}<strong className="clean-capacity">{name === "global" ? formatTotals(state.scan.health.totals, locale) : message(locale, "clean.v1.scan.complete", { count: String(state.scan.plan.targets.length) })}</strong><p className="secondary">{message(locale, "clean.v2.occupancy_note")}</p></>}
            {state.scan.plan.targets.length === 0 && <p>{message(locale, "clean.v2.empty")}</p>}
            {state.scan.plan.targets.length > 0 && state.selectedIds.size === 0 && !slot.stale && !result && <p>{message(locale, "clean.v1.stage.none_selected")}</p>}
            <div className="clean-scope-actions"><button type="button" className="secondary-button" disabled={busy} onClick={() => review(name)}>{message(locale, name === "global" ? "clean.v2.global_review" : "clean.v2.project_review")}</button>
              {state.selectedIds.size > 0 && !slot.stale && !result && <button type="button" className="primary-button" disabled={busy} onClick={() => preview(name)}>{message(locale, "clean.v1.action.clean_selected", { bytes: formatTotals(selected, locale) })}</button>}</div>
            {name === "project" && (assessment?.value ? <ProjectAdvisories locale={locale} value={assessment.value} /> : <p role="status">{message(locale, assessment?.failed ? "clean.v2.advisory_failed" : "clean.v2.advisory_pending")}</p>)}
            <ScanCoverage locale={locale} health={state.scan.health} />
          </div>}
          {(scanning || state.stoppedPreview) && <button type="button" className="secondary-button" onClick={() => review(name)}>{message(locale, "stage.v1.action.details")}</button>}
        </section>;
      })}
    </div>
  </>;
}
