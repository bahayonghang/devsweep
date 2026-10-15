import { createContext, useContext, useReducer, type ReactNode } from "react";
import type { DesktopBridge } from "../../api/bridge";
import { cleanSessionReducer, initialCleanSession } from "./session-state";
import { useScanScope } from "./useScanScope";
import { useAdvisoryPreference } from "./useCleanAdvisory";

function useSession(bridge: DesktopBridge) {
  const [slots, dispatch] = useReducer(cleanSessionReducer, initialCleanSession);
  const scope = useScanScope(bridge, () => dispatch({ type: "roots_changed" }));
  const advisory = useAdvisoryPreference(bridge);
  return { slots, dispatch, scope, advisory };
}
const CleanSessionContext = createContext<ReturnType<typeof useSession> | null>(null);
export function CleanSessionProvider({ bridge, children }: { bridge: DesktopBridge; children: ReactNode }) {
  const session = useSession(bridge);
  return <CleanSessionContext.Provider value={session}>{children}</CleanSessionContext.Provider>;
}
export const useCleanSession = () => useContext(CleanSessionContext);
