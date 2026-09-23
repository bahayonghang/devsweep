import type { ScanTotals, UntrustedTarget } from "../api/types.gen";
import type { AppState } from "./app-state";
import { isSelectable } from "./app-state";

export function selectedTargets(state: AppState): UntrustedTarget[] {
  return (state.scan?.plan.targets ?? []).filter((target) => state.selectedIds.has(target.id));
}

export function selectedTotals(state: AppState): ScanTotals {
  return selectedTargets(state).reduce<ScanTotals>((totals, target) => {
    if (target.size_complete) totals.verified_bytes += target.estimated_bytes;
    else if (target.estimated_bytes > 0) totals.partial_lower_bound_bytes += target.estimated_bytes;
    else totals.unknown_target_count += 1;
    return totals;
  }, { verified_bytes: 0, partial_lower_bound_bytes: 0, unknown_target_count: 0 });
}

export function allExecutableSelected(state: AppState): boolean {
  const executable = (state.scan?.plan.targets ?? []).filter((target) => isSelectable(state, target));
  return executable.length > 0 && executable.every((target) => state.selectedIds.has(target.id));
}

export function hasIrreversibleSelection(state: AppState): boolean {
  return selectedTargets(state).some((target) => !target.reversible);
}
