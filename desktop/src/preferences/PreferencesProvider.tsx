import { useLayoutEffect, type ReactNode } from "react";
import { tauriDesktopFontsBridge, type DesktopFontsBridge, type DesktopPreferencesBridge } from "../api/bridge";
import { DesktopFontsContext } from "./fonts";
import { applyAppearance, useMediaPreference } from "./appearance";
import { DesktopPreferencesContext } from "./context";
import { useDesktopPreferences } from "./store";

export function PreferencesProvider({ bridge, fonts = tauriDesktopFontsBridge, children }: { bridge: DesktopPreferencesBridge; fonts?: DesktopFontsBridge; children: ReactNode }) {
  const store = useDesktopPreferences(bridge);
  const systemLight = useMediaPreference("(prefers-color-scheme: light)", store.preferences.theme === "system");
  const systemReduced = useMediaPreference("(prefers-reduced-motion: reduce)");
  useLayoutEffect(() => {
    applyAppearance(document.documentElement, store.preferences, systemLight, systemReduced);
  }, [store.preferences, systemLight, systemReduced]);
  return <DesktopFontsContext.Provider value={fonts}><DesktopPreferencesContext.Provider value={store}>{children}</DesktopPreferencesContext.Provider></DesktopFontsContext.Provider>;
}
