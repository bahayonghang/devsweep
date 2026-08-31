import type {
  ActionKind,
  AnalyzeNodeV1,
  AnalyzeSnapshotV1,
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
  DesktopSoftwarePreviewResult,
  DesktopSoftwareUninstallResult,
  DryRunOutcome,
  Evidence,
  ExecutionReport,
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
  SoftwarePreviewItemV1,
  SoftwarePreviewV1,
  SoftwareSelectionPlanV1,
  SoftwareSizeEvidence,
  SoftwareSourceEvidence,
  SoftwareSourceId,
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

function decodeSoftwareSize(value: unknown): SoftwareSizeEvidence {
  const input = record(value, "software size evidence");
  const state = oneOf(input.state, ["available", "partial", "unknown"] as const, "software size evidence.state");
  if (state === "unknown") {
    exact(input, ["state", "reason_code"], "software size evidence");
    return { state, reason_code: nonEmptyString(input.reason_code, "software size evidence.reason_code") };
  }
  const basis = oneOf(input.basis, ["reported_estimate", "measured_installed_location"] as const, "software size evidence.basis");
  const source_code = oneOf(input.source_code, ["arp_estimated_size_kib", "msi_estimated_size_kib", "msix_installed_path"] as const, "software size evidence.source_code");
  if (state === "partial") {
    exact(input, ["state", "lower_bound_bytes", "basis", "source_code", "reason_code", "observed_at_unix_ms"], "software size evidence");
    if (basis !== "measured_installed_location" || source_code !== "msix_installed_path") throw new Error("Invalid software partial size basis");
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
  if ((basis === "measured_installed_location") !== (source_code === "msix_installed_path")) throw new Error("Invalid software available size basis");
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
  const code = oneOf(input.code, ["scan_already_running", "scan_failed", "analyze_already_running", "analyze_failed", "invalid_plan", "stale_confirmation", "unknown_target", "inspect_only_target", "io"] as const, "error.code");
  switch (code) {
    case "scan_already_running":
    case "analyze_already_running":
      exact(input, ["code"], "command error"); return { code };
    case "scan_failed":
    case "analyze_failed":
    case "io":
      exact(input, ["code", "message"], "command error"); return { code, message: string(input.message, "error.message") };
    case "invalid_plan": exact(input, ["code", "issues"], "command error"); return { code, issues: array(input.issues, "error.issues", (item) => string(item, "error.issue")) };
    case "stale_confirmation": exact(input, ["code", "expected_digest", "actual_digest"], "command error"); return { code, expected_digest: string(input.expected_digest, "error.expected_digest"), actual_digest: string(input.actual_digest, "error.actual_digest") };
    case "unknown_target": case "inspect_only_target": exact(input, ["code", "target_id"], "command error"); return { code, target_id: string(input.target_id, "error.target_id") };
  }
}
