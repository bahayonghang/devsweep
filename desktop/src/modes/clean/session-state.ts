import { cleanReducer, initialCleanState, type CleanAction, type CleanState } from "./reducer";

export type CleanScope = "global" | "project";
export interface CleanSlot { state: CleanState; revision: number; completedAt: string | null; stale: boolean }
export type CleanSession = Record<CleanScope, CleanSlot>;
export const initialCleanSession: CleanSession = {
  global: { state: initialCleanState, revision: 0, completedAt: null, stale: false },
  project: { state: initialCleanState, revision: 0, completedAt: null, stale: false },
};
export type SessionAction =
  | { type: "update"; scope: CleanScope; revision: number; change: CleanAction; completedAt?: string }
  | { type: "roots_changed" }
  | { type: "hidden" }
  | { type: "protections_changed" };

export function cleanSessionReducer(session: CleanSession, action: SessionAction): CleanSession {
  if (action.type === "hidden") {
    const settle = (slot: CleanSlot): CleanSlot => ({ ...slot, revision: slot.revision + 1,
      state: { ...slot.state, activeScan: null, pending: null,
        phase: slot.state.execution ? "reported" : slot.state.dryRun ? "dry_run" : slot.state.scan ? "reviewed" : "idle" } });
    return { global: settle(session.global), project: settle(session.project) };
  }
  if (action.type === "roots_changed") return { ...session, project: { ...initialCleanSession.project, revision: session.project.revision + 1 } };
  if (action.type === "protections_changed") {
    const invalidate = (slot: CleanSlot): CleanSlot => ({ ...slot, revision: slot.revision + 1, stale: slot.state.scan !== null,
      state: { ...slot.state, phase: slot.state.scan ? "reviewed" : "idle", activeScan: null, pending: null,
        selectedIds: new Set(), dryRun: null, execution: null } });
    return { global: invalidate(session.global), project: invalidate(session.project) };
  }
  const slot = session[action.scope];
  if (action.revision !== slot.revision) return session;
  const state = cleanReducer(slot.state, action.change);
  if (state === slot.state) return session;
  const completed = action.change.type === "scan_completed";
  return { ...session, [action.scope]: { ...slot, state,
    revision: slot.revision + (action.change.type === "scan_requested" ? 1 : 0),
    completedAt: completed ? action.completedAt ?? null : slot.completedAt,
    stale: completed ? false : slot.stale } };
}
