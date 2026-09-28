import { useLayoutEffect, useState } from "react";
import type { DesktopPreferencesV2, DesktopFont, DesktopTheme } from "../api/types.gen";
import type { MessageKey } from "../i18n";

export type PaletteId = Exclude<DesktopTheme, "system">;
type ThemeDefinition = { id: DesktopTheme; label: MessageKey; scheme: "light" | "dark" | "system"; preview: readonly PaletteId[] };

export const THEME_CATALOGUE = {
  dark: { id: "dark", label: "preferences.v1.theme.dark", scheme: "dark", preview: ["dark"] },
  light: { id: "light", label: "preferences.v1.theme.light", scheme: "light", preview: ["light"] },
  system: { id: "system", label: "preferences.v1.theme.system", scheme: "system", preview: ["dark", "light"] },
  catppuccin_latte: { id: "catppuccin_latte", label: "preferences.v2.theme.catppuccin_latte", scheme: "light", preview: ["catppuccin_latte"] },
  catppuccin_mocha: { id: "catppuccin_mocha", label: "preferences.v2.theme.catppuccin_mocha", scheme: "dark", preview: ["catppuccin_mocha"] },
  codex: { id: "codex", label: "preferences.v2.theme.codex", scheme: "dark", preview: ["codex"] },
  claude: { id: "claude", label: "preferences.v2.theme.claude", scheme: "light", preview: ["claude"] },
} as const satisfies Record<DesktopTheme, ThemeDefinition>;

export function resolvePalette(theme: DesktopTheme, systemLight: boolean): PaletteId {
  return theme === "system" ? (systemLight ? "light" : "dark") : theme;
}

export const SYSTEM_FONT_STACK = '"Segoe UI Variable", system-ui, "Microsoft YaHei UI", sans-serif';

/** Quote one validated family for a CSS property value, never stylesheet source. */
export function fontStack(font: DesktopFont): string {
  if (font.kind === "system") return SYSTEM_FONT_STACK;
  const family = font.family.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
  return `"${family}", ${SYSTEM_FONT_STACK}`;
}

export function useMediaPreference(query: string, enabled = true): boolean {
  const [matches, setMatches] = useState(() => globalThis.matchMedia?.(query)?.matches ?? false);
  useLayoutEffect(() => {
    if (!enabled) return;
    const media = globalThis.matchMedia?.(query);
    if (!media) return;
    const changed = () => setMatches(media.matches);
    changed();
    media.addEventListener?.("change", changed);
    return () => media.removeEventListener?.("change", changed);
  }, [query, enabled]);
  return enabled && matches;
}

export function applyAppearance(root: HTMLElement, preferences: DesktopPreferencesV2, systemLight: boolean, systemReduced: boolean) {
  const theme = resolvePalette(preferences.theme, systemLight);
  root.dataset.theme = theme;
  root.dataset.motion = preferences.motion === "reduced" || systemReduced ? "reduced" : "system";
  root.style.colorScheme = THEME_CATALOGUE[theme].scheme;
  root.style.setProperty("--font-ui", fontStack(preferences.font));
  root.style.setProperty("--text-scale", String(preferences.text_scale_percent / 100));
}
