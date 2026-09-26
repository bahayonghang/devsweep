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

export interface WindowControlBridge {
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  isMaximized(): Promise<boolean>;
  startDragging(): Promise<void>;
  close(): Promise<void>;
  onResized(handler: () => void): Promise<() => void>;
}

export const tauriWindowControlBridge: WindowControlBridge = {
  minimize: () => getCurrentWindow().minimize(),
  toggleMaximize: () => getCurrentWindow().toggleMaximize(),
  isMaximized: () => getCurrentWindow().isMaximized(),
  startDragging: () => getCurrentWindow().startDragging(),
  close: () => getCurrentWindow().close(),
  onResized: (handler) => getCurrentWindow().onResized(handler),
};

export type WindowAction = "minimize" | "toggleMaximize" | "startDragging" | "close";
export type WindowControlFailure = WindowAction | "state" | "subscription";

interface WindowControlState {
  readonly maximized: boolean | null;
  readonly pending: WindowAction | null;
  readonly error: WindowControlFailure | null;
}

interface WindowConnection {
  disposed: boolean;
  readSequence: number;
  subscribing: boolean;
  unlisten?: () => void;
}

/** Native state owns the icon. Each effect connection rejects late reads and listeners. */
export class WindowControlController {
  private state: WindowControlState = { maximized: null, pending: null, error: null };
  private connection: WindowConnection | null = null;
  private listeners = new Set<() => void>();

  constructor(private readonly bridge: WindowControlBridge) {}

  getSnapshot = (): WindowControlState => this.state;

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };

  private update(patch: Partial<WindowControlState>) {
    this.state = { ...this.state, ...patch };
    for (const listener of this.listeners) listener();
  }

  connect(): () => void {
    const connection: WindowConnection = { disposed: false, readSequence: 0, subscribing: false };
    this.connection = connection;
    this.update({ pending: null, error: null });
    void this.listen(connection);
    void this.refresh(connection);
    return () => {
      connection.disposed = true;
      connection.unlisten?.();
      if (this.connection === connection) this.connection = null;
    };
  }

  private async listen(connection: WindowConnection) {
    if (connection.subscribing || connection.unlisten || connection.disposed) return;
    connection.subscribing = true;
    try {
      const unlisten = await this.bridge.onResized(() => { void this.refresh(connection); });
      if (connection.disposed) unlisten();
      else {
        connection.unlisten = unlisten;
        if (this.state.error === "subscription") this.update({ error: null });
        // Capture any resize between the initial read and listener registration.
        void this.refresh(connection);
      }
    } catch {
      if (!connection.disposed) this.update({ error: "subscription" });
    } finally {
      connection.subscribing = false;
    }
  }

  private async refresh(connection: WindowConnection) {
    if (connection.disposed) return;
    const sequence = ++connection.readSequence;
    try {
      const maximized = await this.bridge.isMaximized();
      if (!connection.disposed && sequence === connection.readSequence) {
        this.update({ maximized, ...(this.state.error === "state" ? { error: null } : {}) });
      }
    } catch {
      if (!connection.disposed && sequence === connection.readSequence && this.state.error !== "subscription") {
        this.update({ error: "state" });
      }
    }
  }

  async perform(action: WindowAction): Promise<void> {
    const connection = this.connection;
    if (!connection || connection.disposed || this.state.pending) return;
    this.update({ pending: action, ...(this.state.error === action ? { error: null } : {}) });
    try {
      await this.bridge[action]();
      if (!connection.disposed && action === "toggleMaximize") await this.refresh(connection);
    } catch {
      if (!connection.disposed) this.update({ error: action });
    } finally {
      if (!connection.disposed) this.update({ pending: null });
    }
  }

  async retry(): Promise<void> {
    const connection = this.connection;
    if (!connection || connection.disposed) return;
    const error = this.state.error;
    if (error === "state") await this.refresh(connection);
    else if (error && error !== "subscription") await this.perform(error);
    // An action failure may follow a subscription failure. Repair both on retry.
    if (!connection.unlisten) await this.listen(connection);
  }
}

/** Browser fixtures have local window state and never call native window APIs. */
export function createFixtureWindowBridge(): WindowControlBridge & DesktopLifecycleBridge {
  let maximized = false;
  const resized = new Set<() => void>();
  const closing = new Set<() => Promise<void>>();
  return {
    async minimize() {},
    async toggleMaximize() {
      maximized = !maximized;
      for (const listener of resized) listener();
    },
    async isMaximized() { return maximized; },
    async startDragging() {},
    async close() { await Promise.all([...closing].map((handler) => handler())); },
    async onResized(handler) { resized.add(handler); return () => { resized.delete(handler); }; },
    async onCloseRequested(handler) { closing.add(handler); return () => { closing.delete(handler); }; },
    async nativeFaultMode() { return "disabled"; },
  };
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
