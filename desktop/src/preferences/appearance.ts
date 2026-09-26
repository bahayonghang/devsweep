import { useLayoutEffect, useState } from "react";
import type { DesktopPreferencesV1 } from "../api/types.gen";

export const FONT_STACKS = {
  system: '"Segoe UI Variable", system-ui, "Microsoft YaHei UI", sans-serif',
  segoe_ui: '"Segoe UI", "Microsoft YaHei UI", system-ui, sans-serif',
  microsoft_yahei_ui: '"Microsoft YaHei UI", "Segoe UI", system-ui, sans-serif',
} as const;

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

export function applyAppearance(root: HTMLElement, preferences: DesktopPreferencesV1, systemLight: boolean, systemReduced: boolean) {
  const theme = preferences.theme === "system" ? (systemLight ? "light" : "dark") : preferences.theme;
  root.dataset.theme = theme;
  root.dataset.motion = preferences.motion === "reduced" || systemReduced ? "reduced" : "system";
  root.style.colorScheme = theme;
  root.style.setProperty("--font-ui", FONT_STACKS[preferences.font_family]);
  root.style.setProperty("--text-scale", String(preferences.text_scale_percent / 100));
}
