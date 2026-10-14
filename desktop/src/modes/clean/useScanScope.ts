import { useEffect, useRef, useState } from "react";
import type { DesktopBridge } from "../../api/bridge";
import type { ScanOptions } from "../../api/types.gen";

/** A single in-flight edit owns validation and persistence. A late initial
 * read or old bridge result can never overwrite a newer local selection. */
export function useScanScope(bridge: DesktopBridge, invalidate: () => void) {
  const [options, setOptions] = useState<ScanOptions>({ include_projects: true, include_global: true, roots: [] });
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [loadedBridge, setLoadedBridge] = useState<DesktopBridge | null>(null);
  const loading = loadedBridge !== bridge;
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<unknown>(null);
  const [sessionOnly, setSessionOnly] = useState(false);
  const generation = useRef(0);
  const revision = useRef(0);
  const inFlight = useRef(false);

  useEffect(() => {
    const current = ++generation.current;
    const originalRevision = revision.current;
    inFlight.current = false;
    void bridge.cleanScanPreferencesGet().then((snapshot) => {
      if (current !== generation.current || originalRevision !== revision.current) return;
      setOptions((previous) => ({ ...previous, roots: snapshot.roots }));
      setSuggestions(snapshot.suggestions);
      setError(null);
      setSessionOnly(false);
    }).catch((failure: unknown) => {
      if (current === generation.current && originalRevision === revision.current) { setError(failure); setSessionOnly(true); }
    }).finally(() => {
      if (current === generation.current) { setLoadedBridge(bridge); if (!inFlight.current) setSaving(false); }
    });
    return () => { generation.current += 1; };
  }, [bridge]);

  const changeOptions = (next: ScanOptions) => {
    revision.current += 1;
    invalidate();
    setOptions(next);
  };

  const changeRoots = async (roots: string[]) => {
    if (inFlight.current) return false;
    inFlight.current = true;
    const current = generation.current;
    revision.current += 1;
    invalidate();
    setSaving(true);
    let valid = false;
    try {
      const snapshot = await bridge.cleanScanRootsValidate(roots);
      if (current !== generation.current) return false;
      valid = true;
      setOptions((previous) => ({ ...previous, roots: snapshot.roots }));
      setSuggestions(snapshot.suggestions);
      // A failed save leaves these validated roots usable for this session.
      await bridge.cleanScanPreferencesSet(snapshot.roots);
      if (current === generation.current) { setError(null); setSessionOnly(false); }
    } catch (failure) {
      if (current === generation.current) { setError(failure); if (valid) setSessionOnly(true); }
    } finally {
      if (current === generation.current) {
        inFlight.current = false;
        setSaving(false);
      }
    }
    return valid;
  };

  return { options, suggestions, loading, saving, error, sessionOnly, dismissError: () => setError(null), changeOptions, changeRoots };
}
