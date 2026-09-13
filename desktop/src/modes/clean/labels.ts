import type { RiskLevel, TargetKind } from "../../api/types.gen";
import { message, type PresentationLanguageTag } from "../../i18n";

function assertNever(value: never): never {
  throw new Error(`unexpected value: ${JSON.stringify(value)}`);
}

export function kindLabel(locale: PresentationLanguageTag, kind: TargetKind): string {
  switch (kind) {
    case "build_artifacts": return message(locale, "clean.v1.kind.build_artifacts");
    case "dependency_directory": return message(locale, "clean.v1.kind.dependency_directory");
    case "package_cache": return message(locale, "clean.v1.kind.package_cache");
    case "test_cache": return message(locale, "clean.v1.kind.test_cache");
    case "tool_cache": return message(locale, "clean.v1.kind.tool_cache");
    case "virtual_env": return message(locale, "clean.v1.kind.virtual_env");
    default: return assertNever(kind);
  }
}

export function riskLabel(locale: PresentationLanguageTag, risk: RiskLevel): string {
  switch (risk) {
    case "dangerous": return message(locale, "clean.v1.risk.dangerous");
    case "high": return message(locale, "clean.v1.risk.high");
    case "low": return message(locale, "clean.v1.risk.low");
    case "medium": return message(locale, "clean.v1.risk.medium");
    default: return assertNever(risk);
  }
}

export function scopeLabel(locale: PresentationLanguageTag, scope: "project" | "global"): string {
  switch (scope) {
    case "project": return message(locale, "clean.v1.scope.projects");
    case "global": return message(locale, "clean.v1.scope.global");
    default: return assertNever(scope);
  }
}
