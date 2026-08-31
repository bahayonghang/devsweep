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

const rootFixtures = {
  ScanReport: ["scan-report.json", "scan-report.real.json", "clean/scan-report.json"],
  DesktopScanProgress: ["scan-progress.json", "clean/scan-progress.json"],
  DryRunOutcome: ["dry-run-outcome.json", "dry-run-outcome-two-targets.json", "clean/dry-run-outcome.json"],
  ExecutionReport: ["execution-report.json", "clean/execution-report.json"],
  AnalyzeSnapshotV1: ["analyze/snapshot.json"],
  DesktopAnalyzeProgress: ["analyze/progress.json"],
  SoftwareInventoryV1: ["software/inventory.json", "software/inventory-partial.json", "software/all-msi-manual.json"],
  DesktopSoftwarePreviewResult: ["software/preview.json"],
  DesktopSoftwareUninstallResult: ["software/execution-five-terminal.json"],
  DesktopSoftwareAuditResult: ["software/audit-restart.json"],
};

// This graph names nested Rust-owned DTOs. Fields, optionality, nullability,
// variants, and primitive shapes still come exclusively from fixture values.
const references = {
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
  const header = [
    "// Generated by scripts/generate-types.mjs with quicktype-core from named JSON fixtures.",
    "// Runtime validation remains in contract.ts; do not edit this file by hand.",
    "",
  ].join("\n");
  return `${header}${body.replace(/\r\n/g, "\n")}`;
}

const output = await generate();
if (stdout) {
  process.stdout.write(output);
} else {
  await writeFile(path.join(root, "src/api/types.gen.ts"), output, "utf8");
  console.log(`generated src/api/types.gen.ts from ${Object.values(rootFixtures).flat().length + 1} named fixture files`);
}
