import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import {
  FetchingJSONSchemaStore,
  InputData,
  JSONSchemaInput,
  quicktype,
} from "quicktype-core";

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const args = process.argv.slice(2);
const fixtureArg = args.indexOf("--fixtures");
const fixtureRoot = fixtureArg >= 0 ? path.resolve(args[fixtureArg + 1]) : path.join(root, "src/api/fixtures");
const stdout = args.includes("--stdout");
const check = args.includes("--check");
const typesPath = path.join(root, "src/api/types.gen.ts");

const rootFixtures = {
  DesktopPreferencesSnapshot: ["desktop-preferences.json", "desktop-preferences-updated.json"],
  ScanReport: ["scan-report.json", "scan-report.real.json", "clean/scan-report.json"],
  DesktopScanProgress: ["scan-progress.json", "clean/scan-progress.json"],
  DryRunOutcome: ["dry-run-outcome.json", "dry-run-outcome-two-targets.json", "clean/dry-run-outcome.json"],
  ExecutionReport: ["execution-report.json", "clean/execution-report.json"],
  AnalyzeSnapshotV1: ["analyze/snapshot.json"],
  DesktopAnalyzeProgress: ["analyze/progress.json"],
  AnalyzeTrashPreviewV1: ["analyze/trash-preview.json"],
  AnalyzeTrashReportV1: ["analyze/trash-report.json"],
  SoftwareInventoryV1: ["software/inventory.json", "software/inventory-partial.json", "software/all-msi-manual.json"],
  DesktopSoftwarePreviewResult: ["software/preview.json"],
  DesktopSoftwareUninstallResult: ["software/execution-five-terminal.json"],
  DesktopSoftwareAuditResult: ["software/audit-restart.json"],
  DesktopSoftwareUpdatesResult: ["software/updates-available.json", "software/updates-unavailable.json"],
  SoftwareStartupListV1: ["software/startup-list.json"],
  SoftwareStartupToggleReportV1: ["software/startup-toggle.json"],
  DesktopSoftwareLeftoversResult: ["software/leftovers-report.json"],
  DesktopOptimizeListResult: ["optimize/catalogue.json"],
  DesktopOptimizePreviewResult: [
    "optimize/preview-dns.json",
    "optimize/preview-settings-search.json",
    "optimize/preview-settings-storage.json",
    "optimize/preview-settings-energy.json"
  ],
  DesktopOptimizeRunResult: [
    "optimize/execution-dns-succeeded.json",
    "optimize/execution-settings-launched.json",
    "optimize/execution-five-terminal.json"
  ],
  DesktopOptimizeAuditResult: ["optimize/audit.json"],
  StatusSnapshotV1: ["status/snapshot.json", "status/live-snapshot.json"],
  DesktopStatusSnapshotResult: ["status/snapshot-completed.json"],
  DesktopStatusLiveResult: ["status/live-completed.json"],
  HudStatusEvent: ["status/hud-sampling.json", "status/hud-snapshot.json"],
  CleanMovedTotalsV1: ["history/clean-moved-totals.json"],
};

