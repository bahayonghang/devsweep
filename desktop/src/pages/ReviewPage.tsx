import { useEffect, useRef } from "react";
import type { ScanTotals, UntrustedTarget } from "../api/types.gen";
import { DestinationGlyph } from "../app-shell";
import { formatBytes, formatTotals } from "../components/format";
import { TargetTable } from "../components/TargetTable";
import { message, type PresentationLanguageTag } from "../i18n";
import { orderGroups } from "../modes/clean/impact";
import { kindLabel, scopeLabel } from "../modes/clean/labels";
import type { AppState } from "../state/app-state";
import { isSelectable } from "../state/app-state";
import { allExecutableSelected, selectedTotals } from "../state/selectors";

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
  onSelect: (id: string, selected: boolean) => void;
  onSelectAll: (selected: boolean) => void;
  onDryRun: () => void;
  onSkip?: (id: string) => void;
  onRestore?: (id: string) => void;
  onProtect?: (target: UntrustedTarget) => void;
}) {
  const locale = props.locale ?? "en";
  const targets = props.state.scan?.plan.targets ?? [];
  const busy = props.state.pending !== null || props.state.phase === "executing";
  const skippedIds = props.state.skippedIds;
  const groups = orderGroups(targets.filter((target) => !skippedIds.has(target.id)));
  const skipped = targets.filter((target) => skippedIds.has(target.id));
  const executable = targets.filter((target) => isSelectable(props.state, target));
  const selectedExecutable = executable.filter((target) => props.state.selectedIds.has(target.id)).length;
  const totals = selectedTotals(props.state);
  const formatted = formatTotals(totals);
  const selectAllRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (selectAllRef.current) selectAllRef.current.indeterminate = selectedExecutable > 0 && selectedExecutable < executable.length;
  }, [executable.length, selectedExecutable]);
  if (!props.state.scan) return null;
  if (targets.length === 0) {
    return <section className="card clean-stage">
      <p className="stage-eyebrow">{message(locale, "clean.v1.stage.eyebrow")}</p>
      <h2 className="stage-headline">{message(locale, "clean.v1.scan.empty")}</h2>
      <p className="stage-lede">{message(locale, "clean.v1.review.empty_hint")}</p>
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
        <input ref={selectAllRef} type="checkbox" aria-label="Select all executable targets" checked={allExecutableSelected(props.state)} disabled={busy || executable.length === 0} onChange={(event) => props.onSelectAll(event.target.checked)} />
        {message(locale, "clean.v1.preview.selected", { count: String(props.state.selectedIds.size) })}
      </label>
      {props.state.scan.health.diagnostics.length > 0 && <details className="diagnostics"><summary>{props.state.scan.health.diagnostics.length}</summary>{props.state.scan.health.diagnostics.map((item, index) => <p key={`${item.path}-${index}`}><strong>{item.stage}:</strong> {item.detail}</p>)}</details>}
      {props.state.protectedIds.size > 0 && <p className="rescan-hint" role="status">{message(locale, "clean.v1.review.rescan_hint")}</p>}
      <button className="primary-button wide-action" disabled={props.state.selectedIds.size === 0 || busy} onClick={props.onDryRun}>{message(locale, "clean.v1.action.preview")}</button>
    </section>
    <div className="preview-groups">
      {groups.map((group) => <section className="preview-group card" key={`${group.kind}-${group.scope}`}>
        <header>
          <span className="glyph-tile"><DestinationGlyph name={group.kind} /></span>
          <h3>{kindLabel(locale, group.kind)} <span>{group.targets.length}</span></h3>
          <p>{scopeLabel(locale, group.scope)}</p>
          <span className="group-subtotal">{formatTotals(groupTotals(group.targets))}</span>
        </header>
        <TargetTable
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
        />
      </section>)}
      {skipped.length > 0 && <details className="preview-group card skipped-group">
        <summary>{message(locale, "clean.v1.review.skipped", { count: String(skipped.length) })}</summary>
        <TargetTable
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
      </details>}
    </div>
  </section>;
}
