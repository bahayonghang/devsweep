import type {
  ActionKind,
  AnalyzeNodeV1,
  AnalyzeSnapshotV1,
  AnalyzeTrashItemV1,
  AnalyzeTrashPreviewV1,
  AnalyzeTrashRefusalV1,
  AnalyzeTrashReportV1,
  AnalyzeWarningV1,
  CapacityEstimate,
  CleanupIntent,
  CommandError,
  DesktopAnalyzeProgress,
  DesktopAnalyzeResult,
  DesktopScanProgress,
  DesktopScanResult,
  DesktopSoftwareAuditResult,
  DesktopSoftwareInventoryResult,
  DesktopSoftwareLeftoversPreviewResult,
  DesktopSoftwareLeftoversResult,
  DesktopSoftwarePreviewResult,
  DesktopSoftwareUninstallResult,
  DesktopSoftwareUpdatesResult,
  DesktopOptimizeAuditResult,
  DesktopOptimizeListResult,
  DesktopOptimizePreviewResult,
  DesktopOptimizeRunResult,
  DesktopStatusLiveResult,
  DesktopStatusSnapshotResult,
  DryRunOutcome,
  Evidence,
  ExecutionReport,
  MaintenanceActionClass,
  MaintenanceActionOutcomeV1,
  MaintenanceCatalogueEntryV1,
  MaintenanceExecutionOutcome,
  MaintenanceExecutionReportV1,
  MaintenancePlanV1,
  MaintenancePreviewV1,
  OptimizeAuditRecordV1,
  OptimizeAuditTransition,
  OutcomeStatus,
  ScanPreviewSnapshot,
  ScanPreviewTarget,
  ScanReport,
  Scope,
  SoftwareActionOutcomeV1,
  SoftwareAuditRecordV1,
  SoftwareAuditTransition,
  SoftwareEntryV1,
  SoftwareExecutionReportV1,
  SoftwareIdentity,
  SoftwareInventoryV1,
  SoftwareLastUsedEvidence,
  SoftwareLeftoverAppV1,
  SoftwareLeftoverCandidateV1,
  SoftwareLeftoverOutcomeV1,
  SoftwareLeftoverPlanPreviewV1,
  SoftwareLeftoverPlanV1,
  SoftwareLeftoverPreviewV1,
  SoftwareLeftoverReportV1,
  SoftwarePreviewItemV1,
  SoftwarePreviewV1,
  SoftwareSelectionPlanV1,
  SoftwareSizeEvidence,
  SoftwareSourceEvidence,
  SoftwareSourceId,
  SoftwareStartupEntryV1,
  SoftwareStartupListV1,
  SoftwareStartupSourceV1,
  SoftwareStartupToggleReportV1,
  SoftwareUpdateRowV1,
  SoftwareUpdatesV1,
  HudStatusEvent,
  StatusEventV1,
  StatusSnapshotV1,
  UntrustedTarget,
} from "./types.gen";

type RecordValue = Record<string, unknown>;

function record(value: unknown, name: string): RecordValue {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`Invalid ${name}`);
  return value as RecordValue;
}
function string(value: unknown, name: string): string {
  if (typeof value !== "string") throw new Error(`Invalid ${name}`);
  return value;
}
function nonEmptyString(value: unknown, name: string): string {
  const decoded = string(value, name);
  if (decoded.trim().length === 0) throw new Error(`Invalid ${name}`);
  return decoded;
}
function unsignedInteger(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new Error(`Invalid ${name}`);
  return value;
}
function signedInteger(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) throw new Error(`Invalid ${name}`);
  return value;
}
function boolean(value: unknown, name: string): boolean {
  if (typeof value !== "boolean") throw new Error(`Invalid ${name}`);
  return value;
}
function array<T>(value: unknown, name: string, decode: (item: unknown) => T): T[] {
  if (!Array.isArray(value)) throw new Error(`Invalid ${name}`);
  return value.map(decode);
}
function oneOf<T extends string>(value: unknown, values: readonly T[], name: string): T {
  if (typeof value !== "string" || !values.includes(value as T)) throw new Error(`Invalid ${name}`);
  return value as T;
}
function exact(input: RecordValue, keys: readonly string[], name: string): void {
  const allowed = new Set(keys);
  if (Object.keys(input).some((key) => !allowed.has(key))) throw new Error(`Invalid ${name}: unknown field`);
}

const ANALYZE_WARNING_CLASSES = ["partial_budget", "access_denied", "io_error", "churn", "cycle", "reparse", "duplicate_link"] as const;

function nullableUnsignedInteger(value: unknown, name: string): number | null {
  return value === null ? null : unsignedInteger(value, name);
}

function decodeAnalyzeWarning(value: unknown): AnalyzeWarningV1 {
  const input = record(value, "analyze warning");
  exact(input, ["class", "node_id"], "analyze warning");
  return {
    class: oneOf(input.class, ANALYZE_WARNING_CLASSES, "analyze warning.class"),
    node_id: nullableUnsignedInteger(input.node_id, "analyze warning.node_id"),
  };
}

function decodeAnalyzeNode(value: unknown): AnalyzeNodeV1 {
  const input = record(value, "analyze node");
  exact(input, ["id", "parent_id", "kind", "name", "bytes", "immediate_count", "recursive_count", "evidence", "warnings", "mtime_ms"], "analyze node");
  return {
    id: unsignedInteger(input.id, "analyze node.id"),
    parent_id: nullableUnsignedInteger(input.parent_id, "analyze node.parent_id"),
    kind: oneOf(input.kind, ["directory", "file", "reparse"] as const, "analyze node.kind"),
    name: string(input.name, "analyze node.name"),
    bytes: unsignedInteger(input.bytes, "analyze node.bytes"),
    immediate_count: unsignedInteger(input.immediate_count, "analyze node.immediate_count"),
    recursive_count: unsignedInteger(input.recursive_count, "analyze node.recursive_count"),
    evidence: oneOf(input.evidence, ["complete", "incomplete", "unknown"] as const, "analyze node.evidence"),
    warnings: array(input.warnings, "analyze node.warnings", (item) => oneOf(item, ANALYZE_WARNING_CLASSES, "analyze node.warning")),
    mtime_ms: nullableUnsignedInteger(input.mtime_ms, "analyze node.mtime_ms"),
  };
}

export function decodeAnalyzeSnapshot(value: unknown): AnalyzeSnapshotV1 {
  const input = record(value, "analyze snapshot");
  exact(input, ["version", "root", "nodes", "warnings", "completeness", "accounted_owned_bytes"], "analyze snapshot");
  const root = record(input.root, "analyze root");
  exact(root, ["input", "normalized", "volume"], "analyze root");
  const nodes = array(input.nodes, "analyze snapshot.nodes", decodeAnalyzeNode);
  const warnings = array(input.warnings, "analyze snapshot.warnings", decodeAnalyzeWarning);
  const snapshot: AnalyzeSnapshotV1 = {
    version: unsignedInteger(input.version, "analyze snapshot.version"),
    root: {
      input: string(root.input, "analyze root.input"),
      normalized: nonEmptyString(root.normalized, "analyze root.normalized"),
      volume: string(root.volume, "analyze root.volume"),
    },
    nodes,
    warnings,
    completeness: oneOf(input.completeness, ["complete", "partial_budget", "canceled"] as const, "analyze snapshot.completeness"),
    accounted_owned_bytes: unsignedInteger(input.accounted_owned_bytes, "analyze snapshot.accounted_owned_bytes"),
  };
  if (snapshot.version !== 1) throw new Error("Unsupported analyze snapshot version");
  for (const [index, node] of nodes.entries()) {
    if (node.id !== index) throw new Error("Invalid analyze snapshot node order");
    if (index === 0 ? node.parent_id !== null : node.parent_id === null || node.parent_id >= index) {
      throw new Error("Invalid analyze snapshot parent identity");
    }
    if (node.kind !== "directory" && node.immediate_count !== 0) throw new Error("Invalid analyze leaf child count");
  }
  const immediateCounts = new Array<number>(nodes.length).fill(0);
  for (const node of nodes.slice(1)) immediateCounts[node.parent_id!] += 1;
  if (nodes.some((node) => node.immediate_count !== immediateCounts[node.id])) throw new Error("Invalid analyze snapshot immediate count");
  if (warnings.some((warning) => warning.node_id !== null && warning.node_id >= nodes.length)) throw new Error("Invalid analyze warning node identity");
  return snapshot;
}

export function decodeDesktopAnalyzeProgress(value: unknown): DesktopAnalyzeProgress {
  const input = record(value, "desktop analyze progress");
  exact(input, ["operation_id", "sequence", "stored_nodes", "accounted_owned_bytes", "changed_nodes", "queue_depth"], "desktop analyze progress");
  const sequence = unsignedInteger(input.sequence, "analyze progress.sequence");
  const queueDepth = unsignedInteger(input.queue_depth, "analyze progress.queue_depth");
  const changedNodes = array(input.changed_nodes, "analyze progress.changed_nodes", decodeAnalyzeNode);
  if (sequence === 0 || queueDepth > 4 || changedNodes.length > 256) throw new Error("Invalid analyze progress bounds");
  return {
    operation_id: nonEmptyString(input.operation_id, "analyze progress.operation_id"),
    sequence,
    stored_nodes: unsignedInteger(input.stored_nodes, "analyze progress.stored_nodes"),
    accounted_owned_bytes: unsignedInteger(input.accounted_owned_bytes, "analyze progress.accounted_owned_bytes"),
    changed_nodes: changedNodes,
    queue_depth: queueDepth,
  };
}

export function decodeDesktopAnalyzeResult(value: unknown): DesktopAnalyzeResult {
  const input = record(value, "desktop analyze result");
  const type = oneOf(input.type, ["completed", "canceled"] as const, "analyze result.type");
  exact(input, ["type", "operation_id", "snapshot"], "desktop analyze result");
  return {
    type,
    operation_id: nonEmptyString(input.operation_id, "analyze result.operation_id"),
    snapshot: decodeAnalyzeSnapshot(input.snapshot),
  };
}

const ANALYZE_TRASH_REFUSAL_CODES = [
  "protected", "system_location", "volume_root", "profile_root",
  "analysis_root", "reparse_point", "changed_since_snapshot", "not_found",
] as const;
const CONFIRMATION_DIGEST = /^[0-9a-f]{64}$/;

function analyzeTrashTargetId(operationId: string, nodeId: number): string {
  return `analyze.trash:${operationId}:${nodeId}`;
}

function decodeAnalyzeTrashItem(value: unknown, operationId: string): AnalyzeTrashItemV1 {
  const input = record(value, "analyze trash item");
  exact(input, ["node_id", "target_id", "path", "kind", "bytes", "evidence"], "analyze trash item");
  const item: AnalyzeTrashItemV1 = {
    node_id: unsignedInteger(input.node_id, "analyze trash item.node_id"),
    target_id: string(input.target_id, "analyze trash item.target_id"),
    path: nonEmptyString(input.path, "analyze trash item.path"),
    kind: oneOf(input.kind, ["directory", "file", "reparse"] as const, "analyze trash item.kind"),
    bytes: unsignedInteger(input.bytes, "analyze trash item.bytes"),
    evidence: oneOf(input.evidence, ["complete", "incomplete", "unknown"] as const, "analyze trash item.evidence"),
  };
  if (item.target_id !== analyzeTrashTargetId(operationId, item.node_id)) throw new Error("Invalid analyze trash item target identity");
  if (item.kind === "reparse") throw new Error("Invalid analyze trash item kind");
  return item;
}

function decodeAnalyzeTrashRefusal(value: unknown): AnalyzeTrashRefusalV1 {
  const input = record(value, "analyze trash refusal");
  exact(input, ["node_id", "reason_code"], "analyze trash refusal");
  return {
    node_id: unsignedInteger(input.node_id, "analyze trash refusal.node_id"),
    reason_code: oneOf(input.reason_code, ANALYZE_TRASH_REFUSAL_CODES, "analyze trash refusal.reason_code"),
  };
}

