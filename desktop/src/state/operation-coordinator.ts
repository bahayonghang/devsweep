export type OperationKind = "clean.scan" | "clean.dry-run" | "clean.execute" | "software" | "optimize" | "analyze" | "status";

export interface CoordinatedOperation<T> {
  readonly kind: OperationKind;
  readonly id: string;
  readonly cancel: () => void | Promise<void>;
  readonly start: () => Promise<T>;
}

export interface CoordinatedLease<T> {
  readonly id: string;
  readonly result: Promise<T>;
  complete(): Promise<boolean>;
}

interface ActiveOperation {
  readonly kind: OperationKind;
  readonly id: string;
  readonly cancel: () => void | Promise<void>;
  readonly join: Promise<void>;
}

/**
 * Presentation lifecycle coordinator. It intentionally cannot receive plans,
 * digests, confirmation state, or mode-owned payloads.
 */
export class OperationCoordinator {
  private active: ActiveOperation | null = null;
  private transition: Promise<void> = Promise.resolve();
  private closed = false;

  start<T>(operation: CoordinatedOperation<T>): Promise<CoordinatedLease<T> | null> {
    return this.serialize(async () => {
      if (this.closed) return null;
      await this.stopActive();
      if (this.closed) return null;

      let releaseJoin: () => void = () => undefined;
      const join = new Promise<void>((resolve) => { releaseJoin = resolve; });
      const active: ActiveOperation = {
        kind: operation.kind,
        id: operation.id,
        cancel: operation.cancel,
        join,
      };
      this.active = active;

      let result: Promise<T>;
      try {
        result = Promise.resolve(operation.start());
      } catch (error) {
        if (this.active === active) this.active = null;
        releaseJoin();
        throw error;
      }
      void result.then(releaseJoin, releaseJoin);

      let completionRequested = false;
      return {
        id: operation.id,
        result,
        complete: async () => {
          if (completionRequested) return false;
          completionRequested = true;
          return this.complete(operation.id);
        },
      };
    });
  }

  complete(operationId: string): Promise<boolean> {
    return this.serialize(async () => {
      if (this.active?.id !== operationId) return false;
      const operation = this.active;
      this.active = null;
      await operation.join;
      return true;
    });
  }

  cancelAndJoin(): Promise<void> {
    return this.serialize(() => this.stopActive());
  }

  close(): Promise<void> {
    return this.serialize(async () => {
      this.closed = true;
      await this.stopActive();
    });
  }

  activeIdentity(): Readonly<Pick<ActiveOperation, "kind" | "id">> | null {
    return this.active ? { kind: this.active.kind, id: this.active.id } : null;
  }

  private serialize<T>(action: () => Promise<T>): Promise<T> {
    const result = this.transition.then(action, action);
    this.transition = result.then(() => undefined, () => undefined);
    return result;
  }

  private async stopActive(): Promise<void> {
    const operation = this.active;
    if (!operation) return;
    this.active = null;
    let cancellationError: unknown;
    try { await operation.cancel(); }
    catch (error) { cancellationError = error; }
    await operation.join;
    if (cancellationError) throw cancellationError;
  }
}
