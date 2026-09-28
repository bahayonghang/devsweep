import { readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const source = readFileSync(path.join(root, "src/preferences/appearance.css"), "utf8");
const themeIds = JSON.parse(readFileSync(path.join(root, "src/api/fixtures/contract-variants.json"), "utf8")).string_enums.DesktopTheme;
const declarations = (body) => Object.fromEntries([...body.matchAll(/--([a-z-]+):\s*([^;]+);/g)].map((match) => [match[1], match[2].trim()]));
const base = declarations(source.match(/:root, \[data-palette-preview\] \{([^}]+)\}/)[1]);
const required = [
  "stage-canvas", "canvas", "text", "muted", "border", "raised", "shell-raised",
  "control-border", "selection-bg", "selection-text", "selected-bg", "selected-text",
  "disabled-text", "focus", "danger", "warning", "ok", "danger-surface", "ok-surface",
  "theme-accent", "accent", "accent-clean", "accent-software", "accent-optimize", "accent-analyze", "accent-status",
  "capsule-bg", "capsule-border", "capsule-active-bg", "capsule-active-text",
  "on-accent", "on-danger", "danger-fill", "danger-hover", "warning-text", "warning-surface", "warning-border",
  "popup", "popup-text", "popup-border", "titlebar-bg", "titlebar-text",
  "card", "card-border", "shell-surface", "shell-border", "shell-text", "shell-muted",
  "hud-canvas", "hud-text", "hud-muted", "hud-border", "backdrop", "shadow", "tile-alpha",
];
const surfaces = ["stage-canvas", "raised", "shell-raised", "capsule-bg"];
const accents = ["accent-clean", "accent-software", "accent-optimize", "accent-analyze", "accent-status"];
const pairs = [];
for (const foreground of ["text", "muted", "danger", "warning", "ok", ...accents]) {
  for (const background of surfaces) pairs.push([foreground, background, 4.5, "body, table, supporting view, or stage"]);
}
for (const [foreground, background] of [
  ["selection-text", "selection-bg"], ["selected-text", "selected-bg"], ["popup-text", "popup"],
  ["disabled-text", "raised"], ["disabled-text", "popup"], ["capsule-active-text", "capsule-active-bg"],
  ["titlebar-text", "titlebar-bg"], ["hud-text", "hud-canvas"], ["hud-muted", "hud-canvas"],
  ["danger", "danger-surface"], ["warning", "warning-surface"], ["warning-text", "warning-surface"],
  ["ok", "ok-surface"], ["on-danger", "danger-fill"], ["on-danger", "danger-hover"],
  ["stage-canvas", "accent-analyze"], ["stage-canvas", "warning"], ["stage-canvas", "muted"],
]) pairs.push([foreground, background, 4.5, "selection, status, dialog, HUD, or diagram label"]);
for (const accent of accents) pairs.push(["on-accent", accent, 4.5, "primary action label"]);
for (const background of [...surfaces, "popup", "selection-bg", "selected-bg"]) {
  pairs.push(["focus", background, 3, "focus outline adjacent surface"]);
}
for (const background of [...surfaces, "popup"]) pairs.push(["control-border", background, 3, "input or button boundary"]);
pairs.push(["warning-border", "warning-surface", 3, "warning boundary"]);
pairs.push(["warning-border", "stage-canvas", 3, "warning boundary outside"]);
pairs.push(["on-danger", "danger-fill", 3, "close-button inset focus on hover"]);

function luminance(hex) {
  if (!/^#[0-9a-f]{6}$/i.test(hex)) throw new Error("Expected opaque sRGB hex: " + hex);
  const channels = [1, 3, 5].map((offset) => parseInt(hex.slice(offset, offset + 2), 16) / 255)
    .map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}

const measurements = [];
const palettes = {};
for (const id of themeIds.filter((id) => id !== "system")) {
  const body = source.match(new RegExp(':root\\[data-theme="' + id + '"\\][^{]*\\{([^}]+)\\}'))?.[1];
  if (!body) throw new Error("Missing palette " + id);
  const direct = declarations(body);
  const tokens = { ...base, ...direct };
  for (const token of required) if (!tokens[token]) throw new Error(id + " missing " + token);
  function resolve(token, visited = new Set()) {
    if (visited.has(token)) throw new Error("Token cycle " + token);
    visited.add(token);
    const value = tokens[token];
    if (!value) throw new Error(id + " missing alias target " + token);
    const alias = value.match(/^var\(--([a-z-]+)\)$/);
    return alias ? resolve(alias[1], visited) : value;
  }
  palettes[id] = { direct_tokens: Object.keys(direct), resolved: Object.fromEntries(required.map((token) => [token, resolve(token)])) };
  for (const [foreground, background, minimum, context] of pairs) {
    const fg = resolve(foreground);
    const bg = resolve(background);
    const levels = [luminance(fg), luminance(bg)].sort((a, b) => b - a);
    const ratio = (levels[0] + 0.05) / (levels[1] + 0.05);
    measurements.push({ palette: id, foreground, background, foreground_hex: fg, background_hex: bg, context, minimum, ratio, pass: ratio >= minimum });
  }
}
const report = {
  method: "WCAG sRGB relative luminance; opaque declared token pairs in the listed CSS consumer contexts. Native rendering is a separate check.",
  source: "desktop/src/preferences/appearance.css",
  source_sha256: createHash("sha256").update(source).digest("hex"),
  theme_ids: themeIds,
  required_tokens: required,
  palettes,
  measurements,
  failures: measurements.filter((pair) => !pair.pass),
};
const output = JSON.stringify(report, null, 2) + "\n";
const outputFlag = process.argv.indexOf("--output");
if (outputFlag >= 0) writeFileSync(path.resolve(process.argv[outputFlag + 1]), output);
else process.stdout.write(output);
if (report.failures.length) process.exitCode = 1;
