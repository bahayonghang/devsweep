import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const styles = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");

describe("responsive scan workbench styles", () => {
  it("contains table overflow and an explicit narrow layout", () => {
    expect(styles).toContain(".table-frame { overflow: auto;");
    expect(styles).toContain("@media (max-width: 760px)");
    expect(styles).toContain("grid-template-columns: minmax(0, 1fr) 118px");
  });

  it("honors reduced motion without removing semantic progress", () => {
    expect(styles).toContain("@media (prefers-reduced-motion: reduce)");
    expect(styles).toContain("animation-duration: .01ms !important");
    expect(styles).toContain(".shell-header::after { display: none;");
    expect(styles).not.toContain("linear-gradient");
    expect(styles).not.toContain("radial-gradient");
  });

  it("locks shell tokens, target widths, focus, and high-contrast behavior", () => {
    for (const token of ["--shell-deep", "--shell-surface", "--accent", "--focus"]) expect(styles).toContain(token);
    for (const width of ["max-width: 1024px", "max-width: 800px", "max-width: 520px", "min-width: 1440px"]) expect(styles).toContain(width);
    expect(styles).toContain("@media (forced-colors: active)");
    expect(styles).toContain("outline: 3px solid var(--focus)");
    expect(styles).toContain(".shell-brand-icon");
    expect(styles).toContain('font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;');
  });

  it("keeps motifs decorative, bounded, and non-interactive", () => {
    expect(styles).toContain(".shell-header::after");
    expect(styles).toContain("pointer-events: none");
    expect(styles).not.toContain("backdrop-filter");
    expect(styles).not.toContain("filter: blur");
  });
});
