import type { CapacityEstimate, Evidence, ScanTotals } from "../api/types.gen";
import { formatBinaryBytes } from "../i18n";

export function formatBytes(bytes: number): string {
  return formatBinaryBytes(bytes);
}

export function formatTotals(totals: ScanTotals): string {
  const parts: string[] = [];
  if (totals.verified_bytes > 0) parts.push(formatBytes(totals.verified_bytes));
  if (totals.partial_lower_bound_bytes > 0) parts.push(`at least ${formatBytes(totals.partial_lower_bound_bytes)}`);
  if (totals.unknown_target_count > 0) parts.push(`${totals.unknown_target_count} unknown`);
  return parts.length > 0 ? parts.join(" + ") : "0 B";
}

export function formatCapacity(capacity: CapacityEstimate): string {
  switch (capacity.type) {
    case "verified": return formatBytes(capacity.bytes);
    case "partial": return `At least ${formatBytes(capacity.lower_bound_bytes)}`;
    case "unknown": return "Unknown";
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