export function decodeAnalyzeTrashPreview(value: unknown): AnalyzeTrashPreviewV1 {
  const input = record(value, "analyze trash preview");
  exact(input, ["version", "operation_id", "items", "refused", "digest"], "analyze trash preview");
  const operation_id = nonEmptyString(input.operation_id, "analyze trash preview.operation_id");
  const preview: AnalyzeTrashPreviewV1 = {
    version: unsignedInteger(input.version, "analyze trash preview.version"),
    operation_id,
    items: array(input.items, "analyze trash preview.items", (item) => decodeAnalyzeTrashItem(item, operation_id)),
    refused: array(input.refused, "analyze trash preview.refused", decodeAnalyzeTrashRefusal),
    digest: patternString(input.digest, CONFIRMATION_DIGEST, "analyze trash preview.digest"),
  };
  if (preview.version !== 1) throw new Error("Unsupported analyze trash preview version");
  const ids = [...preview.items.map((item) => item.node_id), ...preview.refused.map((refusal) => refusal.node_id)];
  if (new Set(ids).size !== ids.length) throw new Error("Invalid analyze trash preview duplicate node");
  return preview;
}

export function decodeAnalyzeTrashReport(value: unknown): AnalyzeTrashReportV1 {
  const input = record(value, "analyze trash report");
  exact(input, ["version", "operation_id", "moved_node_ids", "report"], "analyze trash report");
  const operation_id = nonEmptyString(input.operation_id, "analyze trash report.operation_id");
  const result: AnalyzeTrashReportV1 = {
    version: unsignedInteger(input.version, "analyze trash report.version"),
    operation_id,
    moved_node_ids: array(input.moved_node_ids, "analyze trash report.moved_node_ids", (item) => unsignedInteger(item, "analyze trash report.moved_node_id")),
    report: decodeExecutedReport(input.report),
  };
  if (result.version !== 1) throw new Error("Unsupported analyze trash report version");
  const prefix = analyzeTrashTargetId(operation_id, 0).slice(0, -1);
  if (result.report.outcomes.some((outcome) => !outcome.target_id.startsWith(prefix) || outcome.action.type !== "move_to_trash")) {
    throw new Error("Invalid analyze trash report target");
  }
  const succeeded = result.report.outcomes
    .filter((outcome) => outcome.status.type === "succeeded")
    .map((outcome) => outcome.target_id)
    .sort();
  const moved = result.moved_node_ids.map((nodeId) => analyzeTrashTargetId(operation_id, nodeId)).sort();
  if (new Set(moved).size !== moved.length || moved.join(",") !== succeeded.join(",")) throw new Error("Invalid analyze trash report moved nodes");
  return result;
}

const SOFTWARE_ELIGIBILITY_REASONS = [
  "protected_product", "source_incomplete", "conflicting_identity", "no_remove",
  "hidden_entry", "system_or_update", "dependency_package", "stub_package",
  "unhealthy_package", "msi_execution_not_supported_v1", "registry_only_manual",
  "unsupported_source", "eligible_current_user_msix",
] as const;
const SOFTWARE_OUTCOMES = [
  "canceled_before_start", "removed", "reboot_required", "still_present", "failed", "unknown_after_dispatch",
] as const;
const SOFTWARE_ERROR_CODES = [
  "adapter_dispatch_failed", "adapter_operation_failed", "adapter_status_unavailable", "adapter_timed_out",
  "adapter_canceled_after_dispatch", "adapter_cancel_failed", "requery_unavailable", "requery_conflicting", "recovered_after_crash",
] as const;
const SOFTWARE_STATUS_CODES = [
  "validated", "dispatch_started", "adapter_succeeded", "adapter_failed", "adapter_reboot_required",
  "requery_present", "requery_absent", "requery_unavailable", "requery_conflicting",
  "canceled_before_start", "removed", "reboot_required", "still_present", "failed", "unknown_after_dispatch", "recovered_before_dispatch",
] as const;
const SOFTWARE_INSTALLED_STATES = ["present", "absent", "unavailable", "conflicting"] as const;
const SHA256 = /^sha256:[0-9a-f]{64}$/;
const SOFTWARE_TOKEN = /^software-token:sha256:[0-9a-f]{64}$/;

function digest(value: unknown, name: string): string {
  const decoded = string(value, name);
  if (!SHA256.test(decoded)) throw new Error(`Invalid ${name}`);
  return decoded;
}

function optionalString(value: unknown, name: string): string | undefined {
  return value === undefined ? undefined : string(value, name);
}

function decodeSoftwareSourceId(value: unknown): SoftwareSourceId {
  const input = record(value, "software source identity");
  const source = oneOf(input.source, ["arp", "msi", "msix_current_user"] as const, "software source identity.source");
  if (source === "arp") {
    exact(input, ["source", "hive", "view"], "software source identity");
    return {
      source,
      hive: oneOf(input.hive, ["current_user", "local_machine"] as const, "software source identity.hive"),
      view: oneOf(input.view, ["registry32", "registry64"] as const, "software source identity.view"),
    };
  }
  if (source === "msi") {
    exact(input, ["source", "context"], "software source identity");
    return { source, context: oneOf(input.context, ["user_unmanaged", "user_managed", "machine"] as const, "software source identity.context") };
  }
  exact(input, ["source"], "software source identity");
  return { source };
}

function decodeSoftwareIdentity(value: unknown): SoftwareIdentity {
  const input = record(value, "software identity");
  const source = oneOf(input.source, ["arp", "msi", "msix"] as const, "software identity.source");
  if (source === "arp") {
    exact(input, ["source", "hive", "view", "subkey"], "software identity");
    return {
      source,
      hive: oneOf(input.hive, ["current_user", "local_machine"] as const, "software identity.hive"),
      view: oneOf(input.view, ["registry32", "registry64"] as const, "software identity.view"),
      subkey: nonEmptyString(input.subkey, "software identity.subkey"),
    };
  }
  if (source === "msi") {
    exact(input, ["source", "product_code", "context"], "software identity");
    return {
      source,
      product_code: nonEmptyString(input.product_code, "software identity.product_code"),
      context: oneOf(input.context, ["user_unmanaged", "user_managed", "machine"] as const, "software identity.context"),
    };
  }
  exact(input, ["source", "package_full_name"], "software identity");
  return { source, package_full_name: nonEmptyString(input.package_full_name, "software identity.package_full_name") };
}

const SOFTWARE_SIZE_SOURCES = {
  reported_estimate: ["arp_estimated_size_kib", "msi_estimated_size_kib"],
  measured_installed_location: ["msix_installed_path"],
  measured_directory: ["leftover_directory"],
} as const;

function decodeSoftwareSize(value: unknown): SoftwareSizeEvidence {
  const input = record(value, "software size evidence");
  const state = oneOf(input.state, ["available", "partial", "unknown"] as const, "software size evidence.state");
  if (state === "unknown") {
    exact(input, ["state", "reason_code"], "software size evidence");
    return { state, reason_code: nonEmptyString(input.reason_code, "software size evidence.reason_code") };
  }
  const basis = oneOf(input.basis, ["reported_estimate", "measured_installed_location", "measured_directory"] as const, "software size evidence.basis");
  const source_code = oneOf(input.source_code, ["arp_estimated_size_kib", "msi_estimated_size_kib", "msix_installed_path", "leftover_directory"] as const, "software size evidence.source_code");
  if (!(SOFTWARE_SIZE_SOURCES[basis] as readonly string[]).includes(source_code)) throw new Error("Invalid software size basis");
  if (state === "partial") {
    exact(input, ["state", "lower_bound_bytes", "basis", "source_code", "reason_code", "observed_at_unix_ms"], "software size evidence");
    if (basis === "reported_estimate") throw new Error("Invalid software partial size basis");
    return {
      state,
      lower_bound_bytes: unsignedInteger(input.lower_bound_bytes, "software size evidence.lower_bound_bytes"),
      basis,
      source_code,
      reason_code: nonEmptyString(input.reason_code, "software size evidence.reason_code"),
      observed_at_unix_ms: unsignedInteger(input.observed_at_unix_ms, "software size evidence.observed_at_unix_ms"),
    };
  }
  exact(input, ["state", "value_bytes", "basis", "source_code", "observed_at_unix_ms"], "software size evidence");
  return {
    state,
    value_bytes: unsignedInteger(input.value_bytes, "software size evidence.value_bytes"),
    basis,
    source_code,
    observed_at_unix_ms: unsignedInteger(input.observed_at_unix_ms, "software size evidence.observed_at_unix_ms"),
  };
}

function decodeSoftwareLastUsed(value: unknown): SoftwareLastUsedEvidence {
  const input = record(value, "software last-used evidence");
  exact(input, ["state", "reason_code"], "software last-used evidence");
  if (input.state !== "unknown" || input.reason_code !== "no_supported_exact_source") throw new Error("Invalid software last-used evidence");
  return { state: "unknown", reason_code: "no_supported_exact_source" };
}

function decodeSoftwareSourceEvidence(value: unknown): SoftwareSourceEvidence {
  const input = record(value, "software source evidence");
  exact(input, input.reason_code === undefined ? ["source", "state"] : ["source", "state", "reason_code"], "software source evidence");
  const state = oneOf(input.state, ["available", "partial", "permission", "unsupported"] as const, "software source evidence.state");
  const reason_code = optionalString(input.reason_code, "software source evidence.reason_code");
  if ((state === "available") !== (reason_code === undefined)) throw new Error("Invalid software source evidence reason");
  return { source: decodeSoftwareSourceId(input.source), state, ...(reason_code === undefined ? {} : { reason_code }) };
}

function decodeSoftwareEntry(value: unknown): SoftwareEntryV1 {
  const input = record(value, "software entry");
  exact(input, ["id", "identity", "scope", "display_name", "publisher", "version", "provenance", "eligibility", "size", "last_used"].filter((key) => input[key] !== undefined), "software entry");
  const identity = decodeSoftwareIdentity(input.identity);
  const scope = oneOf(input.scope, ["current_user", "machine"] as const, "software entry.scope");
  const eligibilityInput = record(input.eligibility, "software eligibility");
  exact(eligibilityInput, ["state", "reason"], "software eligibility");
  const eligibility = {
    state: oneOf(eligibilityInput.state, ["selectable", "manual"] as const, "software eligibility.state"),
    reason: oneOf(eligibilityInput.reason, SOFTWARE_ELIGIBILITY_REASONS, "software eligibility.reason"),
  };
  if ((eligibility.state === "selectable") !== (eligibility.reason === "eligible_current_user_msix")) throw new Error("Invalid software eligibility pairing");
  if (eligibility.state === "selectable" && (identity.source !== "msix" || scope !== "current_user")) throw new Error("Invalid selectable software identity");
  return {
    id: nonEmptyString(input.id, "software entry.id"),
    identity,
    scope,
    ...(input.display_name === undefined ? {} : { display_name: string(input.display_name, "software entry.display_name") }),
    ...(input.publisher === undefined ? {} : { publisher: string(input.publisher, "software entry.publisher") }),
    ...(input.version === undefined ? {} : { version: string(input.version, "software entry.version") }),
    provenance: array(input.provenance, "software entry.provenance", decodeSoftwareSourceId),
    eligibility,
    size: decodeSoftwareSize(input.size),
    last_used: decodeSoftwareLastUsed(input.last_used),
  };
}

export function decodeSoftwareInventory(value: unknown): SoftwareInventoryV1 {
  const input = record(value, "software inventory");
  exact(input, ["version", "observed_at_unix_ms", "sources", "entries", "fingerprint"], "software inventory");
  const inventory = {
    version: unsignedInteger(input.version, "software inventory.version"),
    observed_at_unix_ms: unsignedInteger(input.observed_at_unix_ms, "software inventory.observed_at_unix_ms"),
    sources: array(input.sources, "software inventory.sources", decodeSoftwareSourceEvidence),
    entries: array(input.entries, "software inventory.entries", decodeSoftwareEntry),
    fingerprint: digest(input.fingerprint, "software inventory.fingerprint"),
  };
  if (inventory.version !== 1) throw new Error("Unsupported software inventory version");
  if (new Set(inventory.entries.map((entry) => entry.id)).size !== inventory.entries.length) throw new Error("Duplicate software inventory identity");
  return inventory;
}

