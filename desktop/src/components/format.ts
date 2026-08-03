import type { CapacityEstimate, Evidence, ScanTotals } from "../api/types.gen";

export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) { value /= 1024; index += 1; }
  return `${value >= 10 || index === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[index]}`;
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
