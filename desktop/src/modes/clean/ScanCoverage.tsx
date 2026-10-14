import type { ScanCoverageStatus, ScanHealth } from "../../api/types.gen";
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

/** Coverage is evidence, never a target total or cleanup selection. */
export function ScanCoverage({ health, locale }: { health: ScanHealth; locale: PresentationLanguageTag }) {
  const coverage = health.coverage ?? [];
  return <section className="scan-coverage" aria-label={message(locale, "clean.v1.coverage.title")}>
    <p role="status" className={health.completeness === "partial" ? "warning-text" : undefined}>
      {message(locale, health.completeness === "partial" ? "clean.v1.coverage.partial" : "clean.v1.coverage.summary")}
    </p>
    {coverage.length === 0 ? <p>{message(locale, "clean.v1.coverage.not_recorded")}</p> : <details open={health.completeness === "partial"}>
      <summary>{message(locale, "clean.v1.coverage.title")}</summary>
      <ul>{coverage.map((unit) => <li key={`${unit.scope}:${unit.id}`}>
        <strong>{unit.path ?? unit.id}</strong>
        <span>{message(locale, statusKeys[unit.status])}</span>
        <p>{unit.detail}</p>
      </li>)}</ul>
    </details>}
    {health.diagnostics.length > 0 && <details open={health.completeness === "partial"}>
      <summary>{message(locale, "clean.v1.coverage.diagnostics")}</summary>
      <ul>{health.diagnostics.map((item, index) => <li key={`${item.path}:${index}`}>
        <strong>{item.path}</strong><p>{item.detail}</p>
      </li>)}</ul>
    </details>}
  </section>;
}
