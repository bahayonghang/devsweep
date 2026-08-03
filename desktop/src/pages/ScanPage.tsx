import type { ScanOptions, ScanProgress } from "../api/types.gen";

export function ScanPage(props: {
  scanning: boolean; busy: boolean; cancelRequested: boolean; progress: ScanProgress | null; options: ScanOptions;
  onOptions: (options: ScanOptions) => void; onScan: () => void; onCancel: () => void;
}) {
  const setScope = (key: "include_projects" | "include_global", checked: boolean) => props.onOptions({ ...props.options, [key]: checked });
  return <section className="scan-toolbar" aria-label="Scan controls">
    <div className="scope-controls">
      <label><input type="checkbox" checked={props.options.include_projects} disabled={props.scanning || props.busy} onChange={(event) => setScope("include_projects", event.target.checked)} /> Projects</label>
      <label><input type="checkbox" checked={props.options.include_global} disabled={props.scanning || props.busy} onChange={(event) => setScope("include_global", event.target.checked)} /> Global caches</label>
    </div>
    <div className="scan-status" aria-live="polite">
      {props.scanning && <span className="spinner" aria-hidden="true" />}
      <span>{props.scanning ? `${props.progress?.phase === "global" ? "Global" : "Projects"}: ${props.progress?.message ?? "Starting scan"}` : "Ready"}</span>
    </div>
    {props.scanning ? <button className="secondary-button fixed-action" onClick={props.onCancel} disabled={props.cancelRequested}>{props.cancelRequested ? "Canceling…" : "Cancel scan"}</button> : <button className="primary-button fixed-action" onClick={props.onScan} disabled={props.busy || (!props.options.include_projects && !props.options.include_global)}>Scan</button>}
  </section>;
}
