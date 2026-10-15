import { useEffect, useRef, useState } from "react";
import type { DesktopBridge } from "../../api/bridge";
import type { CleanAdvisoriesV1, ScanReport } from "../../api/types.gen";

const GIB = 1024 ** 3;
export const validThreshold = (value: number) => Number.isInteger(value) && value >= 1 && value <= 8388607;

/** Lives with the retained Clean session, including a failed-save session value. */
export function useAdvisoryPreference(bridge: DesktopBridge) {
  const [threshold, setThreshold] = useState(5);
  const [sessionOnly, setSessionOnly] = useState(false);
  const [saving, setSaving] = useState(false);
  const revision = useRef(0);
  const saveBusy = useRef(false);
  useEffect(() => {
    const id = ++revision.current;
    void bridge.cleanAdvisoryPreferencesGet().then(value => {
      if (revision.current === id) { setThreshold(value.threshold_bytes / GIB); setSessionOnly(false); }
    }).catch(() => { if (revision.current === id) setSessionOnly(true); });
    return () => { revision.current += 1; };
  }, [bridge]);
  const save = async (value: number) => {
    if (!validThreshold(value) || saveBusy.current) return;
    const id = ++revision.current;
    saveBusy.current = true;
    setThreshold(value);
    setSaving(true);
    try {
      await bridge.cleanAdvisoryPreferencesSet(value);
      if (revision.current === id) setSessionOnly(false);
    } catch { if (revision.current === id) setSessionOnly(true); }
    finally { saveBusy.current = false; if (revision.current === id) setSaving(false); }
  };
  return { threshold, sessionOnly, saving, save };
}

/** Advisory-only projection. A late threshold/report response cannot replace a newer one. */
export function useCleanAdvisory(bridge: DesktopBridge, report: ScanReport | null, threshold: number) {
  const [result, setResult] = useState<{ report: ScanReport; threshold: number; value: CleanAdvisoriesV1 | null; failed: boolean } | null>(null);
  useEffect(() => {
    if (!report) return;
    let current = true;
    void bridge.cleanAdvisories(report, threshold).then(value => {
      if (current) setResult({ report, threshold, value, failed: false });
    }).catch(() => { if (current) setResult({ report, threshold, value: null, failed: true }); });
    return () => { current = false; };
  }, [bridge, report, threshold]);
  return result?.report === report && result.threshold === threshold ? result : null;
}
