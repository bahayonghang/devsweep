import { createContext, useContext, useEffect, useMemo, useSyncExternalStore } from "react";
import { tauriDesktopFontsBridge, type DesktopFontsBridge } from "../api/bridge";
import type { FontFamily, FontsUnavailableReason } from "../api/types.gen";

export const DesktopFontsContext = createContext<DesktopFontsBridge>(tauriDesktopFontsBridge);
interface CatalogueState {
  families: readonly FontFamily[] | null;
  loading: boolean;
  failure: FontsUnavailableReason | null;
}

function createCatalogue(bridge: DesktopFontsBridge) {
  let snapshot: CatalogueState = { families: null, loading: false, failure: null };
  let generation = 0;
  let pending = false;
  let attempted = false;
  const listeners = new Set<() => void>();
  const publish = (next: CatalogueState) => { snapshot = next; listeners.forEach((listener) => listener()); };
  const refresh = async () => {
    if (pending) return;
    pending = true;
    attempted = true;
    const request = ++generation;
    publish({ ...snapshot, loading: true, failure: null });
    try {
      const result = await bridge.list();
      if (request !== generation || listeners.size === 0) return;
      publish(result.status === "available"
        ? { families: result.families, loading: false, failure: null }
        : { ...snapshot, loading: false, failure: result.reason });
    } catch {
      if (request === generation && listeners.size > 0) publish({ ...snapshot, loading: false, failure: "enumeration_failed" });
    } finally { if (request === generation) pending = false; }
  };
  return {
    getSnapshot: () => snapshot,
    refresh,
    load: () => { if (!attempted && snapshot.families === null) void refresh(); },
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
        // Effect replay can resubscribe in the same turn. A disposed request
        // cannot populate the session cache or replace a later request.
        queueMicrotask(() => {
          if (listeners.size === 0 && snapshot.loading) {
            generation += 1;
            pending = false;
            attempted = false;
            snapshot = { ...snapshot, loading: false };
          }
        });
      };
    },
  };
}

const catalogues = new WeakMap<DesktopFontsBridge, ReturnType<typeof createCatalogue>>();
export function useFontCatalogue() {
  const bridge = useContext(DesktopFontsContext);
  const catalogue = useMemo(() => {
    let cached = catalogues.get(bridge);
    if (!cached) { cached = createCatalogue(bridge); catalogues.set(bridge, cached); }
    return cached;
  }, [bridge]);
  const state = useSyncExternalStore(catalogue.subscribe, catalogue.getSnapshot);
  useEffect(() => { catalogue.load(); }, [catalogue]);
  return { ...state, refresh: catalogue.refresh };
}
