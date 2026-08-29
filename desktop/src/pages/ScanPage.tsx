import { useEffect, useRef } from "react";
import type { ScanOptions, ScanPhase } from "../api/types.gen";
import type { ActiveScan } from "../state/app-state";

type PhaseState = "pending" | "current" | "complete";
const LIVE_ANNOUNCEMENT_INTERVAL_MS = 1_000;

function phaseState(phase: ScanPhase, requested: ScanPhase[], current: ScanPhase): PhaseState {
  const currentIndex = requested.indexOf(current);
  const phaseIndex = requested.indexOf(phase);
  if (phaseIndex < currentIndex) return "complete";
  return phaseIndex === currentIndex ? "current" : "pending";
}

export function ScanPage(props: {
  activeScan: ActiveScan | null; busy: boolean; options: ScanOptions;
  onOptions: (options: ScanOptions) => void; onScan: () => void; onCancel: () => void;
}) {
  const scanning = props.activeScan !== null;
  const setScope = (key: "include_projects" | "include_global", checked: boolean) => props.onOptions({ ...props.options, [key]: checked });
  const requested: ScanPhase[] = [];
  if (props.options.include_projects) requested.push("projects");
  if (props.options.include_global) requested.push("global");
  const current = props.activeScan?.progress?.phase ?? requested[0] ?? "projects";
  const discovered = props.activeScan?.preview?.totals.target_count ?? 0;
  const message = props.activeScan?.cancelRequested
    ? "Cancel requested; finishing the current safe boundary"
    : props.activeScan?.progress?.message ?? "Starting scan";
  const liveRegion = useRef<HTMLSpanElement>(null);
  const liveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pendingAnnouncement = useRef("");
  const lastAnnouncementAt = useRef(0);
  const lastPriorityKey = useRef("");
  useEffect(() => {
    const clearPending = () => {
      if (liveTimer.current !== null) clearTimeout(liveTimer.current);
      liveTimer.current = null;
    };
    if (!scanning) {
      clearPending();
      lastAnnouncementAt.current = 0;
      lastPriorityKey.current = "";
      if (liveRegion.current) liveRegion.current.textContent = "";
      return;
    }

    const phaseLabel = current === "projects" ? "Projects" : "Global caches";
    const next = `${phaseLabel}. ${message}. ${discovered} discovered so far.`;
    const priorityKey = `${current}:${props.activeScan?.cancelRequested === true}`;
    const now = Date.now();
    if (priorityKey !== lastPriorityKey.current || now - lastAnnouncementAt.current >= LIVE_ANNOUNCEMENT_INTERVAL_MS) {
      clearPending();
      lastPriorityKey.current = priorityKey;
      lastAnnouncementAt.current = now;
      if (liveRegion.current) liveRegion.current.textContent = next;
      return;
    }

    pendingAnnouncement.current = next;
    if (liveTimer.current === null) {
      const remaining = LIVE_ANNOUNCEMENT_INTERVAL_MS - (now - lastAnnouncementAt.current);
      liveTimer.current = setTimeout(() => {
        liveTimer.current = null;
        lastAnnouncementAt.current = Date.now();
        if (liveRegion.current) liveRegion.current.textContent = pendingAnnouncement.current;
      }, remaining);
    }
  }, [current, discovered, message, props.activeScan?.cancelRequested, scanning]);
  useEffect(() => () => {
    if (liveTimer.current !== null) clearTimeout(liveTimer.current);
  }, []);
  return <section className="scan-toolbar" aria-label="Scan controls">
    <div className="scope-controls">
      <label><input type="checkbox" checked={props.options.include_projects} disabled={scanning || props.busy} onChange={(event) => setScope("include_projects", event.target.checked)} /> Projects</label>
      <label><input type="checkbox" checked={props.options.include_global} disabled={scanning || props.busy} onChange={(event) => setScope("include_global", event.target.checked)} /> Global caches</label>
    </div>
    <div className="scan-status" aria-busy={scanning || undefined}>
      {scanning ? <>
        <ol className="phase-rail" aria-label="Requested scan phases">
          {requested.map((phase) => <li key={phase} data-state={phaseState(phase, requested, current)}>
            <span aria-hidden="true" />{phase === "projects" ? "Projects" : "Global caches"}
          </li>)}
        </ol>
        <progress aria-label="Scan progress">Scanning</progress>
        <div className="scan-announcement">
          <span className="scan-message">{message}</span>
          <span className="scan-count">{discovered} discovered so far</span>
        </div>
        <span ref={liveRegion} className="sr-only" role="status" aria-live="polite" aria-atomic="true" />
      </> : <span className="ready-status">Ready</span>}
    </div>
    {scanning ? <button className="secondary-button fixed-action" onClick={props.onCancel} disabled={props.activeScan?.cancelRequested}>{props.activeScan?.cancelRequested ? "Canceling…" : "Cancel scan"}</button> : <button className="primary-button fixed-action" onClick={props.onScan} disabled={props.busy || (!props.options.include_projects && !props.options.include_global)}>Scan</button>}
  </section>;
}
