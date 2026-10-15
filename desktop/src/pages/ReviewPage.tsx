import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { ScanTotals, UntrustedTarget } from "../api/types.gen";
import { DestinationGlyph } from "../app-shell";
import { formatBytes, formatTotals } from "../components/format";
import { TargetTable } from "../components/TargetTable";
import { message, type PresentationLanguageTag } from "../i18n";
import { orderGroups } from "../modes/clean/impact";
import { kindLabel, scopeLabel } from "../modes/clean/labels";
import { ScanCoverage } from "../modes/clean/ScanCoverage";
import type { AppState } from "../state/app-state";
import { isSelectable } from "../state/app-state";
import { allExecutableSelected, selectedTotals } from "../state/selectors";

const NO_TARGETS: UntrustedTarget[] = [];

function SkippedTargets({ summary, children }: { summary: string; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return <details className="preview-group card skipped-group" onToggle={(event) => setOpen(event.currentTarget.open)}>
    <summary>{summary}</summary>
    {open && children}
  </details>;
}

function groupTotals(targets: readonly UntrustedTarget[]): ScanTotals {
  return targets.reduce<ScanTotals>((totals, target) => {
    if (target.size_complete) totals.verified_bytes += target.estimated_bytes;
    else if (target.estimated_bytes > 0) totals.partial_lower_bound_bytes += target.estimated_bytes;
    else totals.unknown_target_count += 1;
    return totals;
  }, { verified_bytes: 0, partial_lower_bound_bytes: 0, unknown_target_count: 0 });
}

function CapacityMeter({ locale, totals }: { locale: PresentationLanguageTag; totals: ScanTotals }) {
  const verified = totals.verified_bytes;
  const partial = totals.partial_lower_bound_bytes;
  const drawn = verified + partial;
  const separator = verified > 0 && partial > 0 ? 2 : 0;
  const usable = 100 - separator;
  const verifiedWidth = drawn === 0 ? 0 : (verified / drawn) * usable;
  const partialWidth = drawn === 0 ? 0 : (partial / drawn) * usable;
  const parts: string[] = [];
  if (verified > 0) parts.push(message(locale, "clean.v1.capacity.verified", { bytes: formatBytes(verified) }));
  if (partial > 0) parts.push(message(locale, "clean.v1.capacity.partial", { bytes: formatBytes(partial) }));
  if (totals.unknown_target_count > 0) parts.push(message(locale, "clean.v1.capacity.unknown"));
  const alternative = parts.join(" · ") || message(locale, "clean.v1.capacity.unknown");
  return <div className="capacity-meter">
    {drawn > 0 && <svg viewBox="0 0 100 8" preserveAspectRatio="none" aria-hidden="true">
      {verifiedWidth > 0 && <rect className="capacity-meter-verified" x={0} y={0} width={verifiedWidth} height={8} />}
      {partialWidth > 0 && <rect className="capacity-meter-partial" x={verifiedWidth + separator} y={0} width={partialWidth} height={8} />}
    </svg>}
    <p className="capacity-meter-text">{alternative}</p>
  </div>;
}

export function ReviewPage(props: {
  locale?: PresentationLanguageTag;
  state: AppState;
  /** Filters presentation only; summaries and selection retain the full report. */
  query?: string;
  disabled?: boolean;
  onSelect: (id: string, selected: boolean) => void;
  onSelectAll: (selected: boolean) => void;
  onDryRun: () => void;
  onSkip?: (id: string) => void;
  onRestore?: (id: string) => void;
  onProtect?: (target: UntrustedTarget) => void;
  onRescan?: (target: UntrustedTarget) => void;
  onAnalyze?: (target: UntrustedTarget) => void;
}) {
  const locale = props.locale ?? "en";
  const targets = props.state.scan?.plan.targets ?? NO_TARGETS;
  const busy = props.disabled || props.state.pending !== null || props.state.phase === "executing";
  const skippedIds = props.state.skippedIds;
  const query = props.query?.trim().toLowerCase() ?? "";
  const [pagination, setPagination] = useState({ query, page: 0 });
  const visibleTargets = useMemo(() => {
    const matching = query ? targets.filter((target) => `${target.path ?? ""} ${target.id} ${target.rule_id} ${target.scope.type === "project" ? target.scope.root : ""}`.toLowerCase().includes(query)) : targets;
    return orderGroups(matching).flatMap(group => group.targets);
  }, [targets, query]);
  const pageCount = Math.max(1, Math.ceil(visibleTargets.length / 100));
  const page = Math.min(pagination.query === query ? pagination.page : 0, pageCount - 1);
  const { groups, skipped } = useMemo(() => {
    const pageTargets = visibleTargets.slice(page * 100, (page + 1) * 100);
    return {
      groups: orderGroups(pageTargets.filter((target) => !skippedIds.has(target.id)))
        .map((group) => ({ ...group, totals: groupTotals(group.targets) })),
      skipped: pageTargets.filter((target) => skippedIds.has(target.id)),
    };
  }, [visibleTargets, skippedIds, page]);
  const executable = targets.filter((target) => isSelectable(props.state, target));
  const selectedExecutable = executable.filter((target) => props.state.selectedIds.has(target.id)).length;
  const totals = selectedTotals(props.state);
  const formatted = formatTotals(totals, locale);
  const selectAllRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (selectAllRef.current) selectAllRef.current.indeterminate = selectedExecutable > 0 && selectedExecutable < executable.length;
  }, [executable.length, selectedExecutable]);
  if (!props.state.scan) return null;
  if (targets.length === 0) {
    return <section className="card clean-stage">
      <p className="stage-eyebrow">{message(locale, "clean.v1.stage.eyebrow")}</p>
      <h2 className="stage-headline">{message(locale, props.state.scan.health.completeness === "partial" ? "clean.v1.coverage.partial_empty" : "clean.v1.scan.empty")}</h2>
      <p className="stage-lede">{message(locale, "clean.v1.review.empty_hint")}</p>
      <ScanCoverage locale={locale} health={props.state.scan.health} />
    </section>;
  }
  return <section className="workspace">
    <section className="card review-summary">
      <div className="capacity-total">
        <strong className="display-capacity">{formatted}</strong>
        <span>{message(locale, "clean.v1.preview.estimated", { bytes: formatted })}</span>
      </div>
      <p>{message(locale, "clean.v1.scan.complete", { count: String(targets.length) })}</p>
      <CapacityMeter locale={locale} totals={totals} />
      <label className="select-all-control">
        <input ref={selectAllRef} type="checkbox" aria-label={message(locale, "clean.v2.select_all")} checked={allExecutableSelected(props.state)} disabled={busy || executable.length === 0} onChange={(event) => props.onSelectAll(event.target.checked)} />
        {message(locale, "clean.v1.preview.selected", { count: String(props.state.selectedIds.size) })}
      </label>
      <ScanCoverage locale={locale} health={props.state.scan.health} />
      {props.state.protectedIds.size > 0 && <p className="rescan-hint" role="status">{message(locale, "clean.v1.review.rescan_hint")}</p>}
      <button className="primary-button wide-action" disabled={props.state.selectedIds.size === 0 || busy} onClick={props.onDryRun}>{message(locale, "clean.v1.action.preview")}</button>
    </section>
    <div className="preview-groups">
      {visibleTargets.length === 0 && <p role="status">{message(locale, "clean.v1.search.empty")}</p>}
      {pageCount > 1 && <div className="target-pagination">
        <p role="status">{message(locale, "clean.v1.list.page", { page: String(page + 1), pages: String(pageCount), start: String(page * 100 + 1), end: String(Math.min((page + 1) * 100, visibleTargets.length)), count: String(visibleTargets.length) })}</p>
        <p>{message(locale, "clean.v2.page_groups")}</p>
        <div>{(["first", "previous", "next", "last"] as const).map(action => <button key={action} type="button" className="secondary-button" disabled={action === "first" || action === "previous" ? page === 0 : page + 1 === pageCount} onClick={() => setPagination({ query, page: action === "first" ? 0 : action === "previous" ? page - 1 : action === "next" ? page + 1 : pageCount - 1 })}>{message(locale, `clean.v1.list.${action}`)}</button>)}</div>
      </div>}
      {groups.map((group) => <section className="preview-group card" key={JSON.stringify([group.owner, group.kind, group.scope])}>
        <header>
          <span className="glyph-tile"><DestinationGlyph name={group.kind} /></span>
          <h3>{group.owner} <span>{group.targets.length}</span></h3>
          <p>{kindLabel(locale, group.kind)} · {scopeLabel(locale, group.scope)}</p>
          <span className="group-subtotal">{formatTotals(group.totals, locale)}</span>
        </header>
        <TargetTable
          key={query}
          locale={locale}
          mode="review"
          targets={group.targets}
          selectedIds={props.state.selectedIds}
          protectedIds={props.state.protectedIds}
          allSelected={false}
          showSelectAll={false}
          disabled={busy}
          onSelect={props.onSelect}
          onSelectAll={() => undefined}
          onSkip={props.onSkip}
          onProtect={props.onProtect}
          onRescan={props.onRescan}
          onAnalyze={props.onAnalyze}
        />
      </section>)}
      {skipped.length > 0 && <SkippedTargets summary={message(locale, "clean.v1.review.skipped", { count: String(skipped.length) })}>
        <TargetTable
          key={query}
          locale={locale}
          mode="review"
          targets={skipped}
          selectedIds={props.state.selectedIds}
          protectedIds={props.state.protectedIds}
          skipped
          allSelected={false}
          showSelectAll={false}
          disabled={busy}
          onSelect={props.onSelect}
          onSelectAll={() => undefined}
          onRestore={props.onRestore}
        />
      </SkippedTargets>}
    </div>
  </section>;
}
