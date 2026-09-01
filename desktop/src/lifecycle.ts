import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { OperationCoordinator } from "./state/operation-coordinator";

export type NativeFaultMode = "disabled" | "route_cancel_once";

export interface ShellRouteCoordinator {
  cancelAndJoin(): Promise<void>;
}

export interface DesktopLifecycleBridge {
  onCloseRequested(handler: () => Promise<void>): Promise<() => void>;
  nativeFaultMode(): Promise<NativeFaultMode>;
}

export function decodeNativeFaultMode(value: unknown): NativeFaultMode {
  if (value === "disabled" || value === "route_cancel_once") return value;
  throw new Error("unsupported native fault mode");
}

export const tauriDesktopLifecycleBridge: DesktopLifecycleBridge = {
  async onCloseRequested(handler) {
    return getCurrentWindow().onCloseRequested(async () => {
      await handler();
    });
  },
  async nativeFaultMode() {
    if (!import.meta.env.DEV) return "disabled";
    try {
      return decodeNativeFaultMode(await invoke<unknown>("debug_native_fault_mode"));
    } catch {
      return "disabled";
    }
  },
};

/**
 * Owns the one application drain shared by native close and React unmount.
 * Cancellation failure is consumed only after OperationCoordinator.close has
 * joined the active work. The installed Tauri close listener owns window
 * destruction after this shared drain resolves.
 */
export class DesktopLifecycleController {
  private drainPromise: Promise<void> | null = null;

  constructor(private readonly coordinator: Pick<OperationCoordinator, "close">) {}

  drain(): Promise<void> {
    if (!this.drainPromise) {
      this.drainPromise = Promise.resolve()
        .then(() => this.coordinator.close())
        .catch(() => undefined);
    }
    return this.drainPromise;
  }

  requestClose(): Promise<void> {
    return this.drain();
  }
}

/** A one-shot presentation-only fault gate used solely by the debug harness. */
export class ShellRouteCoordinatorAdapter implements ShellRouteCoordinator {
  private rejectNextRoute = false;

  constructor(private readonly coordinator: ShellRouteCoordinator) {}

  setNativeFaultMode(mode: NativeFaultMode) {
    this.rejectNextRoute = import.meta.env.DEV && mode === "route_cancel_once";
  }

  async cancelAndJoin(): Promise<void> {
    if (this.rejectNextRoute) {
      this.rejectNextRoute = false;
      throw new Error("debug route cancellation fault");
    }
    await this.coordinator.cancelAndJoin();
  }
}
