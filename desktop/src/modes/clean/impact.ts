import type { TargetKind, UntrustedTarget } from "../../api/types.gen";

/** Lower rank shows first. Regenerable caches lead; restores that need a rebuild follow. */
export const IMPACT_RANK: Record<TargetKind, number> = {
  package_cache: 0,
  tool_cache: 0,
  test_cache: 0,
  build_artifacts: 1,
  dependency_directory: 2,
  virtual_env: 2,
};

type Sized = Pick<UntrustedTarget, "estimated_bytes" | "size_complete">;
type Scope = UntrustedTarget["scope"]["type"];

export interface TargetGroup {
  readonly kind: TargetKind;
  readonly scope: Scope;
  readonly targets: UntrustedTarget[];
}

interface SizeKey {
  readonly verified: number;
  readonly lowerBound: number;
}

function sizeKey(targets: readonly Sized[]): SizeKey {
  let verified = 0;
  let lowerBound = 0;
  for (const target of targets) {
    if (target.size_complete) verified += target.estimated_bytes;
    else lowerBound += target.estimated_bytes;
  }
  return { verified, lowerBound };
}

/** Verified bytes descending, then lower-bound bytes descending. Unknown sizes have zero of both. */
function compareSize(left: SizeKey, right: SizeKey): number {
  return right.verified - left.verified || right.lowerBound - left.lowerBound;
}

/** Row order inside one group: verified sizes first, then lower bound, then unknown. */
export function compareTargets(
  left: UntrustedTarget,
  right: UntrustedTarget,
): number {
  if (left.size_complete !== right.size_complete)
    return left.size_complete ? -1 : 1;
  return (
    right.estimated_bytes - left.estimated_bytes ||
    left.id.localeCompare(right.id)
  );
}

export function orderGroups(
  targets: readonly UntrustedTarget[],
): TargetGroup[] {
  const byKey = new Map<string, TargetGroup>();
  for (const target of targets) {
    const key = `${target.kind}:${target.scope.type}`;
    let group = byKey.get(key);
    if (!group) {
      group = { kind: target.kind, scope: target.scope.type, targets: [] };
      byKey.set(key, group);
    }
    group.targets.push(target);
  }
  const groups = [...byKey.values()];
  for (const group of groups) group.targets.sort(compareTargets);
  return groups.sort(
    (left, right) =>
      IMPACT_RANK[left.kind] - IMPACT_RANK[right.kind] ||
      compareSize(sizeKey(left.targets), sizeKey(right.targets)) ||
      left.kind.localeCompare(right.kind) ||
      left.scope.localeCompare(right.scope),
  );
}