export function decodeDesktopSoftwareInventoryResult(value: unknown): DesktopSoftwareInventoryResult {
  const input = record(value, "desktop software inventory result");
  const type = oneOf(input.type, ["completed", "canceled"] as const, "software inventory result.type");
  exact(input, type === "completed" ? ["type", "operation_id", "inventory"] : ["type", "operation_id"], "desktop software inventory result");
  const operation_id = nonEmptyString(input.operation_id, "software inventory result.operation_id");
  return type === "completed" ? { type, operation_id, inventory: decodeSoftwareInventory(input.inventory) } : { type, operation_id };
}

function decodeSoftwarePlan(value: unknown): SoftwareSelectionPlanV1 {
  const input = record(value, "software selection plan");
  exact(input, ["version", "inventory_fingerprint", "inventory_observed_at_unix_ms", "expires_at_unix_ms", "selected_ids"], "software selection plan");
  const selected_ids = array(input.selected_ids, "software selection plan.selected_ids", (item) => nonEmptyString(item, "software selection plan.selected_id"));
  const plan = {
    version: unsignedInteger(input.version, "software selection plan.version"),
    inventory_fingerprint: digest(input.inventory_fingerprint, "software selection plan.inventory_fingerprint"),
    inventory_observed_at_unix_ms: unsignedInteger(input.inventory_observed_at_unix_ms, "software selection plan.inventory_observed_at_unix_ms"),
    expires_at_unix_ms: unsignedInteger(input.expires_at_unix_ms, "software selection plan.expires_at_unix_ms"),
    selected_ids,
  };
  if (plan.version !== 1 || selected_ids.length === 0 || new Set(selected_ids).size !== selected_ids.length || plan.expires_at_unix_ms < plan.inventory_observed_at_unix_ms) throw new Error("Invalid software selection plan invariant");
  return plan;
}

function decodeSoftwarePreviewItem(value: unknown): SoftwarePreviewItemV1 {
  const input = record(value, "software preview item");
  exact(input, ["id", "identity", "action_class", "scope", "eligibility", "strategy_token"], "software preview item");
  const identity = decodeSoftwareIdentity(input.identity);
  const strategy_token = string(input.strategy_token, "software preview item.strategy_token");
  if (!SOFTWARE_TOKEN.test(strategy_token)) throw new Error("Invalid software preview strategy token");
  if (identity.source !== "msix") throw new Error("Invalid software preview identity");
  return {
    id: nonEmptyString(input.id, "software preview item.id"), identity,
    action_class: oneOf(input.action_class, ["remove_current_user_msix"] as const, "software preview item.action_class"),
    scope: oneOf(input.scope, ["current_user"] as const, "software preview item.scope"),
    eligibility: oneOf(input.eligibility, ["eligible_current_user_msix"] as const, "software preview item.eligibility"),
    strategy_token,
  };
}

function decodeSoftwarePreview(value: unknown): SoftwarePreviewV1 {
  const input = record(value, "software preview");
  exact(input, ["version", "inventory_fingerprint", "irreversible", "selected", "digest"], "software preview");
  const preview = {
    version: unsignedInteger(input.version, "software preview.version"),
    inventory_fingerprint: digest(input.inventory_fingerprint, "software preview.inventory_fingerprint"),
    irreversible: boolean(input.irreversible, "software preview.irreversible"),
    selected: array(input.selected, "software preview.selected", decodeSoftwarePreviewItem),
    digest: digest(input.digest, "software preview.digest"),
  };
  if (preview.version !== 1 || !preview.irreversible || preview.selected.length === 0 || new Set(preview.selected.map((item) => item.id)).size !== preview.selected.length) throw new Error("Invalid software preview invariant");
  return preview;
}

export function decodeDesktopSoftwarePreviewResult(value: unknown): DesktopSoftwarePreviewResult {
  const input = record(value, "desktop software preview result");
  exact(input, ["operation_id", "plan", "preview"], "desktop software preview result");
  const plan = decodeSoftwarePlan(input.plan);
  const preview = decodeSoftwarePreview(input.preview);
  if (plan.inventory_fingerprint !== preview.inventory_fingerprint || plan.selected_ids.join("\n") !== preview.selected.map((item) => item.id).join("\n")) throw new Error("Software preview does not match its selection plan");
  return { operation_id: nonEmptyString(input.operation_id, "software preview result.operation_id"), plan, preview };
}

function decodeSoftwareOutcome(value: unknown): SoftwareActionOutcomeV1 {
  const input = record(value, "software action outcome");
  exact(input, ["operation_id", "software_id", "outcome", "installed_state", "reboot_evidence", "error_code", "irreversible"].filter((key) => input[key] !== undefined), "software action outcome");
  const outcome = oneOf(input.outcome, SOFTWARE_OUTCOMES, "software action outcome.outcome");
  const irreversible = boolean(input.irreversible, "software action outcome.irreversible");
  if (!irreversible) throw new Error("Software action must be irreversible");
  const installed_state = input.installed_state === undefined ? undefined : oneOf(input.installed_state, SOFTWARE_INSTALLED_STATES, "software action outcome.installed_state");
  const error_code = input.error_code === undefined ? undefined : oneOf(input.error_code, SOFTWARE_ERROR_CODES, "software action outcome.error_code");
  return {
    operation_id: nonEmptyString(input.operation_id, "software action outcome.operation_id"),
    software_id: nonEmptyString(input.software_id, "software action outcome.software_id"),
    outcome,
    ...(installed_state === undefined ? {} : { installed_state }),
    reboot_evidence: oneOf(input.reboot_evidence, ["none", "required"] as const, "software action outcome.reboot_evidence"),
    ...(error_code === undefined ? {} : { error_code }),
    irreversible,
  };
}

function decodeSoftwareExecutionReport(value: unknown): SoftwareExecutionReportV1 {
  const input = record(value, "software execution report");
  exact(input, ["version", "irreversible", "outcomes"], "software execution report");
  const report = {
    version: unsignedInteger(input.version, "software execution report.version"),
    irreversible: boolean(input.irreversible, "software execution report.irreversible"),
    outcomes: array(input.outcomes, "software execution report.outcomes", decodeSoftwareOutcome),
  };
  if (report.version !== 1 || !report.irreversible || new Set(report.outcomes.map((item) => item.operation_id)).size !== report.outcomes.length) throw new Error("Invalid software execution report invariant");
  return report;
}

export function decodeDesktopSoftwareUninstallResult(value: unknown): DesktopSoftwareUninstallResult {
  const input = record(value, "desktop software uninstall result");
  exact(input, ["operation_id", "report"], "desktop software uninstall result");
  return { operation_id: nonEmptyString(input.operation_id, "software uninstall result.operation_id"), report: decodeSoftwareExecutionReport(input.report) };
}

function decodeSoftwareAuditTransition(value: unknown): SoftwareAuditTransition {
  const input = record(value, "software audit transition");
  const kind = oneOf(input.kind, ["validated", "dispatch_started", "adapter_completed", "requery_observed", "terminal"] as const, "software audit transition.kind");
  exact(input, kind === "terminal" ? ["kind", "outcome"] : ["kind"], "software audit transition");
  return kind === "terminal" ? { kind, outcome: oneOf(input.outcome, SOFTWARE_OUTCOMES, "software audit transition.outcome") } : { kind };
}

function decodeSoftwareAuditRecord(value: unknown): SoftwareAuditRecordV1 {
  const input = record(value, "software audit record");
  exact(input, ["schema_version", "domain", "operation_id", "timestamp_unix_ms", "identity", "inventory_fingerprint", "preview_digest", "transition", "status_code", "error_code", "reboot_evidence", "installed_state", "requery_result", "adapter_outcome", "irreversible"].filter((key) => input[key] !== undefined), "software audit record");
  const schema_version = unsignedInteger(input.schema_version, "software audit record.schema_version");
  const domain = string(input.domain, "software audit record.domain");
  const irreversible = boolean(input.irreversible, "software audit record.irreversible");
  if (schema_version !== 1 || domain !== "software" || !irreversible) throw new Error("Invalid software audit record invariant");
  const installed_state = input.installed_state === undefined ? undefined : oneOf(input.installed_state, SOFTWARE_INSTALLED_STATES, "software audit record.installed_state");
  const requery_result = input.requery_result === undefined ? undefined : oneOf(input.requery_result, SOFTWARE_INSTALLED_STATES, "software audit record.requery_result");
  const error_code = input.error_code === undefined ? undefined : oneOf(input.error_code, SOFTWARE_ERROR_CODES, "software audit record.error_code");
  const adapter_outcome = input.adapter_outcome === undefined ? undefined : oneOf(input.adapter_outcome, ["success", "failure", "reboot_required", "unfinished"] as const, "software audit record.adapter_outcome");
  return {
    schema_version, domain,
    operation_id: nonEmptyString(input.operation_id, "software audit record.operation_id"),
    timestamp_unix_ms: unsignedInteger(input.timestamp_unix_ms, "software audit record.timestamp_unix_ms"),
    identity: decodeSoftwareIdentity(input.identity),
    inventory_fingerprint: digest(input.inventory_fingerprint, "software audit record.inventory_fingerprint"),
    preview_digest: digest(input.preview_digest, "software audit record.preview_digest"),
    transition: decodeSoftwareAuditTransition(input.transition),
    status_code: oneOf(input.status_code, SOFTWARE_STATUS_CODES, "software audit record.status_code"),
    ...(error_code === undefined ? {} : { error_code }),
    reboot_evidence: oneOf(input.reboot_evidence, ["none", "required"] as const, "software audit record.reboot_evidence"),
    ...(installed_state === undefined ? {} : { installed_state }),
    ...(requery_result === undefined ? {} : { requery_result }),
    ...(adapter_outcome === undefined ? {} : { adapter_outcome }),
    irreversible,
  };
}

export function decodeDesktopSoftwareAuditResult(value: unknown): DesktopSoftwareAuditResult {
  const input = record(value, "desktop software audit result");
  exact(input, ["operation_id", "recovered", "records"], "desktop software audit result");
  return {
    operation_id: nonEmptyString(input.operation_id, "software audit result.operation_id"),
    recovered: array(input.recovered, "software audit result.recovered", decodeSoftwareOutcome),
    records: array(input.records, "software audit result.records", decodeSoftwareAuditRecord),
  };
}

const SOFTWARE_UPDATES_REASONS = [
  "winget_missing", "source_agreement_pending", "non_zero_exit", "timed_out",
  "canceled", "output_truncated", "unrecognized_output", "process_failed",
] as const;
const SOFTWARE_STARTUP_LOCATIONS = ["current_user_run", "machine_run", "machine_run32", "current_user_startup_folder"] as const;
const SOFTWARE_SUPPORT_OUTCOMES = ["succeeded", "failed", "skipped"] as const;
const SOFTWARE_SUPPORT_ERROR_CODES = [
  "registry_write_failed", "verification_failed", "trash_failed", "not_present", "unsafe_path", "protected", "canceled",
] as const;
const SOFTWARE_LEFTOVER_CERTAINTIES = ["certain", "uncertain"] as const;
const LEFTOVER_ID = /^leftover:v1:[0-9a-f]{64}$/;
const STARTUP_ID = /^startup:v1:[0-9a-f]{64}$/;

function patternString(value: unknown, pattern: RegExp, name: string): string {
  const decoded = string(value, name);
  if (!pattern.test(decoded)) throw new Error(`Invalid ${name}`);
  return decoded;
}

function decodeSoftwareUpdateRow(value: unknown): SoftwareUpdateRowV1 {
  const input = record(value, "software update row");
  exact(input, ["id", "name", "name_truncated", "installed_version", "available_version", "source", "matched_software_ids"], "software update row");
  return {
    id: nonEmptyString(input.id, "software update row.id"),
    name: string(input.name, "software update row.name"),
    name_truncated: boolean(input.name_truncated, "software update row.name_truncated"),
    installed_version: string(input.installed_version, "software update row.installed_version"),
    available_version: nonEmptyString(input.available_version, "software update row.available_version"),
    source: string(input.source, "software update row.source"),
    matched_software_ids: array(input.matched_software_ids, "software update row.matched_software_ids", (item) => nonEmptyString(item, "software update row.matched_software_id")),
  };
}

