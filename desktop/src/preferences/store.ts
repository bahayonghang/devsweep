import { useCallback, useEffect, useRef, useState } from "react";
import type { DesktopPreferencesBridge } from "../api/bridge";
import { decodeDesktopPreferencesSnapshot } from "../api/contract";
import defaultSnapshot from "../api/fixtures/desktop-preferences.json";
import type { DesktopPreferencesPatch, DesktopPreferencesSnapshot } from "../api/types.gen";

export const DEFAULT_DESKTOP_PREFERENCES = decodeDesktopPreferencesSnapshot(defaultSnapshot).preferences;

export function useDesktopPreferences(bridge: DesktopPreferencesBridge) {
  const [state, setState] = useState({ preferences: DEFAULT_DESKTOP_PREFERENCES, loading: true, unavailable: false, saving: false, saveError: false });
  const generation = useRef(0);
  const committed = useRef<DesktopPreferencesSnapshot>({ sequence: 0, preferences: DEFAULT_DESKTOP_PREFERENCES });
  const committedBridge = useRef(bridge);
  const saving = useRef(false);
  const [reloadKey, setReloadKey] = useState(0);

  const accept = useCallback((snapshot: DesktopPreferencesSnapshot) => {
    if (snapshot.sequence < committed.current.sequence) return;
    committed.current = snapshot;
    setState((previous) => ({ ...previous, preferences: snapshot.preferences, loading: false }));
  }, []);

  useEffect(() => {
    const current = ++generation.current;
    if (committedBridge.current !== bridge) {
      committedBridge.current = bridge;
      committed.current = { sequence: 0, preferences: DEFAULT_DESKTOP_PREFERENCES };
    }
    saving.current = false;
    let unlisten: (() => void) | undefined;
    const unavailable = () => {
      if (generation.current === current) setState((previous) => ({ ...previous, loading: false, unavailable: true, saving: false }));
    };
    const read = async () => {
      try {
        const snapshot = await bridge.get();
        if (generation.current === current) {
          accept(snapshot);
          setState((previous) => ({ ...previous, unavailable: false }));
        }
      } catch { unavailable(); }
    };
    const initialize = async () => {
      try {
        const remove = await bridge.subscribe((snapshot) => {
          if (generation.current === current) accept(snapshot);
        }, unavailable);
        if (generation.current !== current) { remove(); return; }
        unlisten = remove;
        await read();
      } catch { unavailable(); }
    };
    void initialize();
    const onVisibility = () => { if (unlisten && !document.hidden) void read(); };
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      generation.current += 1;
      unlisten?.();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [bridge, reloadKey, accept]);

  const update = async (patch: DesktopPreferencesPatch): Promise<DesktopPreferencesSnapshot | null> => {
    if (state.loading || state.unavailable || saving.current) return null;
    const current = generation.current;
    saving.current = true;
    setState((previous) => ({ ...previous, saving: true, saveError: false }));
    try {
      const snapshot = await bridge.update(patch);
      if (generation.current !== current) return null;
      accept(snapshot);
      return committed.current;
    } catch {
      if (generation.current === current) setState((previous) => ({ ...previous, saveError: true }));
      return null;
    } finally {
      if (generation.current === current) {
        saving.current = false;
        setState((previous) => ({ ...previous, saving: false }));
      }
    }
  };
  return { ...state, update, reload: () => {
    setState((previous) => ({ ...previous, loading: true, saveError: false }));
    setReloadKey((value) => value + 1);
  } };
}

export type DesktopPreferencesState = ReturnType<typeof useDesktopPreferences>;
