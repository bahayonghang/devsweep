import { useEffect, useRef } from "react";
import type { AppState } from "../state/app-state";
import { isExecutable } from "../state/app-state";
import { allExecutableSelected, selectedTotals } from "../state/selectors";
import { formatTotals } from "../components/format";
import { TargetTable } from "../components/TargetTable";
import { SweepBody } from "../app-shell";
import { message, type PresentationLanguageTag } from "../i18n";
import type { TargetKind, UntrustedTarget } from "../api/types.gen";

function scopeCopy(locale: PresentationLanguageTag, scope: UntrustedTarget["scope"]["type"]): string {
  switch (scope) {
    case "project": return message(locale, "clean.v1.scope.projects");
    case "global": return message(locale, "clean.v1.scope.global");
  }
}

function groupTargets(targets: readonly UntrustedTarget[]): readonly {
  readonly kind: TargetKind;
  readonly scope: UntrustedTarget["scope"]["type"];
  readonly targets: UntrustedTarget[];
}[] {
  const kindOrder: TargetKind[] = [];
  const byKind = new Map<TargetKind, { project: UntrustedTarget[]; global: UntrustedTarget[] }>();
  for (const target of targets) {
    let bucket = byKind.get(target.kind);
    if (!bucket) {
      kindOrder.push(target.kind);
      bucket = { project: [], global: [] };
      byKind.set(target.kind, bucket);
    }
    bucket[target.scope.type].push(target);
  }
  return kindOrder.flatMap((kind) => {
    const bucket = byKind.get(kind);
    if (!bucket) return [];
    const groups: { kind: TargetKind; scope: UntrustedTarget["scope"]["type"]; targets: UntrustedTarget[] }[] = [];
    if (bucket.project.length > 0) groups.push({ kind, scope: "project", targets: bucket.project });
    if (bucket.global.length > 0) groups.push({ kind, scope: "global", targets: bucket.global });
    return groups;
  });
}

export function ReviewPage(props: {
  locale?: PresentationLanguageTag;
  state: AppState;
  onSelect: (id: string, selected: boolean) => void;
  onSelectAll: (selected: boolean) => void;
  onDryRun: () => void;
}) {
  const locale = props.locale ?? "en";
  const targets = props.state.scan?.plan.targets ?? [];
  const busy = props.state.pending !== null || props.state.phase === "executing";
  const groups = groupTargets(targets);
  const executable = targets.filter(isExecutable);
  const selectedExecutable = executable.filter((target) => props.state.selectedIds.has(target.id)).length;
  const selectAllRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (selectAllRef.current) selectAllRef.current.indeterminate = selectedExecutable > 0 && selectedExecutable < executable.length;
  }, [executable.length, selectedExecutable]);
  if (!props.state.scan) return null;
  if (targets.length === 0) {
    return <section className="clean-hero completed-empty">
      <SweepBody size="hero" />
      <h2>{message(locale, "clean.v1.scan.empty")}</h2>
      <p>{message(locale, "clean.v1.review.empty_hint")}</p>
    </section>;
  }
  return <section className="workspace">
    <header className="section-heading"><div><h2>{message(locale, "command.clean")}</h2><p>{message(locale, "clean.v1.scan.complete", { count: String(targets.length) })}</p></div><label className="select-all-control"><input ref={selectAllRef} type="checkbox" aria-label="Select all executable targets" checked={allExecutableSelected(props.state)} disabled={busy || executable.length === 0} onChange={(event) => props.onSelectAll(event.target.checked)} /> {message(locale, "clean.v1.summary.selected", { count: String(props.state.selectedIds.size) })}</label>{props.state.scan.health.diagnostics.length > 0 && <details className="diagnostics"><summary>{props.state.scan.health.diagnostics.length}</summary>{props.state.scan.health.diagnostics.map((item, index) => <p key={`${item.path}-${index}`}><strong>{item.stage}:</strong> {item.detail}</p>)}</details>}</header>
    <div className="preview-groups">
      {groups.map((group) => <section className="preview-group" key={`${group.kind}-${group.scope}`}>
        <header>
          <h3>{group.kind.replaceAll("_", " ")} <span>{group.targets.length}</span></h3>
          <p>{scopeCopy(locale, group.scope)}</p>
        </header>
        <TargetTable
          locale={locale}
          mode="review"
          targets={group.targets}
          selectedIds={props.state.selectedIds}
          allSelected={false}
          showSelectAll={false}
          disabled={busy}
          onSelect={props.onSelect}
          onSelectAll={() => undefined}
        />
      </section>)}
    </div>
    <footer className="action-bar"><div><span className="summary-label">{message(locale, "clean.v1.preview.estimated", { bytes: formatTotals(selectedTotals(props.state)) })}</span><span className="secondary">{message(locale, "clean.v1.summary.selected", { count: String(props.state.selectedIds.size) })}</span></div><button className="primary-button wide-action" disabled={props.state.selectedIds.size === 0 || busy} onClick={props.onDryRun}>{message(locale, "clean.v1.action.preview")}</button></footer>
  </section>;
}
