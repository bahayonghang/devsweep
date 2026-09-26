import { useLayoutEffect, type ReactNode } from "react";
import type { DesktopPreferencesBridge } from "../api/bridge";
import { applyAppearance, useMediaPreference } from "./appearance";
import { DesktopPreferencesContext } from "./context";
import { useDesktopPreferences } from "./store";

export function PreferencesProvider({ bridge, children }: { bridge: DesktopPreferencesBridge; children: ReactNode }) {
  const store = useDesktopPreferences(bridge);
  const systemLight = useMediaPreference("(prefers-color-scheme: light)", store.preferences.theme === "system");
  const systemReduced = useMediaPreference("(prefers-reduced-motion: reduce)");
  useLayoutEffect(() => {
    applyAppearance(document.documentElement, store.preferences, systemLight, systemReduced);
  }, [store.preferences, systemLight, systemReduced]);
  return <DesktopPreferencesContext.Provider value={store}>{children}</DesktopPreferencesContext.Provider>;
}
