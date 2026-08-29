import type { DesktopScanProgress, Ecosystem, ScanPreviewSnapshot, ScanPreviewTarget } from "../api/types.gen";
import { TargetTable } from "../components/TargetTable";
import { formatBytes } from "../components/format";

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

function capacitySummary(preview: ScanPreviewSnapshot): string {
  const parts: string[] = [];
  if (preview.totals.verified_bytes > 0) parts.push(`${formatBytes(preview.totals.verified_bytes)} verified`);
  if (preview.totals.partial_lower_bound_bytes > 0) parts.push(`at least ${formatBytes(preview.totals.partial_lower_bound_bytes)} partial`);
  if (preview.totals.unknown_target_count > 0) parts.push(`${preview.totals.unknown_target_count} unknown`);
  return parts.join(" · ") || "Capacity not available yet";
}

function statusCopy(status: PreviewStatus): string {
  if (status === "canceled") return "Scan canceled. These partial observations remain read-only.";
  if (status === "failed") return "Scan stopped after an error. These partial observations remain read-only.";
  return "Scan in progress. Results are incomplete and cannot be selected until the final report is ready.";
}

export function ScanPreviewPage(props: {
  status: PreviewStatus;
  progress: DesktopScanProgress | null;
  preview: ScanPreviewSnapshot | null;
  canReturnToReport: boolean;
  onReturnToReport: () => void;
}) {
  const targets = props.preview?.targets ?? [];
  const projects = targets.filter((target) => target.scope.type === "project");
  const globals = targets.filter((target) => target.scope.type === "global");
  const active = props.status === "active";
  return <section className="workspace preview-workspace" aria-busy={active || undefined}>
    <header className="section-heading preview-heading">
      <div><h2>Discovered so far</h2><p>{statusCopy(props.status)}</p></div>
      <div className="preview-heading-actions">
        {props.preview && <div className="preview-total" aria-label="Observed capacity summary"><strong>{props.preview.totals.target_count} {props.preview.totals.target_count === 1 ? "target" : "targets"}</strong><span>{capacitySummary(props.preview)}</span></div>}
        {!active && props.canReturnToReport && <button className="secondary-button" onClick={props.onReturnToReport}>Return to previous report</button>}
      </div>
    </header>
    {targets.length === 0 ? <div className="active-empty">
      <h3>{active ? "Discovering cleanup targets…" : "No targets were discovered before the scan stopped"}</h3>
      <p>{active ? props.progress?.message ?? "Preparing the requested scan phases." : "Run another scan to produce a completed cleanup report."}</p>
    </div> : <div className="preview-groups">
      {projects.length > 0 && <section className="preview-group" aria-labelledby="preview-projects">
        <header><h3 id="preview-projects">Projects <span>{projects.length}</span></h3><p>{ecosystemCounts(projects)}</p></header>
        <TargetTable mode="preview" targets={projects} />
      </section>}
      {globals.length > 0 && <section className="preview-group" aria-labelledby="preview-globals">
        <header><h3 id="preview-globals">Global caches <span>{globals.length}</span></h3><p>{ecosystemCounts(globals)}</p></header>
        <TargetTable mode="preview" targets={globals} />
      </section>}
    </div>}
  </section>;
}