// This graph names nested Rust-owned DTOs. Fields, optionality, nullability,
// variants, and primitive shapes still come exclusively from fixture values.
const references = {
  DesktopPreferencesSnapshot: { preferences: "DesktopPreferencesV1" },
  DesktopPreferencesV1: { theme: "DesktopTheme", font_family: "DesktopFontFamily", motion: "DesktopMotion" },
  DesktopPreferencesPatchTheme: { value: "DesktopTheme" },
  DesktopPreferencesPatchFontFamily: { value: "DesktopFontFamily" },
  DesktopPreferencesPatchMotion: { value: "DesktopMotion" },
  ScanReport: { plan: "UntrustedPlan", health: "ScanHealth" },
  UntrustedPlan: { targets: ["UntrustedTarget"] },
  UntrustedTarget: {
    scope: "Scope", ecosystem: "Ecosystem", kind: "TargetKind",
    sizing_warnings: ["SizingWarning"], last_modified: "SystemTime",
    risk: "RiskLevel", evidence: ["Evidence"], intent: "CleanupIntent",
  },
  SizingWarning: { kind: "SizingWarningKind" },
  ScanHealth: { completeness: "ScanCompleteness", diagnostics: ["ScanDiagnostic"], totals: "ScanTotals" },
  ScanDiagnostic: { stage: "ScanDiagnosticStage", outcome: "ScanDiagnosticOutcome", process: "ScanProcessProbe" },
  ScanProcessProbe: { status: "ScanProcessStatus", stdout: "ScanProcessOutput", stderr: "ScanProcessOutput" },
  DesktopScanProgress: { phase: "ScanPhase", preview: "ScanPreviewSnapshot" },
  ScanPreviewSnapshot: { targets: ["ScanPreviewTarget"], totals: "ScanPreviewTotals" },
  ScanPreviewTarget: {
    scope: "Scope", ecosystem: "Ecosystem", kind: "TargetKind",
    sizing_warnings: ["SizingWarning"], last_modified: "SystemTime",
    risk: "RiskLevel", disposition: "ScanPreviewDisposition", evidence: ["Evidence"],
  },
  DesktopScanResultCompleted: { report: "ScanReport" },
  DryRunOutcome: { report: "ExecutionReport" },
  ExecutionReport: { failures: ["ActionFailure"], outcomes: ["TargetOutcome"], notes: ["ExecutionNote"], estimated_recoverable: "ScanTotals" },
  TargetOutcome: { action: "ActionKind", status: "OutcomeStatus", estimated_recoverable: "CapacityEstimate" },
  AnalyzeSnapshotV1: { root: "AnalyzeRootIdentity", nodes: ["AnalyzeNodeV1"], warnings: ["AnalyzeWarningV1"], completeness: "AnalyzeCompleteness" },
  AnalyzeNodeV1: { kind: "AnalyzeNodeKind", evidence: "AnalyzeEvidence", warnings: ["AnalyzeWarningClass"] },
  AnalyzeWarningV1: { class: "AnalyzeWarningClass" },
  DesktopAnalyzeProgress: { changed_nodes: ["AnalyzeNodeV1"] },
  DesktopAnalyzeResultCompleted: { snapshot: "AnalyzeSnapshotV1" },
  DesktopAnalyzeResultCanceled: { snapshot: "AnalyzeSnapshotV1" },
  AnalyzeTrashPreviewV1: { items: ["AnalyzeTrashItemV1"], refused: ["AnalyzeTrashRefusalV1"] },
  AnalyzeTrashItemV1: { kind: "AnalyzeNodeKind", evidence: "AnalyzeEvidence" },
  AnalyzeTrashRefusalV1: { reason_code: "AnalyzeTrashRefusalCode" },
  AnalyzeTrashReportV1: { report: "ExecutionReport" },
  SoftwareInventoryV1: { sources: ["SoftwareSourceEvidence"], entries: ["SoftwareEntryV1"] },
  SoftwareSourceEvidence: { source: "SoftwareSourceId", state: "SoftwareSourceState" },
  SoftwareEntryV1: {
    identity: "SoftwareIdentity", scope: "SoftwareScope", provenance: ["SoftwareSourceId"],
    eligibility: "SoftwareEligibility", size: "SoftwareSizeEvidence", last_used: "SoftwareLastUsedEvidence",
  },
  SoftwareEligibility: { state: "SoftwareEligibilityState", reason: "SoftwareEligibilityReason" },
  SoftwareLastUsedEvidence: { reason_code: "SoftwareLastUsedReason" },
  SoftwarePreviewV1: { selected: ["SoftwarePreviewItemV1"] },
  SoftwarePreviewItemV1: { identity: "SoftwareIdentity", action_class: "SoftwareActionClass", scope: "SoftwareScope", eligibility: "SoftwareEligibilityReason" },
  SoftwareExecutionReportV1: { outcomes: ["SoftwareActionOutcomeV1"] },
  SoftwareActionOutcomeV1: { outcome: "SoftwareExecutionOutcome", installed_state: "SoftwareInstalledState", reboot_evidence: "SoftwareRebootEvidence", error_code: "SoftwareAuditErrorCode" },
  DesktopSoftwarePreviewResult: { plan: "SoftwareSelectionPlanV1", preview: "SoftwarePreviewV1" },
  DesktopSoftwareUninstallResult: { report: "SoftwareExecutionReportV1" },
  DesktopSoftwareAuditResult: { recovered: ["SoftwareActionOutcomeV1"], records: ["SoftwareAuditRecordV1"] },
  SoftwareAuditRecordV1: {
    identity: "SoftwareIdentity", transition: "SoftwareAuditTransition", status_code: "SoftwareAuditStatusCode",
    error_code: "SoftwareAuditErrorCode", reboot_evidence: "SoftwareRebootEvidence",
    installed_state: "SoftwareInstalledState", requery_result: "SoftwareAuditRequeryResult", adapter_outcome: "SoftwareAdapterOutcome",
  },
  DesktopSoftwareInventoryResultCompleted: { inventory: "SoftwareInventoryV1" },
  DesktopSoftwareUpdatesResult: { updates: "SoftwareUpdatesV1" },
  SoftwareUpdatesV1Available: { rows: ["SoftwareUpdateRowV1"] },
  SoftwareUpdatesV1Unavailable: { reason_code: "SoftwareUpdatesReason" },
  SoftwareStartupListV1: { sources: ["SoftwareStartupSourceV1"], entries: ["SoftwareStartupEntryV1"] },
  SoftwareStartupSourceV1: { location: "SoftwareStartupLocation", state: "SoftwareSourceState" },
  SoftwareStartupEntryV1: {
    location: "SoftwareStartupLocation", scope: "SoftwareScope",
    state: "SoftwareStartupState", toggle: "SoftwareStartupToggle",
  },
  SoftwareStartupToggleReportV1: {
    outcome: "SoftwareSupportOutcomeCode", error_code: "SoftwareSupportErrorCode", entry: "SoftwareStartupEntryV1",
  },
  DesktopSoftwareLeftoversPreviewResultDiscovered: { preview: "SoftwareLeftoverPreviewV1" },
  DesktopSoftwareLeftoversPreviewResultPlanned: { plan: "SoftwareLeftoverPlanV1", preview: "SoftwareLeftoverPlanPreviewV1" },
  SoftwareLeftoverPreviewV1: { apps: ["SoftwareLeftoverAppV1"] },
  SoftwareLeftoverAppV1: { identity: "SoftwareIdentity", app_size: "SoftwareSizeEvidence", candidates: ["SoftwareLeftoverCandidateV1"] },
  SoftwareLeftoverCandidateV1: { origin: "SoftwareLeftoverOrigin", certainty: "SoftwareLeftoverCertainty", size: "SoftwareSizeEvidence" },
  SoftwareLeftoverPlanV1: { identity: "SoftwareIdentity" },
  SoftwareLeftoverPlanPreviewV1: { items: ["SoftwareLeftoverCandidateV1"] },
  DesktopSoftwareLeftoversResult: { report: "SoftwareLeftoverReportV1" },
  SoftwareLeftoverReportV1: { outcomes: ["SoftwareLeftoverOutcomeV1"] },
  SoftwareLeftoverOutcomeV1: {
    certainty: "SoftwareLeftoverCertainty", outcome: "SoftwareSupportOutcomeCode", error_code: "SoftwareSupportErrorCode",
  },
  DesktopOptimizeListResultCompleted: { entries: ["MaintenanceCatalogueEntryV1"] },
  MaintenanceCatalogueEntryV1: { action_class: "MaintenanceActionClass" },
  MaintenancePreviewV1: { action_class: "MaintenanceActionClass" },
  MaintenanceExecutionReportV1: { outcomes: ["MaintenanceActionOutcomeV1"] },
  MaintenanceActionOutcomeV1: { action_class: "MaintenanceActionClass", outcome: "MaintenanceExecutionOutcome", error_code: "OptimizeAuditErrorCode" },
  DesktopOptimizePreviewResult: { plan: "MaintenancePlanV1", preview: "MaintenancePreviewV1" },
  DesktopOptimizeRunResult: { report: "MaintenanceExecutionReportV1" },
  DesktopOptimizeAuditResult: { recovered: ["MaintenanceActionOutcomeV1"], records: ["OptimizeAuditRecordV1"] },
  OptimizeAuditRecordV1: {
    action_class: "MaintenanceActionClass", transition: "OptimizeAuditTransition",
    status_code: "OptimizeAuditStatusCode", error_code: "OptimizeAuditErrorCode",
    adapter_outcome: "OptimizeAdapterOutcome",
  },
  OptimizeAuditTransitionTerminal: { outcome: "MaintenanceExecutionOutcome" },
  StatusSnapshotV1: {
    cpu: "CpuAvailabilityV1",
    memory: "MemoryAvailabilityV1",
    volumes: "VolumesAvailabilityV1",
    network: "NetworkAvailabilityV1",
    power: "PowerAvailabilityV1",
    gpu: "GpuAvailabilityV1",
    thermal: "ThermalAvailabilityV1",
    processes: "ProcessesAvailabilityV1",
    unsupported_capabilities: ["UnsupportedCapabilityV1"],
  },
  CpuAvailabilityV1: { value: "CpuV1" },
  MemoryAvailabilityV1: { value: "MemoryV1" },
  VolumesAvailabilityV1: { value: "VolumesV1" },
  NetworkAvailabilityV1: { value: "NetworkV1" },
  PowerAvailabilityV1: { value: "PowerV1" },
  GpuAvailabilityV1: { value: "GpuV1" },
  ThermalAvailabilityV1: { value: "ThermalV1" },
  ProcessesAvailabilityV1: { value: "ProcessesV1" },
  VolumesV1: { items: ["VolumeV1"] },
  NetworkV1: { interfaces: ["NetworkInterfaceV1"] },
  GpuV1: { adapters: ["GpuAdapterV1"] },
  ThermalV1: { zones: ["ThermalZoneV1"] },
  ProcessesV1: { items: ["ProcessV1"] },
  DesktopStatusSnapshotResultCompleted: { snapshot: "StatusSnapshotV1" },
  HudStatusEventSnapshot: { snapshot: "StatusSnapshotV1" },
  StatusEventV1Started: { data: "StatusStartedV1" },
  StatusEventV1Snapshot: { data: "StatusSnapshotV1" },
  StatusEventV1TickSkipped: { data: "TickSkippedV1" },
  StatusEventV1Terminal: { data: "StatusTerminalV1" },
};