function decodeSoftwareUpdates(value: unknown): SoftwareUpdatesV1 {
  const input = record(value, "software updates");
  const state = oneOf(input.state, ["available", "unavailable"] as const, "software updates.state");
  const version = unsignedInteger(input.version, "software updates.version");
  if (version !== 1) throw new Error("Unsupported software updates version");
  const observed_at_unix_ms = unsignedInteger(input.observed_at_unix_ms, "software updates.observed_at_unix_ms");
  if (state === "unavailable") {
    exact(input, ["state", "version", "observed_at_unix_ms", "reason_code"], "software updates");
    return { state, version, observed_at_unix_ms, reason_code: oneOf(input.reason_code, SOFTWARE_UPDATES_REASONS, "software updates.reason_code") };
  }
  exact(input, ["state", "version", "observed_at_unix_ms", "rows"], "software updates");
  return { state, version, observed_at_unix_ms, rows: array(input.rows, "software updates.rows", decodeSoftwareUpdateRow) };
}

export function decodeDesktopSoftwareUpdatesResult(value: unknown): DesktopSoftwareUpdatesResult {
  const input = record(value, "desktop software updates result");
  exact(input, ["operation_id", "updates"], "desktop software updates result");
  return { operation_id: nonEmptyString(input.operation_id, "software updates result.operation_id"), updates: decodeSoftwareUpdates(input.updates) };
}

function decodeSoftwareStartupEntry(value: unknown): SoftwareStartupEntryV1 {
  const input = record(value, "software startup entry");
  exact(input, ["id", "location", "scope", "name", "state", "toggle"], "software startup entry");
  const location = oneOf(input.location, SOFTWARE_STARTUP_LOCATIONS, "software startup entry.location");
  const scope = oneOf(input.scope, ["current_user", "machine"] as const, "software startup entry.scope");
  const toggle = oneOf(input.toggle, ["allowed", "requires_administrator"] as const, "software startup entry.toggle");
  const machine = location === "machine_run" || location === "machine_run32";
  if (machine !== (scope === "machine") || machine !== (toggle === "requires_administrator")) throw new Error("Invalid software startup entry scope");
  return {
    id: patternString(input.id, STARTUP_ID, "software startup entry.id"),
    location,
    scope,
    name: nonEmptyString(input.name, "software startup entry.name"),
    state: oneOf(input.state, ["enabled", "disabled", "unknown"] as const, "software startup entry.state"),
    toggle,
  };
}

function decodeSoftwareStartupSource(value: unknown): SoftwareStartupSourceV1 {
  const input = record(value, "software startup source");
  exact(input, input.reason_code === undefined ? ["location", "state"] : ["location", "state", "reason_code"], "software startup source");
  const state = oneOf(input.state, ["available", "partial", "permission", "unsupported"] as const, "software startup source.state");
  const reason_code = optionalString(input.reason_code, "software startup source.reason_code");
  if ((state === "available") !== (reason_code === undefined)) throw new Error("Invalid software startup source reason");
  return { location: oneOf(input.location, SOFTWARE_STARTUP_LOCATIONS, "software startup source.location"), state, ...(reason_code === undefined ? {} : { reason_code }) };
}

export function decodeSoftwareStartupList(value: unknown): SoftwareStartupListV1 {
  const input = record(value, "software startup list");
  exact(input, ["version", "observed_at_unix_ms", "sources", "entries"], "software startup list");
  const list = {
    version: unsignedInteger(input.version, "software startup list.version"),
    observed_at_unix_ms: unsignedInteger(input.observed_at_unix_ms, "software startup list.observed_at_unix_ms"),
    sources: array(input.sources, "software startup list.sources", decodeSoftwareStartupSource),
    entries: array(input.entries, "software startup list.entries", decodeSoftwareStartupEntry),
  };
  if (list.version !== 1) throw new Error("Unsupported software startup list version");
  if (new Set(list.entries.map((entry) => entry.id)).size !== list.entries.length) throw new Error("Duplicate software startup entry");
  return list;
}

export function decodeSoftwareStartupToggleReport(value: unknown): SoftwareStartupToggleReportV1 {
  const input = record(value, "software startup toggle report");
  exact(input, ["version", "operation_id", "requested_enabled", "outcome", "error_code", "entry"].filter((key) => input[key] !== undefined), "software startup toggle report");
  const outcome = oneOf(input.outcome, SOFTWARE_SUPPORT_OUTCOMES, "software startup toggle report.outcome");
  const error_code = input.error_code === undefined ? undefined : oneOf(input.error_code, SOFTWARE_SUPPORT_ERROR_CODES, "software startup toggle report.error_code");
  if ((outcome === "succeeded") !== (error_code === undefined)) throw new Error("Invalid software startup toggle outcome");
  const entry = decodeSoftwareStartupEntry(input.entry);
  if (entry.toggle !== "allowed") throw new Error("Invalid software startup toggle scope");
  const version = unsignedInteger(input.version, "software startup toggle report.version");
  if (version !== 1) throw new Error("Unsupported software startup toggle report version");
  return {
    version,
    operation_id: nonEmptyString(input.operation_id, "software startup toggle report.operation_id"),
    requested_enabled: boolean(input.requested_enabled, "software startup toggle report.requested_enabled"),
    outcome,
    ...(error_code === undefined ? {} : { error_code }),
    entry,
  };
}

function decodeSoftwareLeftoverCandidate(value: unknown): SoftwareLeftoverCandidateV1 {
  const input = record(value, "software leftover candidate");
  exact(input, ["id", "path", "origin", "certainty", "selected_by_default", "size"], "software leftover candidate");
  const origin = oneOf(input.origin, ["install_location", "roaming_app_data", "local_app_data", "program_data"] as const, "software leftover candidate.origin");
  const certainty = oneOf(input.certainty, SOFTWARE_LEFTOVER_CERTAINTIES, "software leftover candidate.certainty");
  const selected_by_default = boolean(input.selected_by_default, "software leftover candidate.selected_by_default");
  if ((origin === "install_location") !== (certainty === "certain") || selected_by_default !== (certainty === "certain")) throw new Error("Invalid software leftover certainty");
  return {
    id: patternString(input.id, LEFTOVER_ID, "software leftover candidate.id"),
    path: nonEmptyString(input.path, "software leftover candidate.path"),
    origin,
    certainty,
    selected_by_default,
    size: decodeSoftwareSize(input.size),
  };
}

function decodeSoftwareLeftoverApp(value: unknown): SoftwareLeftoverAppV1 {
  const input = record(value, "software leftover app");
  exact(input, ["software_id", "identity", "display_name", "publisher", "app_size", "candidates"].filter((key) => input[key] !== undefined), "software leftover app");
  const candidates = array(input.candidates, "software leftover app.candidates", decodeSoftwareLeftoverCandidate);
  if (new Set(candidates.map((candidate) => candidate.id)).size !== candidates.length) throw new Error("Duplicate software leftover candidate");
  return {
    software_id: nonEmptyString(input.software_id, "software leftover app.software_id"),
    identity: decodeSoftwareIdentity(input.identity),
    ...(input.display_name === undefined ? {} : { display_name: string(input.display_name, "software leftover app.display_name") }),
    ...(input.publisher === undefined ? {} : { publisher: string(input.publisher, "software leftover app.publisher") }),
    app_size: decodeSoftwareSize(input.app_size),
    candidates,
  };
}

function decodeSoftwareLeftoverPreview(value: unknown): SoftwareLeftoverPreviewV1 {
  const input = record(value, "software leftover preview");
  exact(input, ["version", "apps"], "software leftover preview");
  const version = unsignedInteger(input.version, "software leftover preview.version");
  if (version !== 1) throw new Error("Unsupported software leftover preview version");
  return { version, apps: array(input.apps, "software leftover preview.apps", decodeSoftwareLeftoverApp) };
}

function decodeSoftwareLeftoverPlan(value: unknown): SoftwareLeftoverPlanV1 {
  const input = record(value, "software leftover plan");
  exact(input, ["version", "software_id", "identity", "display_name", "publisher", "uninstall_operation_id", "selected_candidate_ids"].filter((key) => input[key] !== undefined), "software leftover plan");
  const selected_candidate_ids = array(input.selected_candidate_ids, "software leftover plan.selected_candidate_ids", (item) => patternString(item, LEFTOVER_ID, "software leftover plan.selected_candidate_id"));
  const version = unsignedInteger(input.version, "software leftover plan.version");
  if (version !== 1 || selected_candidate_ids.length === 0 || new Set(selected_candidate_ids).size !== selected_candidate_ids.length) throw new Error("Invalid software leftover plan invariant");
  return {
    version,
    software_id: nonEmptyString(input.software_id, "software leftover plan.software_id"),
    identity: decodeSoftwareIdentity(input.identity),
    ...(input.display_name === undefined ? {} : { display_name: string(input.display_name, "software leftover plan.display_name") }),
    ...(input.publisher === undefined ? {} : { publisher: string(input.publisher, "software leftover plan.publisher") }),
    uninstall_operation_id: nonEmptyString(input.uninstall_operation_id, "software leftover plan.uninstall_operation_id"),
    selected_candidate_ids,
  };
}

function decodeSoftwareLeftoverPlanPreview(value: unknown): SoftwareLeftoverPlanPreviewV1 {
  const input = record(value, "software leftover plan preview");
  exact(input, ["version", "items", "digest"], "software leftover plan preview");
  const version = unsignedInteger(input.version, "software leftover plan preview.version");
  if (version !== 1) throw new Error("Unsupported software leftover plan preview version");
  return {
    version,
    items: array(input.items, "software leftover plan preview.items", decodeSoftwareLeftoverCandidate),
    digest: digest(input.digest, "software leftover plan preview.digest"),
  };
}

export function decodeDesktopSoftwareLeftoversPreviewResult(value: unknown): DesktopSoftwareLeftoversPreviewResult {
  const input = record(value, "desktop software leftovers preview result");
  const type = oneOf(input.type, ["discovered", "planned"] as const, "software leftovers preview result.type");
  const operation_id = nonEmptyString(input.operation_id, "software leftovers preview result.operation_id");
  if (type === "discovered") {
    exact(input, ["type", "operation_id", "preview"], "desktop software leftovers preview result");
    return { type, operation_id, preview: decodeSoftwareLeftoverPreview(input.preview) };
  }
  exact(input, ["type", "operation_id", "plan", "preview"], "desktop software leftovers preview result");
  const plan = decodeSoftwareLeftoverPlan(input.plan);
  const preview = decodeSoftwareLeftoverPlanPreview(input.preview);
  if (plan.selected_candidate_ids.join("\n") !== preview.items.map((item) => item.id).join("\n")) throw new Error("Software leftover preview does not match its plan");
  return { type, operation_id, plan, preview };
}

function decodeSoftwareLeftoverOutcome(value: unknown): SoftwareLeftoverOutcomeV1 {
  const input = record(value, "software leftover outcome");
  exact(input, ["candidate_id", "certainty", "outcome", "estimated_bytes", "error_code"].filter((key) => input[key] !== undefined), "software leftover outcome");
  const outcome = oneOf(input.outcome, SOFTWARE_SUPPORT_OUTCOMES, "software leftover outcome.outcome");
  const error_code = input.error_code === undefined ? undefined : oneOf(input.error_code, SOFTWARE_SUPPORT_ERROR_CODES, "software leftover outcome.error_code");
  if ((outcome === "succeeded") !== (error_code === undefined)) throw new Error("Invalid software leftover outcome");
  const estimated_bytes = input.estimated_bytes === undefined ? undefined : unsignedInteger(input.estimated_bytes, "software leftover outcome.estimated_bytes");
  return {
    candidate_id: patternString(input.candidate_id, LEFTOVER_ID, "software leftover outcome.candidate_id"),
    certainty: oneOf(input.certainty, SOFTWARE_LEFTOVER_CERTAINTIES, "software leftover outcome.certainty"),
    outcome,
    ...(estimated_bytes === undefined ? {} : { estimated_bytes }),
    ...(error_code === undefined ? {} : { error_code }),
  };
}

