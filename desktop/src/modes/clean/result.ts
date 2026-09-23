import type { ExecutionReport, ScanTotals } from "../../api/types.gen";

/** Bytes this run moved to the Recycle Bin: succeeded trash moves only. */
export function movedTotals(report: ExecutionReport): ScanTotals {
  const totals: ScanTotals = {
    verified_bytes: 0,
    partial_lower_bound_bytes: 0,
    unknown_target_count: 0,
  };
  for (const outcome of report.outcomes) {
    if (
      outcome.status.type !== "succeeded" ||
      outcome.action.type !== "move_to_trash"
    )
      continue;
    const estimate = outcome.estimated_recoverable;
    switch (estimate.type) {
      case "verified":
        totals.verified_bytes += estimate.bytes;
        break;
      case "partial":
        totals.partial_lower_bound_bytes += estimate.lower_bound_bytes;
        break;
      case "unknown":
        totals.unknown_target_count += 1;
        break;
    }
  }
  return totals;
}
