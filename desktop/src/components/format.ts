import type { CapacityEstimate, Evidence, ScanTotals } from "../api/types.gen";
import { formatBinaryBytes, message, type PresentationLanguageTag } from "../i18n";

export function formatBytes(bytes: number): string {
  return formatBinaryBytes(bytes);
}

export function formatTotals(totals: ScanTotals, locale: PresentationLanguageTag = "en"): string {
  const parts: string[] = [];
  if (totals.verified_bytes > 0) parts.push(formatBytes(totals.verified_bytes));
  if (totals.partial_lower_bound_bytes > 0) parts.push(message(locale, "clean.v2.total_partial", { bytes: formatBytes(totals.partial_lower_bound_bytes) }));
  if (totals.unknown_target_count > 0) parts.push(message(locale, "clean.v2.total_unknown", { count: String(totals.unknown_target_count) }));
  return parts.length > 0 ? parts.join(" + ") : "0 B";
}

export function formatCapacity(capacity: CapacityEstimate, locale: PresentationLanguageTag = "en"): string {
  switch (capacity.type) {
    case "verified": return formatBytes(capacity.bytes);
    case "partial": return message(locale, "clean.v2.capacity_partial", { bytes: formatBytes(capacity.lower_bound_bytes) });
    case "unknown": return message(locale, "clean.v2.capacity_unknown");
  }
}

export function formatEvidence(evidence: Evidence): string {
  switch (evidence.type) {
    case "marker_file": return `Marker file: ${evidence.path}`;
    case "known_cache_dir": return `${evidence.source}: ${evidence.path}`;
    case "official_command": return `Official probe: ${evidence.command}`;
    case "rule_matched": return `Rule: ${evidence.rule_id}`;
    case "user_configured": return "User configured";
  }
}
