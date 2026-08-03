import type {
  ActionKind,
  CapacityEstimate,
  CleanupIntent,
  CommandError,
  DryRunOutcome,
  Evidence,
  ExecutionReport,
  OutcomeStatus,
  ScanProgress,
  ScanReport,
  Scope,
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
export function decodeScanProgress(value: unknown): ScanProgress {
  const input = record(value, "scan progress");
  exact(input, ["phase", "message", "partial"], "scan progress");
  if (input.partial !== null) throw new Error("Invalid progress.partial authority");
  return { phase: oneOf(input.phase, ["projects", "global"] as const, "progress.phase"), message: string(input.message, "progress.message"), partial: null };
}
export function decodeCommandError(value: unknown): CommandError {
  const input = record(value, "command error");
  const code = oneOf(input.code, ["scan_already_running", "scan_failed", "invalid_plan", "stale_confirmation", "unknown_target", "inspect_only_target", "io"] as const, "error.code");
  switch (code) {
    case "scan_already_running": exact(input, ["code"], "command error"); return { code };
    case "scan_failed": case "io": exact(input, ["code", "message"], "command error"); return { code, message: string(input.message, "error.message") };
    case "invalid_plan": exact(input, ["code", "issues"], "command error"); return { code, issues: array(input.issues, "error.issues", (item) => string(item, "error.issue")) };
    case "stale_confirmation": exact(input, ["code", "expected_digest", "actual_digest"], "command error"); return { code, expected_digest: string(input.expected_digest, "error.expected_digest"), actual_digest: string(input.actual_digest, "error.actual_digest") };
    case "unknown_target": case "inspect_only_target": exact(input, ["code", "target_id"], "command error"); return { code, target_id: string(input.target_id, "error.target_id") };
  }
}