function addSample(samples, name, value) {
  if (value !== undefined) {
    const values = samples.get(name) ?? [];
    values.push(value);
    samples.set(name, values);
  }
}

function collectReferences(samples) {
  let changed = true;
  const seen = new Map();
  while (changed) {
    changed = false;
    for (const [owner, fields] of Object.entries(references)) {
      const ownerSamples = samples.get(owner) ?? [];
      const offset = seen.get(owner) ?? 0;
      for (const sample of ownerSamples.slice(offset)) {
        if (!sample || typeof sample !== "object" || Array.isArray(sample)) continue;
        for (const [field, target] of Object.entries(fields)) {
          const value = sample[field];
          if (Array.isArray(target)) {
            if (Array.isArray(value)) for (const item of value) addSample(samples, target[0], item);
          } else if (value !== null) {
            addSample(samples, target, value);
          }
        }
      }
      if (ownerSamples.length > offset) {
        seen.set(owner, ownerSamples.length);
        changed = true;
      }
    }
  }
}

function nullable(schema, values) {
  return values.some((value) => value === null)
    ? { anyOf: [schema, { type: "null" }] }
    : schema;
}

function inferValue(values) {
  const present = values.filter((value) => value !== undefined && value !== null);
  if (present.length === 0) return { type: "null" };
  const first = present[0];
  let schema;
  if (Array.isArray(first)) {
    schema = { type: "array", items: inferValue(present.flat()) };
  } else if (typeof first === "object") {
    schema = inferObject(present);
  } else if (typeof first === "string") {
    schema = { type: "string" };
  } else if (typeof first === "number") {
    schema = { type: Number.isInteger(first) ? "integer" : "number" };
  } else if (typeof first === "boolean") {
    schema = { type: "boolean" };
  } else {
    throw new Error(`unsupported fixture value: ${typeof first}`);
  }
  return nullable(schema, values);
}