function decodeSoftwareLeftoverReport(value: unknown): SoftwareLeftoverReportV1 {
  const input = record(value, "software leftover report");
  exact(input, ["version", "operation_id", "software_id", "uninstall_operation_id", "outcomes", "moved_known_bytes", "lower_bound"], "software leftover report");
  const version = unsignedInteger(input.version, "software leftover report.version");
  if (version !== 1) throw new Error("Unsupported software leftover report version");
  const outcomes = array(input.outcomes, "software leftover report.outcomes", decodeSoftwareLeftoverOutcome);
  const moved_known_bytes = unsignedInteger(input.moved_known_bytes, "software leftover report.moved_known_bytes");
  const known = outcomes.reduce((sum, item) => sum + (item.outcome === "succeeded" ? item.estimated_bytes ?? 0 : 0), 0);
  if (known !== moved_known_bytes) throw new Error("Invalid software leftover moved total");
  return {
    version,
    operation_id: nonEmptyString(input.operation_id, "software leftover report.operation_id"),
    software_id: nonEmptyString(input.software_id, "software leftover report.software_id"),
    uninstall_operation_id: nonEmptyString(input.uninstall_operation_id, "software leftover report.uninstall_operation_id"),
    outcomes,
    moved_known_bytes,
    lower_bound: boolean(input.lower_bound, "software leftover report.lower_bound"),
  };
}

export function decodeDesktopSoftwareLeftoversResult(value: unknown): DesktopSoftwareLeftoversResult {
  const input = record(value, "desktop software leftovers result");
  exact(input, ["operation_id", "report"], "desktop software leftovers result");
  return { operation_id: nonEmptyString(input.operation_id, "software leftovers result.operation_id"), report: decodeSoftwareLeftoverReport(input.report) };
}

const OPTIMIZE_IDS = [
  "dns.flush",
  "settings.storage_recommendations",
  "settings.search",
  "settings.energy_recommendations",
  "guidance.drive_optimize",
  "guidance.system_integrity",
  "guidance.filesystem_check",
  "guidance.network_reset",
] as const;
const OPTIMIZE_ACTION_CLASSES = ["execute", "settings_handoff", "guidance"] as const;
const OPTIMIZE_OUTCOMES = ["canceled_before_start", "succeeded", "launched", "failed", "unknown_after_dispatch"] as const;
const OPTIMIZE_STATUS_CODES = [
  "validated", "dispatch_started", "adapter_succeeded", "adapter_failed", "adapter_unfinished",
  "canceled_before_start", "recovered_before_dispatch", "succeeded", "launched", "failed", "unknown_after_dispatch",
] as const;
const OPTIMIZE_ERROR_CODES = [
  "adapter_dispatch_failed", "adapter_operation_failed", "adapter_timed_out",
  "adapter_canceled_after_dispatch", "recovered_after_crash",
] as const;

function decodeOptimizeId(value: unknown, name: string): (typeof OPTIMIZE_IDS)[number] {
  return oneOf(value, OPTIMIZE_IDS, name);
}

function decodeMaintenanceActionClass(value: unknown, name: string): MaintenanceActionClass {
  return oneOf(value, OPTIMIZE_ACTION_CLASSES, name);
}

function decodeCatalogueEntry(value: unknown): MaintenanceCatalogueEntryV1 {
  const input = record(value, "optimize catalogue entry");
  exact(input, ["id", "action_class", "build_floor"], "optimize catalogue entry");
  const id = decodeOptimizeId(input.id, "optimize catalogue entry.id");
  const action_class = decodeMaintenanceActionClass(input.action_class, "optimize catalogue entry.action_class");
  if (id.startsWith("guidance.") && action_class !== "guidance") throw new Error("Guidance id must be guidance-only");
  if (id === "dns.flush" && action_class !== "execute") throw new Error("dns.flush must run here");
  if (id.startsWith("settings.") && action_class !== "settings_handoff") throw new Error("Settings id must open Windows Settings");
  const build_floor = input.build_floor === null ? null : unsignedInteger(input.build_floor, "optimize catalogue entry.build_floor");
  if ((id === "settings.storage_recommendations" || id === "settings.search") && build_floor !== 22000) {
    throw new Error("Settings review floor must be 22000");
  }
  if (id === "settings.energy_recommendations" && build_floor !== 22624) {
    throw new Error("Energy floor must be 22624");
  }
  if ((id === "dns.flush" || id.startsWith("guidance.")) && build_floor !== null) {
    throw new Error("Execute and guidance rows have no build floor");
  }
  return { id, action_class, build_floor };
}

export function decodeDesktopOptimizeListResult(value: unknown): DesktopOptimizeListResult {
  const input = record(value, "desktop optimize list result");
  const type = oneOf(input.type, ["completed", "canceled"] as const, "optimize list result.type");
  exact(input, type === "completed" ? ["type", "operation_id", "catalogue_version", "entries"] : ["type", "operation_id"], "desktop optimize list result");
  const operation_id = nonEmptyString(input.operation_id, "optimize list result.operation_id");
  if (type === "canceled") return { type, operation_id };
  const catalogue_version = unsignedInteger(input.catalogue_version, "optimize list result.catalogue_version");
  const entries = array(input.entries, "optimize list result.entries", decodeCatalogueEntry);
  if (catalogue_version !== 1 || entries.length !== 8) throw new Error("Invalid Optimize catalogue envelope");
  if (entries.map((entry) => entry.id).join("\n") !== OPTIMIZE_IDS.join("\n")) throw new Error("Optimize catalogue ids are not the closed V1 set");
  return { type, operation_id, catalogue_version, entries };
}

function decodeMaintenancePlan(value: unknown): MaintenancePlanV1 {
  const input = record(value, "optimize plan");
  exact(input, ["version", "catalogue_version", "operation_id"], "optimize plan");
  const plan = {
    version: unsignedInteger(input.version, "optimize plan.version"),
    catalogue_version: unsignedInteger(input.catalogue_version, "optimize plan.catalogue_version"),
    operation_id: decodeOptimizeId(input.operation_id, "optimize plan.operation_id"),
  };
  if (plan.version !== 1 || plan.catalogue_version !== 1) throw new Error("Invalid Optimize plan version");
  if (plan.operation_id.startsWith("guidance.")) throw new Error("Guidance entries cannot be planned");
  return plan;
}

function decodeMaintenancePreview(value: unknown): MaintenancePreviewV1 {
  const input = record(value, "optimize preview");
  exact(input, ["version", "catalogue_version", "operation_id", "action_class", "digest"], "optimize preview");
  const preview = {
    version: unsignedInteger(input.version, "optimize preview.version"),
    catalogue_version: unsignedInteger(input.catalogue_version, "optimize preview.catalogue_version"),
    operation_id: decodeOptimizeId(input.operation_id, "optimize preview.operation_id"),
    action_class: decodeMaintenanceActionClass(input.action_class, "optimize preview.action_class"),
    digest: digest(input.digest, "optimize preview.digest"),
  };
  if (preview.version !== 1 || preview.catalogue_version !== 1) throw new Error("Invalid Optimize preview version");
  if (preview.action_class === "guidance") throw new Error("Guidance entries cannot be previewed");
  if (preview.operation_id === "dns.flush" && preview.action_class !== "execute") throw new Error("dns.flush preview must execute");
  if (preview.operation_id.startsWith("settings.") && preview.action_class !== "settings_handoff") throw new Error("Settings preview must be a handoff");
  return preview;
}

export function decodeDesktopOptimizePreviewResult(value: unknown): DesktopOptimizePreviewResult {
  const input = record(value, "desktop optimize preview result");
  exact(input, ["operation_id", "plan", "preview"], "desktop optimize preview result");
  const plan = decodeMaintenancePlan(input.plan);
  const preview = decodeMaintenancePreview(input.preview);
  if (plan.operation_id !== preview.operation_id || plan.catalogue_version !== preview.catalogue_version) {
    throw new Error("Optimize preview does not match its plan");
  }
  return { operation_id: nonEmptyString(input.operation_id, "optimize preview result.operation_id"), plan, preview };
}

function decodeMaintenanceOutcome(value: unknown): MaintenanceActionOutcomeV1 {
  const input = record(value, "optimize action outcome");
  exact(input, ["operation_id", "catalogue_id", "action_class", "outcome", "error_code"].filter((key) => input[key] !== undefined), "optimize action outcome");
  const action_class = decodeMaintenanceActionClass(input.action_class, "optimize action outcome.action_class");
  const outcome = oneOf(input.outcome, OPTIMIZE_OUTCOMES, "optimize action outcome.outcome") as MaintenanceExecutionOutcome;
  if (action_class === "guidance") throw new Error("Guidance has no execution outcome");
  if (action_class === "settings_handoff" && outcome === "succeeded") throw new Error("Settings cannot succeed as maintenance completion");
  if (action_class === "execute" && outcome === "launched") throw new Error("DNS flush cannot launch a Settings page");
  const error_code = input.error_code === undefined ? undefined : oneOf(input.error_code, OPTIMIZE_ERROR_CODES, "optimize action outcome.error_code");
  return {
    operation_id: nonEmptyString(input.operation_id, "optimize action outcome.operation_id"),
    catalogue_id: decodeOptimizeId(input.catalogue_id, "optimize action outcome.catalogue_id"),
    action_class,
    outcome,
    ...(error_code === undefined ? {} : { error_code }),
  };
}

function decodeMaintenanceReport(value: unknown): MaintenanceExecutionReportV1 {
  const input = record(value, "optimize execution report");
  exact(input, ["version", "catalogue_version", "outcomes"], "optimize execution report");
  const report = {
    version: unsignedInteger(input.version, "optimize execution report.version"),
    catalogue_version: unsignedInteger(input.catalogue_version, "optimize execution report.catalogue_version"),
    outcomes: array(input.outcomes, "optimize execution report.outcomes", decodeMaintenanceOutcome),
  };
  if (report.version !== 1 || report.catalogue_version !== 1) throw new Error("Invalid Optimize execution report");
  if (new Set(report.outcomes.map((item) => item.operation_id)).size !== report.outcomes.length) throw new Error("Duplicate Optimize outcome identity");
  return report;
}

export function decodeDesktopOptimizeRunResult(value: unknown): DesktopOptimizeRunResult {
  const input = record(value, "desktop optimize run result");
  exact(input, ["operation_id", "report"], "desktop optimize run result");
  return { operation_id: nonEmptyString(input.operation_id, "optimize run result.operation_id"), report: decodeMaintenanceReport(input.report) };
}

function decodeOptimizeAuditTransition(value: unknown): OptimizeAuditTransition {
  const input = record(value, "optimize audit transition");
  const kind = oneOf(input.kind, ["validated", "dispatch_started", "adapter_completed", "terminal"] as const, "optimize audit transition.kind");
  exact(input, kind === "terminal" ? ["kind", "outcome"] : ["kind"], "optimize audit transition");
  return kind === "terminal" ? { kind, outcome: oneOf(input.outcome, OPTIMIZE_OUTCOMES, "optimize audit transition.outcome") } : { kind };
}

function decodeOptimizeAuditRecord(value: unknown): OptimizeAuditRecordV1 {
  const input = record(value, "optimize audit record");
  exact(input, ["schema_version", "domain", "operation_id", "timestamp_unix_ms", "catalogue_version", "catalogue_id", "action_class", "preview_digest", "transition", "status_code", "error_code", "adapter_outcome"].filter((key) => input[key] !== undefined), "optimize audit record");
  const schema_version = unsignedInteger(input.schema_version, "optimize audit record.schema_version");
  const domain = string(input.domain, "optimize audit record.domain");
  if (schema_version !== 1 || domain !== "optimize") throw new Error("Invalid Optimize audit record invariant");
  const error_code = input.error_code === undefined ? undefined : oneOf(input.error_code, OPTIMIZE_ERROR_CODES, "optimize audit record.error_code");
  const adapter_outcome = input.adapter_outcome === undefined ? undefined : oneOf(input.adapter_outcome, ["success", "failure", "unfinished"] as const, "optimize audit record.adapter_outcome");
  return {
    schema_version, domain,
    operation_id: nonEmptyString(input.operation_id, "optimize audit record.operation_id"),
    timestamp_unix_ms: unsignedInteger(input.timestamp_unix_ms, "optimize audit record.timestamp_unix_ms"),
    catalogue_version: unsignedInteger(input.catalogue_version, "optimize audit record.catalogue_version"),
    catalogue_id: decodeOptimizeId(input.catalogue_id, "optimize audit record.catalogue_id"),
    action_class: decodeMaintenanceActionClass(input.action_class, "optimize audit record.action_class"),
    preview_digest: digest(input.preview_digest, "optimize audit record.preview_digest"),
    transition: decodeOptimizeAuditTransition(input.transition),
    status_code: oneOf(input.status_code, OPTIMIZE_STATUS_CODES, "optimize audit record.status_code"),
    ...(error_code === undefined ? {} : { error_code }),
    ...(adapter_outcome === undefined ? {} : { adapter_outcome }),
  };
}

