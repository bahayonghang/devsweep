import type { DesktopScanProgress, Ecosystem, ScanPreviewSnapshot, ScanPreviewTarget } from "../api/types.gen";
import { TargetTable } from "../components/TargetTable";
import { formatBytes } from "../components/format";
import { message, type PresentationLanguageTag } from "../i18n";

type PreviewStatus = "active" | "canceled" | "failed";

const ecosystemOrder: Ecosystem[] = ["rust", "node", "python", "generic", "docker"];

function ecosystemCounts(targets: ScanPreviewTarget[]): string {
  const counts = new Map<Ecosystem, number>();
  for (const target of targets) counts.set(target.ecosystem, (counts.get(target.ecosystem) ?? 0) + 1);
  return ecosystemOrder
    .filter((ecosystem) => counts.has(ecosystem))
    .map((ecosystem) => `${ecosystem[0].toUpperCase()}${ecosystem.slice(1)} ${counts.get(ecosystem)}`)
    .join(" · ");
}

function capacitySummary(locale: PresentationLanguageTag, preview: ScanPreviewSnapshot): string {
  const parts: string[] = [];
  if (preview.totals.verified_bytes > 0) parts.push(message(locale, "clean.v1.capacity.verified", { bytes: formatBytes(preview.totals.verified_bytes) }));
  if (preview.totals.partial_lower_bound_bytes > 0) parts.push(message(locale, "clean.v1.capacity.partial", { bytes: formatBytes(preview.totals.partial_lower_bound_bytes) }));
  if (preview.totals.unknown_target_count > 0) parts.push(message(locale, "clean.v1.capacity.unknown"));
  return parts.join(" · ") || message(locale, "clean.v1.capacity.unknown");
}

export function ScanPreviewPage(props: {
  locale?: PresentationLanguageTag;
  status: PreviewStatus;
  progress: DesktopScanProgress | null;
  preview: ScanPreviewSnapshot | null;
  canReturnToReport: boolean;
  onReturnToReport: () => void;
}) {
  const locale = props.locale ?? "en";
  const targets = props.preview?.targets ?? [];
  const projects = targets.filter((target) => target.scope.type === "project");
  const globals = targets.filter((target) => target.scope.type === "global");
  const active = props.status === "active";
  const statusText = props.status === "canceled" || props.status === "failed"
    ? message(locale, "clean.v1.scan.canceled")
    : message(locale, "clean.v1.scan.partial");
  return <section className="workspace preview-workspace" aria-busy={active || undefined}>
    <header className="section-heading preview-heading">
      <div><h2>{message(locale, "clean.v1.action.scan")}</h2><p>{statusText}</p></div>
      <div className="preview-heading-actions">
        {props.preview && <div className="preview-total" aria-label={message(locale, "clean.v1.preview.estimated", { bytes: "" })}><strong>{message(locale, "clean.v1.scan.complete", { count: String(props.preview.totals.target_count) })}</strong><span>{capacitySummary(locale, props.preview)}</span></div>}
        {!active && props.canReturnToReport && <button className="secondary-button" onClick={props.onReturnToReport}>{message(locale, "clean.v1.action.cancel")}</button>}
      </div>
    </header>
    {targets.length === 0 ? <div className="active-empty">
      <h3>{active ? message(locale, "clean.v1.action.scan") : message(locale, "clean.v1.scan.empty")}</h3>
      <p>{active ? props.progress?.message ?? message(locale, "clean.v1.status.ready") : message(locale, "clean.v1.review.empty_hint")}</p>
    </div> : <div className="preview-groups">
      {projects.length > 0 && <section className="preview-group" aria-labelledby="preview-projects">
        <header><h3 id="preview-projects">{message(locale, "clean.v1.scope.projects")} <span>{projects.length}</span></h3><p>{ecosystemCounts(projects)}</p></header>
        <TargetTable locale={locale} mode="preview" targets={projects} />
      </section>}
      {globals.length > 0 && <section className="preview-group" aria-labelledby="preview-globals">
        <header><h3 id="preview-globals">{message(locale, "clean.v1.scope.global")} <span>{globals.length}</span></h3><p>{ecosystemCounts(globals)}</p></header>
        <TargetTable locale={locale} mode="preview" targets={globals} />
      </section>}
    </div>}
  </section>;
}
