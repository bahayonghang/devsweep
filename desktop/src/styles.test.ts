import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const styles = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");
const cleanStyles = readFileSync(resolve(process.cwd(), "src/modes/clean/styles.css"), "utf8");
const stageStyles = readFileSync(resolve(process.cwd(), "src/stage/styles.css"), "utf8");

describe("responsive scan workbench styles", () => {
  it("contains table overflow and an explicit narrow layout", () => {
    expect(styles).toContain(".table-frame { overflow: auto;");
    expect(styles).toContain("@media (max-width: 760px)");
    expect(styles).toContain("grid-template-columns: minmax(0, 1fr) 118px");
  });

  it("honors reduced motion without removing semantic progress", () => {
    expect(styles).toContain("@media (prefers-reduced-motion: reduce)");
    expect(styles).toContain("animation-duration: .01ms !important");
    expect(styles).not.toContain("linear-gradient");
    expect(styles).not.toContain("radial-gradient");
  });

  it("locks capsule tokens, target widths, focus, and high-contrast behavior", () => {
    for (const token of ["--stage-canvas", "--capsule-bg", "--capsule-border", "--capsule-active-bg", "--capsule-active-text", "--accent", "--focus"]) expect(styles).toContain(token);
    for (const width of ["max-width: 800px", "max-width: 520px", "min-width: 1440px"]) expect(styles).toContain(width);
    expect(styles).toContain("@media (forced-colors: active)");
    expect(styles).toContain("outline: 3px solid var(--focus)");
    expect(styles).toContain(".shell-brand-icon");
    expect(styles).toContain(".mode-tab[aria-selected=\"true\"] { color: var(--capsule-active-text); background: var(--capsule-active-bg); }");
    for (const removed of [".shell-sidebar", ".page-header", "--sidebar", "--canvas-clean", "--canvas-software", ".sweep-body"]) expect(styles).not.toContain(removed);
    expect(styles).toContain(".card { background: var(--card); border: 1px solid var(--card-border); border-radius: var(--radius-card); padding: 16px; }");
    expect(styles).not.toContain(".mode-capsule");
    expect(styles).not.toContain(".shell-more");
    expect(styles).not.toContain(".sweep-body-hero");
    expect(styles).not.toContain("--capsule-track");
    expect(styles).toContain(".stage-host { min-width: 0; display: flex; flex: 1; flex-direction: column; background: transparent; }");
    expect(styles).toContain('font-family: "Segoe UI Variable", "Segoe UI", system-ui, sans-serif;');
  });

  it("scrolls the pill capsule horizontally and shrinks the planet below 800px", () => {
    expect(styles).toMatch(/\.capsule \{[^}]*overflow-x: auto;[^}]*border-radius: 999px;/);
    const start = styles.indexOf("@media (max-width: 800px)");
    const next = styles.indexOf("@media", start + 1);
    const block = styles.slice(start, next === -1 ? undefined : next);
    expect(block).toContain(".capsule-bar");
    const stageBlock = stageStyles.slice(stageStyles.indexOf("@media (max-width: 800px)"));
    expect(stageBlock).toMatch(/--planet-size: 168px/);
  });

  it("keeps the planet decorative, bounded, and non-interactive", () => {
    expect(stageStyles).toMatch(/\.planet \{[^}]*pointer-events: none;/);
    expect(styles).not.toContain("backdrop-filter");
    expect(styles).not.toContain("filter: blur");
  });

  it("keeps shared controls, banners, badges, and dialogs on the dark canvas", () => {
    // Every light surface removed when the five modes moved to one grammar.
    // A reintroduced light fallback renders white on the dark canvas in the
    // four modes that have no .clean-mode override.
    for (const light of ["#fff0ef", "#ebf6ef", "#fff5d9", "#ffebe8", "#fff0e9", "#9fa9a4", "#4e5a54", "#6b7671", "#aeb9b3", "#f1f4f2"]) {
      expect(styles).not.toContain(light);
    }
    expect(styles).toMatch(/:root \{[^}]*color-scheme: dark;/);
    expect(styles).toContain(".secondary-button { color: var(--text); background: var(--raised); border-color: var(--border); }");
    expect(styles).toContain("color: var(--danger); background: var(--raised); border-bottom: 1px solid var(--danger);");
    expect(styles).toContain("dialog { width: min(520px, calc(100vw - 32px)); padding: 0; color: var(--text); background: var(--raised); border: 1px solid var(--border); border-radius: var(--radius-card); }");
    expect(styles).toContain(".risk-dangerous { color: var(--text); background: var(--danger); border-color: var(--danger); }");
  });

  it("draws every raised surface and control from the radius tokens", () => {
    // A literal radius on a raised surface is a second visual system. The
    // pill (999px) and circle (50%) shapes are primitives, not surface radii.
    const surfaceRadii = (styles.match(/border-radius: \d+px/g) ?? []).filter((radius) => radius !== "border-radius: 999px");
    expect(surfaceRadii).toEqual([]);
    for (const token of ["--radius-card", "--radius-control", "--radius-tile"]) expect(styles).toContain(`border-radius: var(${token})`);
  });

  it("gives every shared surface a forced-colors rule", () => {
    const start = styles.indexOf("@media (forced-colors: active)");
    const block = styles.slice(start);
    for (const selector of [".error-banner", "dialog", ".secondary-button", ".status-chip", ".badge", ".outcome-list", ".digest-line"]) {
      expect(block).toContain(selector);
    }
  });

  it("states the shared grammar once instead of patching it per mode", () => {
    for (const patch of [".clean-mode .secondary-button", ".clean-mode .error-banner", ".clean-mode dialog", ".clean-mode .dialog-warning", ".clean-mode .digest"]) {
      expect(styles).not.toContain(patch);
    }
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