export function decodeDesktopOptimizeAuditResult(value: unknown): DesktopOptimizeAuditResult {
  const input = record(value, "desktop optimize audit result");
  exact(input, ["operation_id", "recovered", "records"], "desktop optimize audit result");
  return {
    operation_id: nonEmptyString(input.operation_id, "optimize audit result.operation_id"),
    recovered: array(input.recovered, "optimize audit result.recovered", decodeMaintenanceOutcome),
    records: array(input.records, "optimize audit result.records", decodeOptimizeAuditRecord),
  };
}

function decodeScope(value: unknown): Scope {
  const input = record(value, "scope");
  const type = oneOf(input.type, ["global", "project"] as const, "scope.type");
  exact(input, type === "global" ? ["type"] : ["type", "root"], "scope");
  return type === "global" ? { type } : { type, root: string(input.root, "scope.root") };
}
function decodeEvidence(value: unknown): Evidence {
  const input = record(value, "evidence");
  const type = oneOf(input.type, ["marker_file", "known_cache_dir", "official_command", "rule_matched", "user_configured"] as const, "evidence.type");
  const keys = {
    marker_file: ["type", "path"], known_cache_dir: ["type", "source", "path"],
    official_command: ["type", "command"], rule_matched: ["type", "rule_id"], user_configured: ["type"],
  } as const;
  exact(input, keys[type], "evidence");
  switch (type) {
    case "marker_file": return { type, path: string(input.path, "evidence.path") };
    case "known_cache_dir": return { type, source: string(input.source, "evidence.source"), path: string(input.path, "evidence.path") };
    case "official_command": return { type, command: string(input.command, "evidence.command") };
    case "rule_matched": return { type, rule_id: string(input.rule_id, "evidence.rule_id") };
    case "user_configured": return { type };
  }
}
function decodeIntent(value: unknown): CleanupIntent {
  const input = record(value, "intent");
  const type = oneOf(input.type, ["trash_project_artifact", "run_built_in_action", "inspect_only"] as const, "intent.type");
  exact(input, type === "run_built_in_action" ? ["type", "provider_id", "action_id"] : ["type", "rule_id"], "intent");
  switch (type) {
    case "trash_project_artifact": return { type, rule_id: string(input.rule_id, "intent.rule_id") };
    case "run_built_in_action": return { type, provider_id: string(input.provider_id, "intent.provider_id"), action_id: string(input.action_id, "intent.action_id") };
    case "inspect_only": return { type, rule_id: string(input.rule_id, "intent.rule_id") };
  }
}
function decodeTarget(value: unknown): UntrustedTarget {
  const input = record(value, "target");
  exact(input, ["id", "rule_id", "scope", "ecosystem", "kind", "path", "estimated_bytes", "size_complete", "sizing_warnings", "last_modified", "risk", "reversible", "selected_by_default", "evidence", "intent"], "target");
  const lastModified = input.last_modified;
  if (lastModified !== null) {
    const time = record(lastModified, "last_modified");
    exact(time, ["secs_since_epoch", "nanos_since_epoch"], "last_modified");
    unsignedInteger(time.secs_since_epoch, "last_modified.secs_since_epoch");
    unsignedInteger(time.nanos_since_epoch, "last_modified.nanos_since_epoch");
  }
  return {
    id: string(input.id, "target.id"), rule_id: string(input.rule_id, "target.rule_id"),
    scope: decodeScope(input.scope), ecosystem: oneOf(input.ecosystem, ["rust", "node", "python", "docker", "generic"] as const, "target.ecosystem"),
    kind: oneOf(input.kind, ["package_cache", "build_artifacts", "dependency_directory", "virtual_env", "test_cache", "tool_cache"] as const, "target.kind"),
    path: input.path === null ? null : string(input.path, "target.path"), estimated_bytes: unsignedInteger(input.estimated_bytes, "target.estimated_bytes"),
    size_complete: boolean(input.size_complete, "target.size_complete"), sizing_warnings: input.sizing_warnings === undefined ? undefined : array(input.sizing_warnings, "target.sizing_warnings", (item) => {
      const warning = record(item, "sizing_warning");
      exact(warning, ["kind", "detail"], "sizing_warning");
      return { kind: oneOf(warning.kind, ["canceled", "entry_budget_exhausted", "metadata_unavailable", "reparse_safety_unverified", "max_depth_reached", "directory_read_failed", "directory_entry_read_failed", "path_unresolved"] as const, "sizing_warning.kind"), detail: string(warning.detail, "sizing_warning.detail") };
    }), last_modified: lastModified as UntrustedTarget["last_modified"],
    risk: oneOf(input.risk, ["low", "medium", "high", "dangerous"] as const, "target.risk"), reversible: boolean(input.reversible, "target.reversible"),
    selected_by_default: boolean(input.selected_by_default, "target.selected_by_default"), evidence: array(input.evidence, "target.evidence", decodeEvidence), intent: decodeIntent(input.intent),
  };
}
function decodeTotals(value: unknown) {
  const input = record(value, "totals");
  exact(input, ["verified_bytes", "partial_lower_bound_bytes", "unknown_target_count"], "totals");
  return { verified_bytes: unsignedInteger(input.verified_bytes, "totals.verified_bytes"), partial_lower_bound_bytes: unsignedInteger(input.partial_lower_bound_bytes, "totals.partial_lower_bound_bytes"), unknown_target_count: unsignedInteger(input.unknown_target_count, "totals.unknown_target_count") };
}
function decodeProcess(value: unknown) {
  const input = record(value, "process");
  exact(input, ["status", "stdout", "stderr"], "process");
  const statusInput = record(input.status, "process.status");
  const type = oneOf(statusInput.type, ["success", "not_found", "timeout", "exit", "invalid_output", "canceled"] as const, "process.status.type");
  exact(statusInput, type === "exit" ? ["type", "code"] : ["type"], "process.status");
  const status = type === "exit" ? { type, code: statusInput.code === null ? null : signedInteger(statusInput.code, "process.status.code") } : { type };
  const output = (value: unknown, name: string) => {
    const stream = record(value, name);
    exact(stream, ["truncated", "retained_bytes", "total_bytes"], name);
    return { truncated: boolean(stream.truncated, `${name}.truncated`), retained_bytes: unsignedInteger(stream.retained_bytes, `${name}.retained_bytes`), total_bytes: unsignedInteger(stream.total_bytes, `${name}.total_bytes`) };
  };
  return { status, stdout: output(input.stdout, "process.stdout"), stderr: output(input.stderr, "process.stderr") };
}
function decodeAction(value: unknown): ActionKind {
  const input = record(value, "action");
  const type = oneOf(input.type, ["command", "move_to_trash", "inspect_only", "permanent_delete"] as const, "action.type");
  exact(input, type === "command" ? ["type", "irreversible"] : ["type"], "action");
  return type === "command" ? { type, irreversible: boolean(input.irreversible, "action.irreversible") } : { type };
}
function decodeCapacity(value: unknown): CapacityEstimate {
  const input = record(value, "capacity");
  const type = oneOf(input.type, ["verified", "partial", "unknown"] as const, "capacity.type");
  exact(input, type === "verified" ? ["type", "bytes"] : type === "partial" ? ["type", "lower_bound_bytes"] : ["type"], "capacity");
  if (type === "verified") return { type, bytes: unsignedInteger(input.bytes, "capacity.bytes") };
  if (type === "partial") return { type, lower_bound_bytes: unsignedInteger(input.lower_bound_bytes, "capacity.lower_bound_bytes") };
  return { type };
}
function decodeStatus(value: unknown): OutcomeStatus {
  const input = record(value, "status");
  const type = oneOf(input.type, ["succeeded", "failed", "skipped"] as const, "status.type");
  exact(input, type === "failed" ? ["type", "message"] : type === "skipped" ? ["type", "reason"] : ["type"], "status");
  if (type === "failed") return { type, message: string(input.message, "status.message") };
  if (type === "skipped") return { type, reason: string(input.reason, "status.reason") };
  return { type };
}
export function decodeExecutionReport(value: unknown): ExecutionReport {
  const input = record(value, "execution report");
  exact(input, ["dry_run", "selected", "attempted", "succeeded", "failed", "skipped", "failures", "outcomes", "notes", "estimated_recoverable", "confirmation_digest", "audit_log"], "execution report");
  const report: ExecutionReport = {
    dry_run: boolean(input.dry_run, "report.dry_run"), selected: unsignedInteger(input.selected, "report.selected"), attempted: unsignedInteger(input.attempted, "report.attempted"),
    succeeded: unsignedInteger(input.succeeded, "report.succeeded"), failed: unsignedInteger(input.failed, "report.failed"), skipped: unsignedInteger(input.skipped, "report.skipped"),
    failures: array(input.failures, "report.failures", (item) => { const failure = record(item, "failure"); exact(failure, ["target_id", "message"], "failure"); return { target_id: string(failure.target_id, "failure.target_id"), message: string(failure.message, "failure.message") }; }),
    outcomes: array(input.outcomes, "report.outcomes", (item) => { const outcome = record(item, "outcome"); exact(outcome, ["target_id", "action", "status", "estimated_recoverable"], "outcome"); return { target_id: string(outcome.target_id, "outcome.target_id"), action: decodeAction(outcome.action), status: decodeStatus(outcome.status), estimated_recoverable: decodeCapacity(outcome.estimated_recoverable) }; }),
    notes: array(input.notes, "report.notes", (item) => { const note = record(item, "note"); exact(note, ["type", "target_id"], "note"); return { type: oneOf(note.type, ["duplicate_selection_removed"] as const, "note.type"), target_id: string(note.target_id, "note.target_id") }; }),
    estimated_recoverable: decodeTotals(input.estimated_recoverable), confirmation_digest: string(input.confirmation_digest, "report.confirmation_digest"),
    audit_log: input.audit_log === null ? null : string(input.audit_log, "report.audit_log"),
  };
  if (report.selected !== report.attempted || report.attempted !== report.outcomes.length || report.succeeded + report.failed + report.skipped !== report.attempted || report.failures.length !== report.failed) {
    throw new Error("Invalid execution report counts");
  }
  const statuses = report.outcomes.reduce((counts, outcome) => {
    counts[outcome.status.type] += 1;
    return counts;
  }, { succeeded: 0, failed: 0, skipped: 0 });
  if (statuses.succeeded !== report.succeeded || statuses.failed !== report.failed || statuses.skipped !== report.skipped) throw new Error("Invalid execution report status counts");
  const outcomeIds = new Set(report.outcomes.map((outcome) => outcome.target_id));
  if (outcomeIds.size !== report.outcomes.length) throw new Error("Invalid execution report duplicate target");
  const failedIds = new Set(report.outcomes.filter((outcome) => outcome.status.type === "failed").map((outcome) => outcome.target_id));
  const failureIds = new Set(report.failures.map((failure) => failure.target_id));
  if (failureIds.size !== failedIds.size || report.failures.some((failure) => !failedIds.has(failure.target_id))) throw new Error("Invalid execution report failures");
  return report;
}
export function decodeDryRunOutcome(value: unknown): DryRunOutcome {
  const input = record(value, "dry-run outcome");
  exact(input, ["report", "digest"], "dry-run outcome");
  const report = decodeExecutionReport(input.report);
  const digest = string(input.digest, "dry-run digest");
  if (!report.dry_run || report.audit_log !== null || digest !== report.confirmation_digest) throw new Error("Invalid dry-run outcome invariant");
  return { report, digest };
}
export function decodeExecutedReport(value: unknown): ExecutionReport {
  const report = decodeExecutionReport(value);
  if (report.dry_run || report.audit_log === null) throw new Error("Invalid execution result mode");
  return report;
}
export function reportMatchesSelection(report: ExecutionReport, selectedIds: readonly string[]): boolean {
  const selected = new Set(selectedIds);
  return selected.size === report.selected && report.outcomes.every((outcome) => selected.has(outcome.target_id));
}
export function decodeScanReport(value: unknown): ScanReport {
  const input = record(value, "scan report");
  exact(input, ["version", "plan", "health"], "scan report");
  const plan = record(input.plan, "plan");
  exact(plan, ["version", "targets"], "plan");
  const health = record(input.health, "health");
  exact(health, ["completeness", "diagnostics", "totals"], "health");
  const report = {
    version: unsignedInteger(input.version, "scan.version"),
    plan: { version: unsignedInteger(plan.version, "plan.version"), targets: array(plan.targets, "plan.targets", decodeTarget) },
    health: { completeness: oneOf(health.completeness, ["complete", "partial"] as const, "health.completeness"), diagnostics: array(health.diagnostics, "health.diagnostics", (item) => {
      const diagnostic = record(item, "diagnostic");
      exact(diagnostic, ["stage", "path", "outcome", "detail", "process"], "diagnostic");
      const decoded = { stage: oneOf(diagnostic.stage, ["discovery", "sizing", "cargo_metadata", "provider"] as const, "diagnostic.stage"), path: string(diagnostic.path, "diagnostic.path"), outcome: oneOf(diagnostic.outcome, ["skipped", "failed", "canceled", "output_truncated"] as const, "diagnostic.outcome"), detail: string(diagnostic.detail, "diagnostic.detail") };
      return diagnostic.process === undefined ? decoded : { ...decoded, process: decodeProcess(diagnostic.process) };
    }), totals: decodeTotals(health.totals) },
  };
  if (report.version !== 1 || report.plan.version !== 2) throw new Error("Unsupported scan or plan version");
  return report;
}