function inferObject(objects, owner) {
  const keys = [...new Set(objects.flatMap((object) => Object.keys(object)))].sort();
  const properties = {};
  const required = [];
  for (const key of keys) {
    const values = objects.map((object) => object[key]);
    const target = references[owner]?.[key];
    if (target) {
      const name = Array.isArray(target) ? target[0] : target;
      const reference = { $ref: `#/$defs/${name}` };
      properties[key] = Array.isArray(target)
        ? { type: "array", items: reference }
        : nullable(reference, values);
    } else {
      properties[key] = inferValue(values);
    }
    if (values.every((value) => value !== undefined)) required.push(key);
  }
  return { type: "object", additionalProperties: false, properties, required };
}

function definitionFor(name, values, catalog) {
  if (catalog.string_enums[name]) {
    return { type: "string", enum: [...new Set(values)].sort() };
  }
  if (catalog.tagged_unions[name]) {
    return {
      oneOf: Object.keys(catalog.tagged_unions[name].variants).sort()
        .map((variant) => ({ $ref: `#/$defs/${variant}` })),
    };
  }
  const objects = values.filter((value) => value && typeof value === "object" && !Array.isArray(value));
  return objects.length > 0 ? inferObject(objects, name) : inferValue(values);
}

function stripInterface(source, name) {
  return source.replace(new RegExp(`export interface ${name} \\{[\\s\\S]*?\\n\\}\n\n`, "g"), "");
}

function setTagLiteral(source, name, tag, value) {
  const pattern = new RegExp(`(export interface ${name} \\{[\\s\\S]*?\\n\\s*${tag}:\\s*)string;`);
  if (!pattern.test(source)) throw new Error(`quicktype output is missing ${name}.${tag}`);
  return source.replace(pattern, `$1${JSON.stringify(value)};`);
}

