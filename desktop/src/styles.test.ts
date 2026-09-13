import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const styles = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");
const cleanStyles = readFileSync(resolve(process.cwd(), "src/modes/clean/styles.css"), "utf8");

describe("responsive scan workbench styles", () => {
  it("contains table overflow and an explicit narrow layout", () => {
    expect(styles).toContain(".table-frame { overflow: auto;");
    expect(styles).toContain("@media (max-width: 760px)");
    expect(styles).toContain("grid-template-columns: minmax(0, 1fr) 118px");
  });

  it("honors reduced motion without removing semantic progress", () => {
    expect(styles).toContain("@media (prefers-reduced-motion: reduce)");
    expect(styles).toContain("animation-duration: .01ms !important");
    expect(styles).toContain(".sweep-body-ring { animation: none;");
    expect(styles).not.toContain("linear-gradient");
    expect(styles).not.toContain("radial-gradient");
  });

  it("locks shell tokens, target widths, focus, and high-contrast behavior", () => {
    for (const token of ["--shell-deep", "--shell-surface", "--canvas-clean", "--accent", "--focus"]) expect(styles).toContain(token);
    for (const width of ["max-width: 1024px", "max-width: 800px", "max-width: 520px", "min-width: 1440px"]) expect(styles).toContain(width);
    expect(styles).toContain("@media (forced-colors: active)");
    expect(styles).toContain("outline: 3px solid var(--focus)");
    expect(styles).toContain(".shell-brand-icon");
    expect(styles).toContain(".shell-sidebar");
    expect(styles).toContain(".page-header");
    expect(styles).toContain(".card { background: var(--card); border: 1px solid var(--card-border); border-radius: var(--radius-card); padding: 16px; }");
    expect(styles).not.toContain(".mode-capsule");
    expect(styles).not.toContain(".shell-more");
    expect(styles).not.toContain(".sweep-body-hero");
    expect(styles).not.toContain("--capsule-track");
    expect(styles).toContain(".mode-workbench { min-width: 0; display: flex; flex: 1; flex-direction: column; background: transparent; }");
    expect(styles).toContain('font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;');
  });

  it("turns the sidebar into a horizontal top strip below 800px", () => {
    const start = styles.indexOf("@media (max-width: 800px)");
    const next = styles.indexOf("@media", start + 1);
    const block = styles.slice(start, next === -1 ? undefined : next);
    expect(block).toContain("overflow-x: auto");
    expect(block).toContain("flex-direction: row");
    expect(block).toContain("min-width: max-content");
    expect(block).toContain("grid-template-columns: minmax(0, 1fr)");
  });

  it("keeps motifs decorative, bounded, and non-interactive", () => {
    expect(styles).toContain(".sweep-body");
    expect(styles).toContain("pointer-events: none");
    expect(styles).not.toContain("backdrop-filter");
    expect(styles).not.toContain("filter: blur");
  });

  it("restyles Clean tables and result capacity onto the dark canvas", () => {
    expect(styles).toContain(".table-frame { overflow: auto; background: var(--raised);");
    expect(styles).toContain(".capacity-total .display-capacity { font-size: clamp(2rem, 6vw, 3.5rem);");
    expect(styles).not.toContain(".completed-empty h2 { color: #25352d;");
    expect(styles).not.toContain("background: #fff; border: 1px solid #c9d0cc;");
    expect(cleanStyles).toContain(".stage-headline { font-size: clamp(1.6rem, 3vw, 2.25rem);");
    expect(cleanStyles).toContain(".clean-mode *, .clean-mode *::before, .clean-mode *::after { animation: none !important; transition: none !important; }");
    expect(cleanStyles).not.toContain("linear-gradient");
    expect(cleanStyles).not.toContain("radial-gradient");
    expect(cleanStyles).not.toContain("backdrop-filter");
    expect(cleanStyles).not.toContain("filter: blur");
    expect(cleanStyles).not.toContain(".sweep-body-hero");
  });
});