function decodePreviewTarget(value: unknown): ScanPreviewTarget {
  const input = record(value, "scan preview target");
  exact(input, ["id", "scope", "ecosystem", "kind", "path", "estimated_bytes", "size_complete", "sizing_warnings", "last_modified", "risk", "disposition", "evidence"], "scan preview target");
  const lastModified = input.last_modified;
  if (lastModified !== null) {
    const time = record(lastModified, "preview.last_modified");
    exact(time, ["secs_since_epoch", "nanos_since_epoch"], "preview.last_modified");
    unsignedInteger(time.secs_since_epoch, "preview.last_modified.secs_since_epoch");
    unsignedInteger(time.nanos_since_epoch, "preview.last_modified.nanos_since_epoch");
  }
  return {
    id: nonEmptyString(input.id, "preview.id"),
    scope: decodeScope(input.scope),
    ecosystem: oneOf(input.ecosystem, ["rust", "node", "python", "docker", "generic"] as const, "preview.ecosystem"),
    kind: oneOf(input.kind, ["package_cache", "build_artifacts", "dependency_directory", "virtual_env", "test_cache", "tool_cache"] as const, "preview.kind"),
    path: input.path === null ? null : string(input.path, "preview.path"),
    estimated_bytes: unsignedInteger(input.estimated_bytes, "preview.estimated_bytes"),
    size_complete: boolean(input.size_complete, "preview.size_complete"),
    sizing_warnings: array(input.sizing_warnings, "preview.sizing_warnings", (item) => {
      const warning = record(item, "preview.sizing_warning");
      exact(warning, ["kind", "detail"], "preview.sizing_warning");
      return { kind: oneOf(warning.kind, ["canceled", "entry_budget_exhausted", "metadata_unavailable", "reparse_safety_unverified", "max_depth_reached", "directory_read_failed", "directory_entry_read_failed", "path_unresolved"] as const, "preview.sizing_warning.kind"), detail: string(warning.detail, "preview.sizing_warning.detail") };
    }),
    last_modified: lastModified as ScanPreviewTarget["last_modified"],
    risk: oneOf(input.risk, ["low", "medium", "high", "dangerous"] as const, "preview.risk"),
    disposition: oneOf(input.disposition, ["candidate", "inspect_only"] as const, "preview.disposition"),
    evidence: array(input.evidence, "preview.evidence", decodeEvidence),
  };
}

function decodeScanPreview(value: unknown): ScanPreviewSnapshot {
  const input = record(value, "scan preview");
  exact(input, ["targets", "totals"], "scan preview");
  const targets = array(input.targets, "preview.targets", decodePreviewTarget);
  const ids = new Set(targets.map((target) => target.id));
  if (ids.size !== targets.length) throw new Error("Invalid scan preview duplicate target id");
  const totalsInput = record(input.totals, "preview.totals");
  exact(totalsInput, ["target_count", "verified_bytes", "partial_lower_bound_bytes", "unknown_target_count"], "preview.totals");
  const totals = {
    target_count: unsignedInteger(totalsInput.target_count, "preview.totals.target_count"),
    verified_bytes: unsignedInteger(totalsInput.verified_bytes, "preview.totals.verified_bytes"),
    partial_lower_bound_bytes: unsignedInteger(totalsInput.partial_lower_bound_bytes, "preview.totals.partial_lower_bound_bytes"),
    unknown_target_count: unsignedInteger(totalsInput.unknown_target_count, "preview.totals.unknown_target_count"),
  };
  if (totals.target_count !== targets.length) throw new Error("Invalid scan preview target count");
  let verifiedBytes = 0;
  let partialLowerBoundBytes = 0;
  let unknownTargetCount = 0;
  for (const target of targets) {
    if (target.size_complete) verifiedBytes += target.estimated_bytes;
    else if (target.estimated_bytes > 0) partialLowerBoundBytes += target.estimated_bytes;
    else unknownTargetCount += 1;
    if (![verifiedBytes, partialLowerBoundBytes].every(Number.isSafeInteger)) throw new Error("Invalid scan preview capacity totals");
  }
  if (totals.verified_bytes !== verifiedBytes || totals.partial_lower_bound_bytes !== partialLowerBoundBytes || totals.unknown_target_count !== unknownTargetCount) {
    throw new Error("Invalid scan preview capacity totals");
  }
  return { targets, totals };
}

export function decodeDesktopScanProgress(value: unknown): DesktopScanProgress {
  const input = record(value, "desktop scan progress");
  exact(input, ["scan_id", "sequence", "phase", "message", "preview"], "desktop scan progress");
  const sequence = unsignedInteger(input.sequence, "progress.sequence");
  if (sequence === 0) throw new Error("Invalid progress.sequence");
  return {
    scan_id: nonEmptyString(input.scan_id, "progress.scan_id"),
    sequence,
    phase: oneOf(input.phase, ["projects", "global"] as const, "progress.phase"),
    message: string(input.message, "progress.message"),
    preview: input.preview === null ? null : decodeScanPreview(input.preview),
  };
}

export function decodeDesktopScanResult(value: unknown): DesktopScanResult {
  const input = record(value, "desktop scan result");
  const type = oneOf(input.type, ["completed", "canceled"] as const, "scan result.type");
  exact(input, type === "completed" ? ["type", "scan_id", "report"] : ["type", "scan_id"], "desktop scan result");
  const scan_id = nonEmptyString(input.scan_id, "scan result.scan_id");
  return type === "completed" ? { type, scan_id, report: decodeScanReport(input.report) } : { type, scan_id };
}
export function decodeCommandError(value: unknown): CommandError {
  const input = record(value, "command error");
  const code = oneOf(input.code, [
    "scan_already_running", "scan_failed", "analyze_already_running", "analyze_failed", "analyze_stale_operation",
    "software_already_running", "software_failed", "software_stale_authority", "software_audit_unavailable",
    "optimize_already_running", "optimize_failed", "optimize_stale_authority", "optimize_unavailable",
    "optimize_audit_unavailable", "status_already_running", "status_failed", "invalid_plan", "stale_confirmation", "unknown_target", "inspect_only_target", "io",
    "protection_store_unavailable", "protection_audit_unknown", "protection_path_missing", "protection_confirmation_required",
    "history_store_unavailable", "history_not_found", "rule_not_found",
  ] as const, "error.code");
  switch (code) {
    case "scan_already_running":
    case "analyze_already_running":
    case "software_already_running":
    case "optimize_already_running":
    case "status_already_running":
      exact(input, ["code"], "command error"); return { code };
    case "scan_failed":
    case "analyze_failed":
    case "analyze_stale_operation":
    case "software_failed":
    case "software_stale_authority":
    case "software_audit_unavailable":
    case "optimize_failed":
    case "optimize_stale_authority":
    case "optimize_unavailable":
    case "optimize_audit_unavailable":
    case "status_failed":
    case "io":
    case "protection_store_unavailable":
    case "protection_audit_unknown":
    case "protection_path_missing":
    case "protection_confirmation_required":
    case "history_store_unavailable":
    case "history_not_found":
    case "rule_not_found":
      exact(input, ["code", "message"], "command error"); return { code, message: string(input.message, "error.message") };
    case "invalid_plan": exact(input, ["code", "issues"], "command error"); return { code, issues: array(input.issues, "error.issues", (item) => string(item, "error.issue")) };
    case "stale_confirmation": exact(input, ["code", "expected_digest", "actual_digest"], "command error"); return { code, expected_digest: string(input.expected_digest, "error.expected_digest"), actual_digest: string(input.actual_digest, "error.actual_digest") };
    case "unknown_target": case "inspect_only_target": exact(input, ["code", "target_id"], "command error"); return { code, target_id: string(input.target_id, "error.target_id") };
    default: {
      const _exhaustive: never = code;
      return _exhaustive;
    }
  }
}

const PROCESS_ROW_KEYS = [
  "pid", "name", "cpu_basis_points_of_one_logical_core", "private_bytes", "read_bytes_per_second", "write_bytes_per_second",
] as const;
const UNSUPPORTED_CODES = [
  "gpu_utilization", "vram", "thermal", "fan", "smart", "physical_disk_activity",
] as const;
const SNAPSHOT_KEYS = [
  "snapshot_id", "sampled_at_unix_ms", "sample_window_ms", "logical_processor_count",
  "cpu", "memory", "volumes", "network", "power", "gpu", "thermal", "processes", "unsupported_capabilities",
] as const;

function decodeAvailability<T>(
  value: unknown,
  name: string,
  decodeValue: (input: unknown) => T,
): StatusSnapshotV1["cpu"] {
  const input = record(value, name);
  const state = oneOf(input.state, ["available", "partial", "unavailable", "permission_denied", "unsupported"] as const, `${name}.state`);
  if (state === "available") {
    exact(input, ["state", "sampled_at_unix_ms", "age_ms", "value"], name);
    return {
      state,
      sampled_at_unix_ms: unsignedInteger(input.sampled_at_unix_ms, `${name}.sampled_at_unix_ms`),
      age_ms: unsignedInteger(input.age_ms, `${name}.age_ms`),
      value: decodeValue(input.value),
    } as StatusSnapshotV1["cpu"];
  }
  if (state === "partial") {
    exact(input, ["state", "sampled_at_unix_ms", "age_ms", "value", "reason_codes"], name);
    const reason_codes = array(input.reason_codes, `${name}.reason_codes`, (item) => nonEmptyString(item, `${name}.reason_code`));
    if (reason_codes.length === 0) throw new Error(`Invalid ${name}.reason_codes`);
    return {
      state,
      sampled_at_unix_ms: unsignedInteger(input.sampled_at_unix_ms, `${name}.sampled_at_unix_ms`),
      age_ms: unsignedInteger(input.age_ms, `${name}.age_ms`),
      value: decodeValue(input.value),
      reason_codes,
    } as StatusSnapshotV1["cpu"];
  }
  exact(input, ["state", "sampled_at_unix_ms", "reason_code"], name);
  return {
    state,
    sampled_at_unix_ms: nullableUnsignedInteger(input.sampled_at_unix_ms, `${name}.sampled_at_unix_ms`),
    reason_code: nonEmptyString(input.reason_code, `${name}.reason_code`),
  } as StatusSnapshotV1["cpu"];
}

function decodeCpuValue(value: unknown) {
  const input = record(value, "cpu.value");
  exact(input, ["system_utilization_basis_points"], "cpu.value");
  return { system_utilization_basis_points: unsignedInteger(input.system_utilization_basis_points, "cpu.system_utilization_basis_points") };
}

function decodeMemoryValue(value: unknown) {
  const input = record(value, "memory.value");
  exact(input, ["total_bytes", "available_bytes", "used_bytes"], "memory.value");
  return {
    total_bytes: unsignedInteger(input.total_bytes, "memory.total_bytes"),
    available_bytes: unsignedInteger(input.available_bytes, "memory.available_bytes"),
    used_bytes: unsignedInteger(input.used_bytes, "memory.used_bytes"),
  };
}

