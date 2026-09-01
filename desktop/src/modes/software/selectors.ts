import type { SoftwareEntryV1, SoftwareIdentity, SoftwareSizeEvidence } from "../../api/types.gen";
import type { SoftwareSort, SoftwareState } from "./state";

export interface SoftwareSizeSummary {
  readonly estimatedBytes: number;
  readonly measuredBytes: number;
  readonly lowerBoundBytes: number;
  readonly unknownCount: number;
}

function compareCodeUnits(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

export function softwareIdentityText(identity: SoftwareIdentity): string {
  switch (identity.source) {
    case "arp": return `ARP:${identity.hive}:${identity.view}:${identity.subkey}`;
    case "msi": return `MSI:${identity.context}:${identity.product_code}`;
    case "msix": return `MSIX:${identity.package_full_name}`;
  }
}

export function softwareSource(entry: SoftwareEntryV1): "arp" | "msi" | "msix" {
  return entry.identity.source;
}

export function softwareSizeSortValue(size: SoftwareSizeEvidence): number {
  if (size.state === "available") return size.value_bytes;
  if (size.state === "partial") return size.lower_bound_bytes;
  return -1;
}

export function selectSoftwareEntries(
  entries: readonly SoftwareEntryV1[],
  query: string,
  sort: SoftwareSort,
): readonly SoftwareEntryV1[] {
  const normalized = query.trim().toLowerCase();
  const filtered = normalized.length === 0 ? entries : entries.filter((entry) => [
    entry.display_name ?? "",
    entry.publisher ?? "",
    entry.id,
    softwareIdentityText(entry.identity),
    entry.eligibility.reason,
  ].some((value) => value.toLowerCase().includes(normalized)));
  return [...filtered].sort((left, right) => {
    let result = 0;
    if (sort === "size") result = softwareSizeSortValue(right.size) - softwareSizeSortValue(left.size);
    else if (sort === "eligibility") result = compareCodeUnits(left.eligibility.reason, right.eligibility.reason);
    else if (sort === "source") result = compareCodeUnits(softwareSource(left), softwareSource(right));
    else result = compareCodeUnits((left.display_name ?? left.id).toLowerCase(), (right.display_name ?? right.id).toLowerCase());
    return result || compareCodeUnits(left.id, right.id);
  });
}

export function selectedSoftwareEntries(state: SoftwareState): readonly SoftwareEntryV1[] {
  return state.inventory?.entries.filter((entry) => state.selectedIds.has(entry.id)) ?? [];
}

export function summarizeSoftwareSize(entries: readonly SoftwareEntryV1[]): SoftwareSizeSummary {
  return entries.reduce<SoftwareSizeSummary>((summary, entry) => {
    if (entry.size.state === "unknown") return { ...summary, unknownCount: summary.unknownCount + 1 };
    if (entry.size.state === "partial") return { ...summary, lowerBoundBytes: summary.lowerBoundBytes + entry.size.lower_bound_bytes };
    if (entry.size.basis === "reported_estimate") return { ...summary, estimatedBytes: summary.estimatedBytes + entry.size.value_bytes };
    return { ...summary, measuredBytes: summary.measuredBytes + entry.size.value_bytes };
  }, { estimatedBytes: 0, measuredBytes: 0, lowerBoundBytes: 0, unknownCount: 0 });
}
