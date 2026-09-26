import type { DesktopPreferencesBridge } from "../api/bridge";
import { decodeDesktopPreferences } from "../api/contract";
import type { DesktopPreferencesSnapshot } from "../api/types.gen";
import { DEFAULT_DESKTOP_PREFERENCES } from "./store";

/** In-memory preview store. No native command or user settings file is used. */
export function createFixturePreferencesBridge(): DesktopPreferencesBridge {
  let snapshot: DesktopPreferencesSnapshot = { sequence: 1, preferences: { ...DEFAULT_DESKTOP_PREFERENCES } };
  const listeners = new Set<(snapshot: DesktopPreferencesSnapshot) => void>();
  return {
    get: async () => snapshot,
    update: async (patch) => {
      const changes = patch.field === "reset_appearance"
        ? { theme: DEFAULT_DESKTOP_PREFERENCES.theme, font_family: DEFAULT_DESKTOP_PREFERENCES.font_family, text_scale_percent: DEFAULT_DESKTOP_PREFERENCES.text_scale_percent }
        : patch.field === "reset_performance"
          ? { motion: DEFAULT_DESKTOP_PREFERENCES.motion, planet_fps: DEFAULT_DESKTOP_PREFERENCES.planet_fps, status_interval_seconds: DEFAULT_DESKTOP_PREFERENCES.status_interval_seconds, status_process_limit: DEFAULT_DESKTOP_PREFERENCES.status_process_limit, hud_interval_seconds: DEFAULT_DESKTOP_PREFERENCES.hud_interval_seconds }
          : { [patch.field]: patch.value };
      snapshot = { sequence: snapshot.sequence + 1, preferences: decodeDesktopPreferences({ ...snapshot.preferences, ...changes }) };
      listeners.forEach((listener) => listener(snapshot));
      return snapshot;
    },
    subscribe: async (listener) => { listeners.add(listener); return () => { listeners.delete(listener); }; },
  };
}