async function generate() {
  const samples = new Map();
  for (const [name, files] of Object.entries(rootFixtures)) {
    for (const file of files) {
      addSample(samples, name, JSON.parse(await readFile(path.join(fixtureRoot, file), "utf8")));
    }
  }
  const catalog = JSON.parse(await readFile(path.join(fixtureRoot, "contract-variants.json"), "utf8"));
  const preferencePatches = JSON.parse(await readFile(path.join(fixtureRoot, "desktop-preferences-patches.json"), "utf8"));
  catalog.tagged_unions.DesktopPreferencesPatch = { tag: "field", variants: Object.fromEntries(
    preferencePatches.map((patch) => ["DesktopPreferencesPatch" + patch.field.split("_").map((part) => part[0].toUpperCase() + part.slice(1)).join(""), [patch]]),
  ) };
  for (const [name, values] of Object.entries(catalog.additional_samples)) {
    for (const value of values) addSample(samples, name, value);
  }
  for (const [name, values] of Object.entries(catalog.string_enums)) {
    for (const value of values) addSample(samples, name, value);
  }
  for (const [unionName, union] of Object.entries(catalog.tagged_unions)) {
    for (const [variantName, variantSamples] of Object.entries(union.variants)) {
      for (const value of variantSamples) {
        addSample(samples, unionName, value);
        addSample(samples, variantName, value);
      }
    }
  }
  collectReferences(samples);

  const definitions = {};
  for (const name of [...samples.keys()].sort()) {
    definitions[name] = { title: name, ...definitionFor(name, samples.get(name), catalog) };
  }
  const exportedNames = Object.keys(definitions).sort();
  const schema = {
    $schema: "http://json-schema.org/draft-07/schema#",
    type: "object",
    additionalProperties: false,
    properties: Object.fromEntries(exportedNames.map((name) => [name, { $ref: `#/$defs/${name}` }])),
    required: exportedNames,
    $defs: definitions,
  };

  const schemaInput = new JSONSchemaInput(new FetchingJSONSchemaStore());
  await schemaInput.addSource({ name: "GeneratedContract", schema: JSON.stringify(schema) });
  const inputData = new InputData();
  inputData.addInput(schemaInput);
  const result = await quicktype({
    inputData,
    lang: "typescript",
    rendererOptions: {
      "acronym-style": "original",
      "just-types": "true",
      "prefer-unions": "true",
    },
  });
  let body = `${result.lines.join("\n")}\n`;
  body = stripInterface(body, "GeneratedContract");
  for (const [name, union] of Object.entries(catalog.tagged_unions)) {
    const variants = Object.keys(union.variants).sort();
    const tag = union.tag ?? "type";
    if (variants.length === 1) {
      body = setTagLiteral(body, name, tag, union.variants[variants[0]][0][tag]);
      continue;
    }
    for (const variant of variants) {
      body = setTagLiteral(body, variant, tag, union.variants[variant][0][tag]);
    }
    body = stripInterface(body, name);
    body = body.replace(new RegExp(`export type ${name}Type = [^;]+;\\n\n`, "g"), "");
    body += `export type ${name} = ${variants.join(" | ")};\n`;
  }
  // quicktype may emit a merged leftover `V1` from mixed Status event `data` objects.
  body = stripInterface(body, "V1");
  // quicktype may also emit an unused merge of the two leftover `preview` shapes.
  body = stripInterface(body, "SoftwareLeftoverPPreviewV1");
  const header = [
    "// Generated by scripts/generate-types.mjs with quicktype-core from named JSON fixtures.",
    "// Runtime validation remains in contract.ts; do not edit this file by hand.",
    "",
  ].join("\n");
  return `${header}${canonicalizeNewlines(body)}`;
}

function canonicalizeNewlines(text) {
  return text.replace(/^\uFEFF/, "").replace(/\r\n/g, "\n");
}

const output = await generate();
if (check) {
  let committed;
  try {
    committed = await readFile(typesPath, "utf8");
  } catch (error) {
    if (error && error.code === "ENOENT") {
      console.error(`generated types file missing: ${typesPath}`);
      process.exit(1);
    }
    throw error;
  }
  if (canonicalizeNewlines(committed) !== output) {
    console.error("desktop/src/api/types.gen.ts does not match generate-types.mjs --stdout");
    process.exit(1);
  }
} else if (stdout) {
  process.stdout.write(output);
} else {
  await writeFile(typesPath, output, "utf8");
  console.log(`generated src/api/types.gen.ts from ${Object.values(rootFixtures).flat().length + 1} named fixture files`);
}
