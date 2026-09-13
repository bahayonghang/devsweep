import type { ReactNode } from "react";

export const GLYPH_NAMES = [
  "clean",
  "software",
  "optimize",
  "analyze",
  "status",
  "protection",
  "rules",
  "history",
  "language",
  "help",
  "build_artifacts",
  "dependency_directory",
  "package_cache",
  "test_cache",
  "tool_cache",
  "virtual_env",
  "docker",
  "generic",
  "node",
  "python",
  "rust",
  "risk",
] as const;

export type GlyphName = typeof GLYPH_NAMES[number];

function GlyphSvg({ children }: { readonly children: ReactNode }) {
  return <svg
    viewBox="0 0 16 16"
    width={16}
    height={16}
    fill="none"
    stroke="currentColor"
    strokeWidth={1.5}
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
    focusable="false"
  >{children}</svg>;
}

export function GlyphClean() {
  return <GlyphSvg>
    <path d="M11.5 2.5 L6 12" />
    <path d="M3.2 11.2 L6.2 12.4 L8.8 13.6" />
    <path d="M3 12.8 L6.4 13.2 L8.2 14.5" />
    <path d="M10.2 3.4 L12.2 4.6" />
  </GlyphSvg>;
}

export function GlyphSoftware() {
  return <GlyphSvg>
    <rect x="3" y="4" width="10" height="9" rx="1" />
    <path d="M3 7 H13" />
  </GlyphSvg>;
}

export function GlyphOptimize() {
  return <GlyphSvg>
    <path d="M3.2 11.5 A5.5 5.5 0 1 1 12.8 11.5" />
    <path d="M8 11 L11 7.2" />
    <circle cx="8" cy="11" r="0.8" />
  </GlyphSvg>;
}

export function GlyphAnalyze() {
  return <GlyphSvg>
    <path d="M8 14 V7" />
    <path d="M8 7 L4 3.6" />
    <path d="M8 7 L12 3.6" />
    <path d="M8 10 L5 13.2" />
    <path d="M8 10 L11 13.2" />
  </GlyphSvg>;
}

export function GlyphStatus() {
  return <GlyphSvg>
    <path d="M2 8 H4.8 L6.4 4.2 L9.4 12.2 L11 8 H14" />
  </GlyphSvg>;
}

export function GlyphProtection() {
  return <GlyphSvg>
    <path d="M8 2.6 L13 5 V8.4 C13 11.4 10.6 13.4 8 14.4 C5.4 13.4 3 11.4 3 8.4 V5 Z" />
  </GlyphSvg>;
}

export function GlyphRules() {
  return <GlyphSvg>
    <path d="M3 4 H4.6 M6.2 4 H13" />
    <path d="M3 8 H4.6 M6.2 8 H13" />
    <path d="M3 12 H4.6 M6.2 12 H13" />
  </GlyphSvg>;
}

export function GlyphHistory() {
  return <GlyphSvg>
    <circle cx="8" cy="8" r="5.5" />
    <path d="M8 5 V8.4 L10.4 10" />
  </GlyphSvg>;
}

export function GlyphLanguage() {
  return <GlyphSvg>
    <path d="M3 3.4 H13 V10.2 H7.6 L5 13.2 V10.2 H3 Z" />
  </GlyphSvg>;
}

export function GlyphHelp() {
  return <GlyphSvg>
    <path d="M6 5.4 C6 3.8 7.2 3 8.3 3 C9.6 3 10.6 3.9 10.6 5.3 C10.6 7.1 8 7.3 8 9.1" />
    <circle cx="8" cy="12.2" r="0.7" />
  </GlyphSvg>;
}

export function GlyphBuildArtifacts() {
  return <GlyphSvg>
    <rect x="2.5" y="3" width="11" height="4" />
    <rect x="2.5" y="9" width="5" height="4" />
    <rect x="8.5" y="9" width="5" height="4" />
  </GlyphSvg>;
}

export function GlyphDependencyDirectory() {
  return <GlyphSvg>
    <path d="M4 4.2 H7.2 L8.2 5.2" />
    <path d="M2.6 6 H6.2 L7.6 7.4 H13.4 V13 H2.6 Z" />
  </GlyphSvg>;
}

