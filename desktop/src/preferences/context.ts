import { createContext, useContext } from "react";
import { DEFAULT_DESKTOP_PREFERENCES, type DesktopPreferencesState } from "./store";

export const DesktopPreferencesContext = createContext<DesktopPreferencesState>({
  preferences: DEFAULT_DESKTOP_PREFERENCES, loading: false, unavailable: false, saving: false, saveError: false,
  update: async () => null, reload: () => undefined,
});
export const usePreferences = () => useContext(DesktopPreferencesContext);
