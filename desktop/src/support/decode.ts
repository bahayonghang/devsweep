import type {
  HistoryDetailV1,
  HistoryDomain,
  HistoryListV1,
  HistoryOperationSummaryV1,
  HistoryStoreStatusV1,
  ProtectionMutationReport,
  RuleProjectionV1,
} from "./types";
import type { CleanMovedTotalsV1 } from "../api/types.gen";

const FORBIDDEN = new Set(["argv", "args", "program", "cwd", "env", "environment", "action_path", "plan", "message", "command", "path"]);

function record(value: unknown, name: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`Invalid ${name}`);
  return value as Record<string, unknown>;
}

function string(value: unknown, name: string): string {
  if (typeof value !== "string") throw new Error(`Invalid ${name}`);
  return value;
}

function rejectForbidden(value: unknown, name: string): void {
  if (Array.isArray(value)) {
    value.forEach((item, index) => rejectForbidden(item, `${name}[${index}]`));
    return;
  }
  if (typeof value !== "object" || value === null) return;
  for (const [key, child] of Object.entries(value)) {
    if (FORBIDDEN.has(key)) throw new Error(`${name} contains forbidden field ${key}`);
    rejectForbidden(child, `${name}.${key}`);
  }
}

export function decodeProtectionPaths(value: unknown): string[] {
  if (!Array.isArray(value)) throw new Error("Invalid protection list");
  return value.map((item, index) => {
    if (typeof item === "string") return item;
    throw new Error(`Invalid protection path ${index}`);
  });
}

export function decodeProtectionMutation(value: unknown): ProtectionMutationReport {
  const input = record(value, "protection mutation");
  const identity = string(input.identity_sha256, "identity_sha256");
  if (!/^[0-9a-f]{64}$/.test(identity)) throw new Error("Invalid identity hash");
  return {
    operation_id: string(input.operation_id, "operation_id"),
    action: input.action === "add" || input.action === "remove" ? input.action : (() => { throw new Error("Invalid action"); })(),
    outcome_code: string(input.outcome_code, "outcome_code"),
    error_code: input.error_code === null || input.error_code === undefined ? null : string(input.error_code, "error_code"),
    identity_sha256: identity,
    display_path: string(input.display_path, "display_path"),
    changed: typeof input.changed === "boolean" ? input.changed : (() => { throw new Error("Invalid changed"); })(),
  };
}

export function decodeRuleProjection(value: unknown): RuleProjectionV1 {
  const input = record(value, "rule");
  return {
    id: string(input.id, "id"),
    source: input.source === "shipped_registry" ? "shipped_registry" : (() => { throw new Error("Invalid rule source"); })(),
    ecosystem: string(input.ecosystem, "ecosystem"),
    scope: string(input.scope, "scope"),
    safety_class: string(input.safety_class, "safety_class"),
    risk: string(input.risk, "risk"),
    platform_applicability: string(input.platform_applicability, "platform_applicability"),
    inspect_only: Boolean(input.inspect_only),
    rationale: string(input.rationale, "rationale"),
  };
}

export function decodeRules(value: unknown): RuleProjectionV1[] {
  if (!Array.isArray(value)) throw new Error("Invalid rules list");
  return value.map(decodeRuleProjection);
}

function decodeDomain(value: unknown): HistoryDomain {
  if (value === "clean" || value === "software" || value === "optimize") return value;
  throw new Error("Invalid history domain");
}

function decodeStore(value: unknown): HistoryStoreStatusV1 {
  const input = record(value, "history store");
  const state = input.state;
  if (state !== "available" && state !== "empty" && state !== "unavailable" && state !== "unsupported") {
    throw new Error("Invalid history store state");
  }
  return {
    domain: decodeDomain(input.domain),
    state,
    reason_code: input.reason_code === null || input.reason_code === undefined ? null : string(input.reason_code, "reason_code"),
  };
}

function decodeSummary(value: unknown): HistoryOperationSummaryV1 {
  const input = record(value, "history summary");
  return {
    domain: decodeDomain(input.domain),
    operation_id: string(input.operation_id, "operation_id"),
    first_timestamp_unix_ms: Number(input.first_timestamp_unix_ms),
    last_timestamp_unix_ms: Number(input.last_timestamp_unix_ms),
    outcome_code: string(input.outcome_code, "outcome_code"),
    stable_codes: Array.isArray(input.stable_codes) ? input.stable_codes.map((code) => string(code, "stable_code")) : [],
    partial: Boolean(input.partial),
    unknown: Boolean(input.unknown),
    unsupported: Boolean(input.unsupported),
  };
}

export function decodeHistoryList(value: unknown): HistoryListV1 {
  rejectForbidden(value, "history list");
  const input = record(value, "history list");
  return {
    operations: Array.isArray(input.operations) ? input.operations.map(decodeSummary) : [],
    stores: Array.isArray(input.stores) ? input.stores.map(decodeStore) : [],
  };
}

export function decodeHistoryDetail(value: unknown): HistoryDetailV1 {
  rejectForbidden(value, "history detail");
  const input = record(value, "history detail");
  return {
    summary: decodeSummary(input.summary),
    records: Array.isArray(input.records) ? input.records.map((item) => record(item, "history record")) : [],
  };
}

const CLEAN_MOVED_TOTALS_KEYS = ["known_bytes", "lower_bound", "unknown_records"];

function count(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new Error(`Invalid ${name}`);
  return value;
}

export function decodeCleanMovedTotals(value: unknown): CleanMovedTotalsV1 {
  const input = record(value, "clean moved totals");
  const keys = Object.keys(input).sort();
  if (keys.length !== CLEAN_MOVED_TOTALS_KEYS.length || keys.some((key, index) => key !== CLEAN_MOVED_TOTALS_KEYS[index])) {
    throw new Error("Invalid clean moved totals fields");
  }
  if (typeof input.lower_bound !== "boolean") throw new Error("Invalid lower_bound");
  const unknownRecords = count(input.unknown_records, "unknown_records");
  if (unknownRecords > 0 && !input.lower_bound) throw new Error("Unknown records require a lower bound");
  return {
    known_bytes: count(input.known_bytes, "known_bytes"),
    unknown_records: unknownRecords,
    lower_bound: input.lower_bound,
  };
}
