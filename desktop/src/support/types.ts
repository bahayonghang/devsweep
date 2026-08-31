export type HistoryDomain = "clean" | "software" | "optimize";
export type HistoryStoreState = "available" | "empty" | "unavailable" | "unsupported";

export interface ProtectionMutationReport {
  readonly operation_id: string;
  readonly action: "add" | "remove";
  readonly outcome_code: string;
  readonly error_code: string | null;
  readonly identity_sha256: string;
  readonly display_path: string;
  readonly changed: boolean;
}

export interface RuleProjectionV1 {
  readonly id: string;
  readonly source: "shipped_registry";
  readonly ecosystem: string;
  readonly scope: string;
  readonly safety_class: string;
  readonly risk: string;
  readonly platform_applicability: string;
  readonly inspect_only: boolean;
  readonly rationale: string;
}

export interface HistoryStoreStatusV1 {
  readonly domain: HistoryDomain;
  readonly state: HistoryStoreState;
  readonly reason_code: string | null;
}

export interface HistoryOperationSummaryV1 {
  readonly domain: HistoryDomain;
  readonly operation_id: string;
  readonly first_timestamp_unix_ms: number;
  readonly last_timestamp_unix_ms: number;
  readonly outcome_code: string;
  readonly stable_codes: readonly string[];
  readonly partial: boolean;
  readonly unknown: boolean;
  readonly unsupported: boolean;
}

export interface HistoryListV1 {
  readonly operations: readonly HistoryOperationSummaryV1[];
  readonly stores: readonly HistoryStoreStatusV1[];
}

export interface HistoryDetailV1 {
  readonly summary: HistoryOperationSummaryV1;
  readonly records: readonly Record<string, unknown>[];
}