function decodeVolumesValue(value: unknown) {
  const input = record(value, "volumes.value");
  exact(input, ["items", "complete"], "volumes.value");
  return {
    items: array(input.items, "volumes.items", (item) => {
      const volume = record(item, "volume");
      exact(volume, ["volume_id", "mount_points", "total_bytes", "available_bytes"], "volume");
      return {
        volume_id: nonEmptyString(volume.volume_id, "volume.volume_id"),
        mount_points: array(volume.mount_points, "volume.mount_points", (mount) => string(mount, "volume.mount_point")),
        total_bytes: unsignedInteger(volume.total_bytes, "volume.total_bytes"),
        available_bytes: unsignedInteger(volume.available_bytes, "volume.available_bytes"),
      };
    }),
    complete: boolean(input.complete, "volumes.complete"),
  };
}

function decodeNetworkValue(value: unknown) {
  const input = record(value, "network.value");
  exact(input, ["interval_ms", "interfaces"], "network.value");
  return {
    interval_ms: unsignedInteger(input.interval_ms, "network.interval_ms"),
    interfaces: array(input.interfaces, "network.interfaces", (item) => {
      const iface = record(item, "network.interface");
      exact(iface, ["interface_luid", "name", "rx_bytes_per_second", "tx_bytes_per_second"], "network.interface");
      return {
        interface_luid: nonEmptyString(iface.interface_luid, "network.interface_luid"),
        name: string(iface.name, "network.name"),
        rx_bytes_per_second: unsignedInteger(iface.rx_bytes_per_second, "network.rx_bytes_per_second"),
        tx_bytes_per_second: unsignedInteger(iface.tx_bytes_per_second, "network.tx_bytes_per_second"),
      };
    }),
  };
}

function decodePowerValue(value: unknown) {
  const input = record(value, "power.value");
  exact(input, ["battery_present", "ac_state", "charge_basis_points", "remaining_seconds"], "power.value");
  const battery_present = boolean(input.battery_present, "power.battery_present");
  const charge_basis_points = nullableUnsignedInteger(input.charge_basis_points, "power.charge_basis_points");
  const remaining_seconds = nullableUnsignedInteger(input.remaining_seconds, "power.remaining_seconds");
  if (!battery_present && (charge_basis_points !== null || remaining_seconds !== null)) {
    throw new Error("Missing battery must not report charge or remaining time");
  }
  return {
    battery_present,
    ac_state: oneOf(input.ac_state, ["online", "offline", "unknown"] as const, "power.ac_state"),
    charge_basis_points,
    remaining_seconds,
  };
}

function decodeGpuValue(value: unknown) {
  const input = record(value, "gpu.value");
  exact(input, ["adapters"], "gpu.value");
  return {
    adapters: array(input.adapters, "gpu.adapters", (item) => {
      const adapter = record(item, "gpu.adapter");
      exact(adapter, ["adapter_id", "utilization_basis_points"], "gpu.adapter");
      const utilization_basis_points = unsignedInteger(adapter.utilization_basis_points, "gpu.utilization_basis_points");
      if (utilization_basis_points > 10_000) throw new Error("Invalid gpu.utilization_basis_points");
      return { adapter_id: nonEmptyString(adapter.adapter_id, "gpu.adapter_id"), utilization_basis_points };
    }),
  };
}

function decodeThermalValue(value: unknown) {
  const input = record(value, "thermal.value");
  exact(input, ["zones"], "thermal.value");
  return {
    zones: array(input.zones, "thermal.zones", (item) => {
      const zone = record(item, "thermal.zone");
      exact(zone, ["zone_id", "temperature_tenths_celsius"], "thermal.zone");
      return {
        zone_id: nonEmptyString(zone.zone_id, "thermal.zone_id"),
        temperature_tenths_celsius: signedInteger(zone.temperature_tenths_celsius, "thermal.temperature_tenths_celsius"),
      };
    }),
  };
}

function decodeProcessRow(value: unknown) {
  const input = record(value, "process");
  exact(input, PROCESS_ROW_KEYS, "process");
  return {
    pid: unsignedInteger(input.pid, "process.pid"),
    name: string(input.name, "process.name"),
    cpu_basis_points_of_one_logical_core: unsignedInteger(input.cpu_basis_points_of_one_logical_core, "process.cpu"),
    private_bytes: unsignedInteger(input.private_bytes, "process.private_bytes"),
    read_bytes_per_second: unsignedInteger(input.read_bytes_per_second, "process.read_bytes_per_second"),
    write_bytes_per_second: unsignedInteger(input.write_bytes_per_second, "process.write_bytes_per_second"),
  };
}

function decodeProcessesValue(value: unknown) {
  const input = record(value, "processes.value");
  exact(input, [
    "items", "enumerated_count", "returned_count", "requested_limit", "enumeration_ceiling",
    "detail_budget_ms", "truncated_by_limit", "budget_exhausted",
  ], "processes.value");
  const items = array(input.items, "processes.items", decodeProcessRow);
  const returned_count = unsignedInteger(input.returned_count, "processes.returned_count");
  if (returned_count !== items.length) throw new Error("Invalid processes.returned_count");
  return {
    items,
    enumerated_count: unsignedInteger(input.enumerated_count, "processes.enumerated_count"),
    returned_count,
    requested_limit: unsignedInteger(input.requested_limit, "processes.requested_limit"),
    enumeration_ceiling: unsignedInteger(input.enumeration_ceiling, "processes.enumeration_ceiling"),
    detail_budget_ms: unsignedInteger(input.detail_budget_ms, "processes.detail_budget_ms"),
    truncated_by_limit: boolean(input.truncated_by_limit, "processes.truncated_by_limit"),
    budget_exhausted: boolean(input.budget_exhausted, "processes.budget_exhausted"),
  };
}

function decodeUnsupportedCapability(value: unknown) {
  const input = record(value, "unsupported capability");
  exact(input, ["code", "state", "reason_code"], "unsupported capability");
  return {
    code: oneOf(input.code, UNSUPPORTED_CODES, "unsupported capability.code"),
    state: oneOf(input.state, ["unsupported"] as const, "unsupported capability.state"),
    reason_code: nonEmptyString(input.reason_code, "unsupported capability.reason_code"),
  };
}

function hasMeasuredValue(availability: { readonly state: string }): boolean {
  return availability.state === "available" || availability.state === "partial";
}

export function decodeStatusSnapshot(value: unknown): StatusSnapshotV1 {
  const input = record(value, "status snapshot");
  exact(input, SNAPSHOT_KEYS, "status snapshot");
  const gpu = decodeAvailability(input.gpu, "gpu", decodeGpuValue) as StatusSnapshotV1["gpu"];
  const thermal = decodeAvailability(input.thermal, "thermal", decodeThermalValue) as StatusSnapshotV1["thermal"];
  const unsupported_capabilities = array(input.unsupported_capabilities, "unsupported_capabilities", decodeUnsupportedCapability);
  const codes = unsupported_capabilities.map((item) => item.code);
  const expected = UNSUPPORTED_CODES.filter((code) =>
    code === "gpu_utilization" ? !hasMeasuredValue(gpu) : code === "thermal" ? !hasMeasuredValue(thermal) : true);
  if (codes.join("\n") !== expected.join("\n")) {
    throw new Error("Status V1 unsupported capabilities do not match the GPU and thermal probes");
  }
  return {
    snapshot_id: nonEmptyString(input.snapshot_id, "snapshot.snapshot_id"),
    sampled_at_unix_ms: unsignedInteger(input.sampled_at_unix_ms, "snapshot.sampled_at_unix_ms"),
    sample_window_ms: unsignedInteger(input.sample_window_ms, "snapshot.sample_window_ms"),
    logical_processor_count: unsignedInteger(input.logical_processor_count, "snapshot.logical_processor_count"),
    cpu: decodeAvailability(input.cpu, "cpu", decodeCpuValue) as StatusSnapshotV1["cpu"],
    memory: decodeAvailability(input.memory, "memory", decodeMemoryValue) as StatusSnapshotV1["memory"],
    volumes: decodeAvailability(input.volumes, "volumes", decodeVolumesValue) as StatusSnapshotV1["volumes"],
    network: decodeAvailability(input.network, "network", decodeNetworkValue) as StatusSnapshotV1["network"],
    power: decodeAvailability(input.power, "power", decodePowerValue) as StatusSnapshotV1["power"],
    gpu,
    thermal,
    processes: decodeAvailability(input.processes, "processes", decodeProcessesValue) as StatusSnapshotV1["processes"],
    unsupported_capabilities,
  };
}

export function decodeStatusEvent(value: unknown): StatusEventV1 {
  const input = record(value, "status event");
  const event = oneOf(input.event, ["status_started", "status_snapshot", "tick_skipped", "status_terminal"] as const, "status event.event");
  exact(input, ["schema_version", "event", "operation_id", "sequence", "emitted_at_unix_ms", "data"], "status event");
  const schema_version = unsignedInteger(input.schema_version, "status event.schema_version");
  if (schema_version !== 1) throw new Error("Unsupported status event schema_version");
  const envelope = {
    schema_version,
    operation_id: nonEmptyString(input.operation_id, "status event.operation_id"),
    sequence: unsignedInteger(input.sequence, "status event.sequence"),
    emitted_at_unix_ms: unsignedInteger(input.emitted_at_unix_ms, "status event.emitted_at_unix_ms"),
  };
  if (event === "status_started") {
    const data = record(input.data, "status_started.data");
    exact(data, ["interval_ms", "process_limit"], "status_started.data");
    return {
      ...envelope,
      event,
      data: {
        interval_ms: unsignedInteger(data.interval_ms, "status_started.interval_ms"),
        process_limit: unsignedInteger(data.process_limit, "status_started.process_limit"),
      },
    };
  }
  if (event === "status_snapshot") {
    return { ...envelope, event, data: decodeStatusSnapshot(input.data) };
  }
  if (event === "tick_skipped") {
    const data = record(input.data, "tick_skipped.data");
    exact(data, ["reason", "skipped_total"], "tick_skipped.data");
    return {
      ...envelope,
      event,
      data: {
        reason: oneOf(data.reason, ["sample_in_flight"] as const, "tick_skipped.reason"),
        skipped_total: unsignedInteger(data.skipped_total, "tick_skipped.skipped_total"),
      },
    };
  }
  const data = record(input.data, "status_terminal.data");
  exact(data, ["reason", "error_code"], "status_terminal.data");
  return {
    ...envelope,
    event,
    data: {
      reason: oneOf(data.reason, ["completed", "canceled", "broken_pipe", "producer_error"] as const, "status_terminal.reason"),
      error_code: data.error_code === null ? null : nonEmptyString(data.error_code, "status_terminal.error_code"),
    },
  };
}

export function decodeHudStatusEvent(value: unknown): HudStatusEvent {
  const input = record(value, "hud status event");
  const type = oneOf(input.type, ["sampling", "snapshot"] as const, "hud status event.type");
  exact(input, type === "snapshot" ? ["type", "snapshot"] : ["type"], "hud status event");
  return type === "snapshot" ? { type, snapshot: decodeStatusSnapshot(input.snapshot) } : { type };
}

export function decodeDesktopStatusSnapshotResult(value: unknown): DesktopStatusSnapshotResult {
  const input = record(value, "desktop status snapshot result");
  const type = oneOf(input.type, ["completed", "canceled"] as const, "status snapshot result.type");
  exact(input, type === "completed" ? ["type", "operation_id", "snapshot"] : ["type", "operation_id"], "desktop status snapshot result");
  const operation_id = nonEmptyString(input.operation_id, "status snapshot result.operation_id");
  return type === "completed"
    ? { type, operation_id, snapshot: decodeStatusSnapshot(input.snapshot) }
    : { type, operation_id };
}

export function decodeDesktopStatusLiveResult(value: unknown): DesktopStatusLiveResult {
  const input = record(value, "desktop status live result");
  const type = oneOf(input.type, ["completed", "canceled"] as const, "status live result.type");
  exact(input, ["type", "operation_id"], "desktop status live result");
  return { type, operation_id: nonEmptyString(input.operation_id, "status live result.operation_id") };
}