export function GlyphPackageCache() {
  return <GlyphSvg>
    <path d="M3 5.2 L8 2.6 L13 5.2 V11.4 L8 14 L3 11.4 Z" />
    <path d="M3 5.2 L8 7.6 L13 5.2" />
    <path d="M8 7.6 V14" />
  </GlyphSvg>;
}

export function GlyphTestCache() {
  return <GlyphSvg>
    <path d="M6 2.5 H10 M7.2 2.5 V6 L4.2 13.4 H11.8 L8.8 6 V2.5" />
    <path d="M5.4 10 H10.6" />
  </GlyphSvg>;
}

export function GlyphToolCache() {
  return <GlyphSvg>
    <path d="M4 12.4 L9.2 7.2" />
    <path d="M9.2 7.2 L10.6 5 C12.6 3.2 13.8 4.6 12.2 6.2 L10.8 7.6" />
    <path d="M3.4 11.4 L5.6 13.6" />
  </GlyphSvg>;
}

export function GlyphVirtualEnv() {
  return <GlyphSvg>
    <path d="M8 13.4 C4.2 13.4 3.2 8.2 8 2.6 C12.8 8.2 11.8 13.4 8 13.4 Z" />
    <path d="M8 13.2 V6.2" />
  </GlyphSvg>;
}

export function GlyphDocker() {
  return <GlyphSvg>
    <rect x="2.5" y="6.2" width="11" height="6" rx="0.6" />
    <path d="M4.6 6.2 V4.6 H6.6 V6.2 M8.2 6.2 V4.6 H10.2 V6.2" />
    <path d="M2.5 9.2 H13.5" />
  </GlyphSvg>;
}

export function GlyphGeneric() {
  return <GlyphSvg>
    <circle cx="8" cy="8" r="2.2" />
  </GlyphSvg>;
}

export function GlyphNode() {
  return <GlyphSvg>
    <path d="M8 2.6 L13 5.4 V10.6 L8 13.4 L3 10.6 V5.4 Z" />
  </GlyphSvg>;
}

export function GlyphPython() {
  return <GlyphSvg>
    <path d="M5 3.4 C8.2 3.4 8.2 7 11 7 C13.4 7 13.4 11.4 10.6 11.4 C8.2 11.4 8.2 8.6 5.2 8.6 C3.2 8.6 2.6 12.6 5.6 13.2" />
  </GlyphSvg>;
}

export function GlyphRust() {
  return <GlyphSvg>
    <circle cx="8" cy="8" r="2.4" />
    <path d="M8 2.6 V4.4 M8 11.6 V13.4 M2.6 8 H4.4 M11.6 8 H13.4 M4.2 4.2 L5.4 5.4 M10.6 10.6 L11.8 11.8 M11.8 4.2 L10.6 5.4 M5.4 10.6 L4.2 11.8" />
  </GlyphSvg>;
}

export function GlyphRisk() {
  return <GlyphSvg>
    <circle cx="8" cy="8" r="3.2" />
  </GlyphSvg>;
}

export const GLYPHS: Readonly<Record<GlyphName, () => ReactNode>> = {
  clean: GlyphClean,
  software: GlyphSoftware,
  optimize: GlyphOptimize,
  analyze: GlyphAnalyze,
  status: GlyphStatus,
  protection: GlyphProtection,
  rules: GlyphRules,
  history: GlyphHistory,
  language: GlyphLanguage,
  help: GlyphHelp,
  build_artifacts: GlyphBuildArtifacts,
  dependency_directory: GlyphDependencyDirectory,
  package_cache: GlyphPackageCache,
  test_cache: GlyphTestCache,
  tool_cache: GlyphToolCache,
  virtual_env: GlyphVirtualEnv,
  docker: GlyphDocker,
  generic: GlyphGeneric,
  node: GlyphNode,
  python: GlyphPython,
  rust: GlyphRust,
  risk: GlyphRisk,
};

export function DestinationGlyph({ name }: { readonly name: GlyphName }) {
  const Glyph = GLYPHS[name];
  return <Glyph />;
}
