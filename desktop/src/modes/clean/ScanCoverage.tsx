import type { ScanCoverageStatus, ScanHealth } from "../../api/types.gen";
import { useState } from "react";
import { message, type MessageKey, type PresentationLanguageTag } from "../../i18n";

const statusKeys: Record<ScanCoverageStatus, MessageKey> = {
  complete: "clean.v1.coverage.complete", empty: "clean.v1.coverage.empty",
  absent: "clean.v1.coverage.absent", unsupported: "clean.v1.coverage.unsupported",
  skipped: "clean.v1.coverage.skipped", failed: "clean.v1.coverage.failed",
  canceled: "clean.v1.coverage.canceled", timeout: "clean.v1.coverage.timeout",
  output_truncated: "clean.v1.coverage.output_truncated", budget_limited: "clean.v1.coverage.budget_limited",
  protected: "clean.v1.coverage.protected", busy: "clean.v1.coverage.busy",
  ownership_unknown: "clean.v1.coverage.ownership_unknown",
};

function EvidenceList({ entries, label, initialOpen, locale }: { entries: { id: string; heading: string; detail: string; status?: string }[]; label: string; initialOpen: boolean; locale: PresentationLanguageTag }) {
  const [open, setOpen] = useState(initialOpen);
  const [requestedPage, setPage] = useState(0);
  const pages = Math.max(1, Math.ceil(entries.length / 100)), page = Math.min(requestedPage, pages - 1);
  return <details open={open} onToggle={event => setOpen(event.currentTarget.open)}><summary>{label}</summary>{open && <>
    <ul>{entries.slice(page * 100, (page + 1) * 100).map(entry => <li key={entry.id}><strong>{entry.heading}</strong>{entry.status && <span>{entry.status}</span>}<p>{entry.detail}</p></li>)}</ul>
    {pages > 1 && <div className="target-pagination"><span>{page + 1} / {pages}</span><button type="button" disabled={page === 0} onClick={() => setPage(page - 1)}>{message(locale, "clean.v1.list.previous")}</button><button type="button" disabled={page + 1 === pages} onClick={() => setPage(page + 1)}>{message(locale, "clean.v1.list.next")}</button></div>}
  </>}</details>;
}

/** Coverage is evidence, never a target total or cleanup selection. */
export function ScanCoverage({ health, locale }: { health: ScanHealth; locale: PresentationLanguageTag }) {
  const coverage = health.coverage ?? [];
  return <section className="scan-coverage" aria-label={message(locale, "clean.v1.coverage.title")}>
    <p role="status" className={health.completeness === "partial" ? "warning-text" : undefined}>
      {message(locale, health.completeness === "partial" ? "clean.v1.coverage.partial" : "clean.v1.coverage.summary")}
    </p>
    {coverage.length === 0 ? <p>{message(locale, "clean.v1.coverage.not_recorded")}</p> : <EvidenceList locale={locale} initialOpen={health.completeness === "partial"} label={message(locale, "clean.v1.coverage.title")} entries={coverage.map(unit => ({ id: `${unit.scope}:${unit.id}`, heading: unit.path ?? unit.id, detail: unit.detail, status: message(locale, statusKeys[unit.status]) }))} />}
    {health.diagnostics.length > 0 && <EvidenceList locale={locale} initialOpen={health.completeness === "partial"} label={message(locale, "clean.v1.coverage.diagnostics")} entries={health.diagnostics.map((item, index) => ({ id: `${item.path}:${index}`, heading: item.path, detail: item.detail }))} />}
  </section>;
}
