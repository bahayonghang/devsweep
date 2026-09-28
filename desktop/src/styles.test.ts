import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { THEME_CATALOGUE } from "./preferences/appearance";
import { resolve } from "node:path";

const appearance = readFileSync(resolve(process.cwd(), "src/preferences/appearance.css"), "utf8");
const styles = appearance + readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");
const cleanStyles = readFileSync(resolve(process.cwd(), "src/modes/clean/styles.css"), "utf8");
const stageStyles = readFileSync(resolve(process.cwd(), "src/stage/styles.css"), "utf8");

describe("responsive scan workbench styles", () => {
  it("shares complete semantic palettes and scale-aware font sizes with the HUD", () => {
    const main = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");
    const hud = readFileSync(resolve(process.cwd(), "src/hud/hud.css"), "utf8");
    expect(main).toContain('@import "./preferences/appearance.css"');
    expect(hud).toContain('@import "../preferences/appearance.css"');
    expect(main).toContain("font-size: calc(14px * var(--text-scale))");
    expect(hud).toContain("font-size: calc(13px * var(--text-scale))");
    expect(hud).toContain("#root { height: 100%; overflow: auto; }");
  });
  it("contains table overflow and an explicit narrow layout", () => {
    expect(styles).toContain(".table-frame { overflow: auto;");
    expect(styles).toContain("@media (max-width: 760px)");
    expect(styles).toContain("grid-template-columns: minmax(0, 1fr) 118px");
  });

  it("measures every semantic palette and consumer contrast pair", () => {
    const report = JSON.parse(execFileSync(process.execPath, ["scripts/check-palette-contrast.mjs"], { encoding: "utf8" })) as {
      theme_ids: string[]; required_tokens: string[]; failures: unknown[];
      palettes: Record<string, { direct_tokens: string[]; resolved: Record<string, string> }>;
      measurements: { palette: string; foreground: string; background: string; minimum: number; ratio: number; pass: boolean }[];
    };
    expect(report.theme_ids).toEqual(Object.keys(THEME_CATALOGUE));
    expect(Object.keys(report.palettes)).toEqual(report.theme_ids.filter((id) => id !== "system"));
    expect(report.failures).toEqual([]);
    for (const [id, palette] of Object.entries(report.palettes)) {
      expect(palette.direct_tokens, id).toEqual(report.palettes.dark.direct_tokens);
      expect(Object.keys(palette.resolved), id).toEqual(report.required_tokens);
      const measured = report.measurements.filter((pair) => pair.palette === id);
      expect(measured.length).toBeGreaterThan(70);
      for (const pair of measured) expect(pair.ratio, JSON.stringify(pair)).toBeGreaterThanOrEqual(pair.minimum);
      for (const [foreground, background, minimum] of [
        ["text", "raised", 4.5],
        ["control-border", "raised", 3],
        ["control-border", "stage-canvas", 3],
      ] as const) {
        expect(measured, `${id} support dialog contrast`).toContainEqual(expect.objectContaining({ foreground, background, minimum, pass: true }));
      }
    }
    expect(report.palettes.catppuccin_latte.resolved).toMatchObject({ "stage-canvas": "#eff1f5", raised: "#e6e9ef", "shell-raised": "#dce0e8", text: "#4c4f69", muted: "#5c5f77", "theme-accent": "#1e66f5" });
    expect(report.palettes.catppuccin_mocha.resolved).toMatchObject({ "stage-canvas": "#1e1e2e", raised: "#181825", "shell-raised": "#313244", text: "#cdd6f4", muted: "#bac2de", "theme-accent": "#cba6f7" });
  });

  it("overrides all palette colors in forced colors after every preset", () => {
    const forcedStart = appearance.indexOf("@media (forced-colors: active)");
    const forced = appearance.slice(forcedStart);
    expect(forced).toContain(':root, :root[data-theme], [data-palette-preview]');
    const dark = appearance.match(/:root\[data-theme="dark"\][^{]*\{([^}]+)\}/)![1];
    const tokens = [...dark.matchAll(/--([a-z-]+):/g)].map((match) => match[1]);
    for (const token of tokens) expect(forced).toContain("--" + token + ":");
    expect(forced).not.toMatch(/#[a-f0-9]{3,8}|rgb\(/i);
    for (const theme of Object.values(THEME_CATALOGUE).filter((theme) => theme.id !== "system")) {
      expect(appearance.indexOf(':root[data-theme="' + theme.id + '"]')).toBeLessThan(forcedStart);
      expect(appearance).toContain('[data-palette-preview="' + theme.id + '"]');
    }
    expect(styles).toContain('background: var(--popup)');
    expect(styles).toContain('.settings-choice-item[data-selected] { color: var(--selected-text); background: var(--selected-bg);');
    expect(styles).not.toContain("--preview-canvas");
    const analyze = readFileSync(resolve(process.cwd(), "src/modes/analyze/styles.css"), "utf8");
    const analyzeForced = analyze.slice(analyze.indexOf("@media (forced-colors: active)"));
    expect(analyzeForced).toContain('.analyze-tile, .analyze-tile.evidence-incomplete, .analyze-tile.evidence-aggregated { fill: Canvas;');
    expect(analyzeForced).toContain('.analyze-tile.is-focused + text, .analyze-tile:focus-visible + text { fill: HighlightText; }');
  });

  it("uses palette tokens for support dialogs and system colors only in forced colors", () => {
    const support = readFileSync(resolve(process.cwd(), "src/support/styles.css"), "utf8");
    const forcedStart = support.indexOf("@media (forced-colors: active)");
    expect(forcedStart).toBeGreaterThan(-1);
    const dialog = support.slice(0, forcedStart).match(/\.support-page dialog\s*\{([^}]+)\}/)?.[1];
    expect(dialog).toBeDefined();
    expect(dialog).toContain("color: var(--text);");
    expect(dialog).toContain("background: var(--raised);");
    expect(dialog).toContain("border: 1px solid var(--control-border);");
    expect(dialog).not.toMatch(/\bCanvas(?:Text)?\b/);
    const forcedDialog = support.slice(forcedStart).match(/\.support-page dialog\s*\{([^}]+)\}/)?.[1];
    expect(forcedDialog).toBeDefined();
    expect(forcedDialog).toContain("color: CanvasText;");
    expect(forcedDialog).toContain("background: Canvas;");
    expect(forcedDialog).toContain("border-color: CanvasText;");
    expect(forcedDialog).toContain("forced-color-adjust: auto;");
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
    expect(styles).toContain("font-family: var(--font-ui);");
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
    expect(styles).toContain(".secondary-button { color: var(--text); background: var(--raised); border-color: var(--control-border); }");
    expect(styles).toContain("color: var(--danger); background: var(--raised); border-bottom: 1px solid var(--danger);");
    expect(styles).toContain("dialog { width: min(520px, calc(100vw - 32px)); padding: 0; color: var(--text); background: var(--raised); border: 1px solid var(--border); border-radius: var(--radius-card); }");
    expect(styles).toContain(".risk-dangerous { color: var(--on-danger); background: var(--danger-fill); border-color: var(--danger-fill); }");
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
    expect(styles).toContain(".capacity-total .display-capacity { font-size: clamp(2rem, calc(6vw * var(--text-scale)), 3.5rem);");
    expect(styles).not.toContain(".completed-empty h2 { color: #25352d;");
    expect(styles).not.toContain("background: #fff; border: 1px solid #c9d0cc;");
    expect(cleanStyles).toContain(".stage-headline { font-size: clamp(1.6rem, calc(3vw * var(--text-scale)), 2.25rem);");
    expect(cleanStyles).toContain(".clean-mode *, .clean-mode *::before, .clean-mode *::after { animation: none !important; transition: none !important; }");
    expect(cleanStyles).not.toContain("linear-gradient");
    expect(cleanStyles).not.toContain("radial-gradient");
    expect(cleanStyles).not.toContain("backdrop-filter");
    expect(cleanStyles).not.toContain("filter: blur");
    expect(cleanStyles).not.toContain(".sweep-body-hero");
  });
});
