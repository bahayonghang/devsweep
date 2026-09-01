import { useEffect, useRef } from "react";
import type { ScanOptions, ScanPhase } from "../api/types.gen";
import { SweepBody } from "../app-shell";
import { message, type PresentationLanguageTag } from "../i18n";
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
  locale?: PresentationLanguageTag;
  hero?: boolean;
  activeScan: ActiveScan | null; busy: boolean; options: ScanOptions;
  onOptions: (options: ScanOptions) => void; onScan: () => void; onCancel: () => void;
}) {
  const locale = props.locale ?? "en";
  const hero = props.hero ?? true;
  const scanning = props.activeScan !== null;
  const setScope = (key: "include_projects" | "include_global", checked: boolean) => props.onOptions({ ...props.options, [key]: checked });
  const requested: ScanPhase[] = [];
  if (props.options.include_projects) requested.push("projects");
  if (props.options.include_global) requested.push("global");
  const current = props.activeScan?.progress?.phase ?? requested[0] ?? "projects";
  const discovered = props.activeScan?.preview?.totals.target_count ?? 0;
  const progressMessage = props.activeScan?.cancelRequested
    ? message(locale, "clean.v1.action.cancel_scan")
    : props.activeScan?.progress?.message ?? message(locale, "clean.v1.action.scan");
  const projectsLabel = message(locale, "clean.v1.scope.projects");
  const globalLabel = message(locale, "clean.v1.scope.global");
  const liveRegion = useRef<HTMLSpanElement>(null);
  const liveTimer = useRef<number | null>(null);
  const pendingAnnouncement = useRef("");
  const lastAnnouncementAt = useRef(0);
  const lastPriorityKey = useRef("");
  useEffect(() => {
    const clearPending = () => {
      if (liveTimer.current !== null) {
        clearTimeout(liveTimer.current);
        liveTimer.current = null;
      }
    };
    if (!scanning) {
      clearPending();
      lastAnnouncementAt.current = 0;
      lastPriorityKey.current = "";
      if (liveRegion.current) liveRegion.current.textContent = "";
      return;
    }

    const phaseLabel = current === "projects" ? projectsLabel : globalLabel;
    const next = `${phaseLabel}. ${progressMessage}. ${discovered}`;
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
      liveTimer.current = window.setTimeout(() => {
        liveTimer.current = null;
        lastAnnouncementAt.current = Date.now();
        if (liveRegion.current) liveRegion.current.textContent = pendingAnnouncement.current;
      }, remaining);
    }
  }, [current, discovered, globalLabel, progressMessage, projectsLabel, props.activeScan?.cancelRequested, scanning]);
  useEffect(() => () => {
    if (liveTimer.current !== null) clearTimeout(liveTimer.current);
  }, []);
  const controls = <>
    <div className="scope-controls">
      <label><input type="checkbox" checked={props.options.include_projects} disabled={scanning || props.busy} onChange={(event) => setScope("include_projects", event.target.checked)} /> {projectsLabel}</label>
      <label><input type="checkbox" checked={props.options.include_global} disabled={scanning || props.busy} onChange={(event) => setScope("include_global", event.target.checked)} /> {globalLabel}</label>
    </div>
    <div className="scan-status" aria-busy={scanning || undefined}>
      {scanning ? <>
        <ol className="phase-rail" aria-label={message(locale, "clean.v1.action.scan")}>
          {requested.map((phase) => <li key={phase} data-state={phaseState(phase, requested, current)}>
            <span aria-hidden="true" />{phase === "projects" ? projectsLabel : globalLabel}
          </li>)}
        </ol>
        <progress aria-label={message(locale, "clean.v1.action.scan")}>{message(locale, "clean.v1.action.scan")}</progress>
        <div className="scan-announcement">
          <span className="scan-message">{progressMessage}</span>
          <span className="scan-count">{discovered}</span>
        </div>
        <span ref={liveRegion} className="sr-only" role="status" aria-live="polite" aria-atomic="true" />
      </> : <span className="ready-status">{message(locale, "clean.v1.status.ready")}</span>}
    </div>
    {scanning ? <button className="secondary-button fixed-action" onClick={props.onCancel} disabled={props.activeScan?.cancelRequested}>{props.activeScan?.cancelRequested ? message(locale, "clean.v1.action.cancel") : message(locale, "clean.v1.action.cancel_scan")}</button> : <button className="primary-button fixed-action" onClick={props.onScan} disabled={props.busy || (!props.options.include_projects && !props.options.include_global)}>{message(locale, "clean.v1.action.scan")}</button>}
  </>;
  return <section className={hero ? "clean-hero" : "scan-toolbar"} aria-label={message(locale, "clean.v1.action.scan")}>
    {hero && <SweepBody size="hero" />}
    {hero && !scanning && <p className="clean-hero-hint">{message(locale, "clean.v1.review.empty_hint")}</p>}
    {hero ? <div className="scan-toolbar clean-hero-controls">{controls}</div> : controls}
  </section>;
}
