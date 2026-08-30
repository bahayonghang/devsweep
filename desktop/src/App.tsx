import { useEffect, useMemo, useReducer, useRef, useState } from "react";
import type { CommandError, DesktopScanResult, DryRunOutcome, ExecutionReport, ScanOptions } from "./api/types.gen";
import { tauriBridge, type DesktopBridge } from "./api/bridge";
import { AppShell, type ModeRegistration } from "./app-shell";
import { decodeCommandError } from "./api/contract";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { ErrorBanner } from "./components/ErrorBanner";
import { ExecutePage } from "./pages/ExecutePage";
import { ReviewPage } from "./pages/ReviewPage";
import { ScanPage } from "./pages/ScanPage";
import { ScanPreviewPage } from "./pages/ScanPreviewPage";
import { appReducer, initialState } from "./state/app-state";
import { hasIrreversibleSelection } from "./state/selectors";
import {
  message,
  resolvePresentationLanguage,
  tauriPresentationSettingsBridge,
  type MessageKey,
  type PresentationLanguageTag,
  type PresentationSettingsBridge,
} from "./i18n";
import {
  DesktopLifecycleController,
  ShellRouteCoordinatorAdapter,
  tauriDesktopLifecycleBridge,
  type DesktopLifecycleBridge,
} from "./lifecycle";
import { OperationCoordinator } from "./state/operation-coordinator";

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected desktop error" }; }
}

let scanIdSequence = 0;

function createScanId(): string {
  scanIdSequence += 1;
  const randomId = globalThis.crypto?.randomUUID?.();
  return randomId ? `scan-${randomId}` : `scan-${Date.now()}-${scanIdSequence}`;
}

function CleanMode({ bridge, coordinator }: { bridge: DesktopBridge; coordinator: OperationCoordinator }) {
  const [state, dispatch] = useReducer(appReducer, initialState);
  const [options, setOptions] = useState<ScanOptions>({ include_projects: true, include_global: true, roots: ["."] });
  const mounted = useRef(true);

  useEffect(() => () => { mounted.current = false; }, []);

  const scan = async () => {
    if (state.phase === "scanning" || state.phase === "executing" || state.pending !== null) return;
    const scanId = createScanId();
    let progressError: CommandError | null = null;
    let started = false;
    try {
      const lease = await coordinator.start<DesktopScanResult>({
        kind: "clean.scan",
        id: scanId,
        cancel: () => bridge.scanCancel(scanId),
        start: () => {
          started = true;
          if (mounted.current) dispatch({ type: "scan_requested", scanId });
          return bridge.scanStart(
            scanId,
            options,
            (progress) => {
              if (mounted.current) dispatch({ type: "scan_progressed", progress });
            },
            (error) => {
              if (progressError) return;
              progressError = commandError(error);
              if (mounted.current) {
                dispatch({ type: "scan_cancel_requested", scanId });
                dispatch({ type: "command_failed", error: progressError });
              }
              void bridge.scanCancel(scanId).catch(() => undefined);
            },
          );
        },
      });
      if (!lease) return;
      try {
        const result = await lease.result;
        if (!mounted.current) return;
        if (progressError) dispatch({ type: "scan_failed", scanId, error: progressError });
        else if (result.type === "completed") dispatch({ type: "scan_completed", scanId, report: result.report });
        else dispatch({ type: "scan_canceled", scanId });
      } finally {
        await lease.complete().catch(() => false);
      }
    } catch (error) {
      if (started && mounted.current) {
        dispatch({ type: "scan_failed", scanId, error: progressError ?? commandError(error) });
      }
    }
  };
  const cancel = async () => {
    const scanId = state.activeScan?.scanId;
    if (!scanId) return;
    dispatch({ type: "scan_cancel_requested", scanId });
    try { await bridge.scanCancel(scanId); }
    catch (error) { dispatch({ type: "command_failed", error: commandError(error) }); }
  };
  const dryRun = async () => {
    if (!state.scan || state.selectedIds.size === 0) return;
    const plan = state.scan.plan;
    const selectedIds = [...state.selectedIds];
    const operationId = `dry-run-${Date.now()}`;
    let started = false;
    try {
      const lease = await coordinator.start<DryRunOutcome>({
        kind: "clean.dry-run",
        id: operationId,
        cancel: () => undefined,
        start: () => {
          started = true;
          if (mounted.current) dispatch({ type: "dry_run_requested" });
          return bridge.planDryRun(plan, selectedIds);
        },
      });
      if (!lease) return;
      try {
        const outcome = await lease.result;
        if (mounted.current) dispatch({ type: "dry_run_succeeded", outcome });
      } finally { await lease.complete().catch(() => false); }
    }
    catch (error) {
      if (started && mounted.current) dispatch({ type: "command_failed", error: commandError(error) });
    }
  };
  const execute = async () => {
    if (state.phase !== "confirming" || !state.scan || !state.dryRun) return;
    const plan = state.scan.plan;
    const selectedIds = [...state.selectedIds];
    const digest = state.dryRun.digest;
    const operationId = `execute-${Date.now()}`;
    let started = false;
    try {
      const lease = await coordinator.start<ExecutionReport>({
        kind: "clean.execute",
        id: operationId,
        cancel: () => undefined,
        start: () => {
          started = true;
          if (mounted.current) dispatch({ type: "execute_requested" });
          return bridge.planExecute(plan, selectedIds, digest);
        },
      });
      if (!lease) return;
      try {
        const report = await lease.result;
        if (mounted.current) dispatch({ type: "execute_succeeded", report });
      } finally { await lease.complete().catch(() => false); }
    }
    catch (error) {
      if (started && mounted.current) dispatch({ type: "command_failed", error: commandError(error) });
    }
  };

  const showDryRun = state.dryRun && ["dry_run", "confirming", "executing"].includes(state.phase);
  const preview = state.activeScan
    ? { status: "active" as const, progress: state.activeScan.progress, preview: state.activeScan.preview }
    : state.stoppedPreview
      ? { status: state.stoppedPreview.kind, progress: state.stoppedPreview.progress, preview: state.stoppedPreview.preview }
      : null;
  return <div className="clean-mode">
    <ScanPage activeScan={state.activeScan} busy={state.phase === "executing" || state.pending !== null} options={options} onOptions={setOptions} onScan={() => void scan()} onCancel={() => void cancel()} />
    {state.error && <ErrorBanner error={state.error} onDismiss={() => dispatch({ type: "error_dismissed" })} />}
    <div className="main-content">
      {state.execution && state.phase === "reported" ? <ExecutePage report={state.execution} final onConfirm={() => undefined} onReturn={() => dispatch({ type: "review_requested" })} />
        : showDryRun && state.dryRun ? <ExecutePage report={state.dryRun.report} final={false} onConfirm={() => dispatch({ type: "confirmation_opened" })} onReturn={() => dispatch({ type: "review_requested" })} />
        : preview ? <ScanPreviewPage status={preview.status} progress={preview.progress} preview={preview.preview} canReturnToReport={state.scan !== null} onReturnToReport={() => dispatch({ type: "stopped_preview_dismissed" })} />
        : <ReviewPage state={state} onSelect={(targetId, selected) => dispatch({ type: "selection_changed", targetId, selected })} onSelectAll={(selected) => dispatch({ type: "select_all_changed", selected })} onDryRun={() => void dryRun()} />}
    </div>
    <ConfirmDialog open={state.phase === "confirming" || state.phase === "executing"} digest={state.dryRun?.digest ?? ""} irreversible={hasIrreversibleSelection(state)} busy={state.phase === "executing"} onCancel={() => dispatch({ type: "confirmation_closed" })} onConfirm={() => void execute()} />
  </div>;
}

interface AppProps {
  bridge?: DesktopBridge;
  presentationSettings?: PresentationSettingsBridge;
  userLocales?: readonly string[];
  coordinator?: OperationCoordinator;
  lifecycle?: DesktopLifecycleBridge;
}

interface PresentationResourceIdentity {
  readonly settingsBridge: PresentationSettingsBridge;
  readonly userLocales: readonly string[];
}

type PresentationStoreState = PresentationResourceIdentity & (
  | { readonly status: "loading" }
  | { readonly status: "unavailable" }
  | {
    readonly status: "ready";
    readonly locale: PresentationLanguageTag;
    readonly saving: boolean;
    readonly saveError: boolean;
  }
);

const DEFAULT_USER_LOCALES = ["en"] as const;

function bilingualMessage(key: MessageKey): string {
  return `${message("en", key)} / ${message("zh-CN", key)}`;
}

function PresentationStoreGate({ state }: { readonly state: "loading" | "unavailable" }) {
  if (state === "loading") {
    return <main className="presentation-store-gate" aria-busy="true">
      <section className="presentation-store-panel">
        <p role="status">{bilingualMessage("shell.v1.store.loading")}</p>
      </section>
    </main>;
  }
  return <main className="presentation-store-gate">
    <section className="presentation-store-panel" role="alert" aria-labelledby="presentation-store-error-title">
      <h1 id="presentation-store-error-title">{bilingualMessage("shell.v1.store.unavailable.title")}</h1>
      <p>{message("en", "shell.v1.store.unavailable.detail")} {message("en", "shell.v1.store.unavailable.recovery")}</p>
      <p>{message("zh-CN", "shell.v1.store.unavailable.detail")}{message("zh-CN", "shell.v1.store.unavailable.recovery")}</p>
    </section>
  </main>;
}

export function App({
  bridge = tauriBridge,
  presentationSettings = tauriPresentationSettingsBridge,
  userLocales = globalThis.navigator?.languages ?? DEFAULT_USER_LOCALES,
  coordinator: coordinatorOverride,
  lifecycle = tauriDesktopLifecycleBridge,
}: AppProps) {
  const coordinator = useMemo(
    () => coordinatorOverride ?? new OperationCoordinator(),
    [coordinatorOverride],
  );
  const lifecycleController = useMemo(
    () => new DesktopLifecycleController(coordinator),
    [coordinator],
  );
  const shellCoordinator = useMemo(
    () => new ShellRouteCoordinatorAdapter(coordinator),
    [coordinator],
  );
  const [presentation, setPresentation] = useState<PresentationStoreState>(() => ({
    status: "loading",
    settingsBridge: presentationSettings,
    userLocales,
  }));
  const presentationGeneration = useRef(0);
  const saveInFlight = useRef(false);

  useEffect(() => {
    const generation = ++presentationGeneration.current;
    saveInFlight.current = false;
    void presentationSettings.load().then((settings) => {
      if (generation !== presentationGeneration.current) return;
      setPresentation({
        status: "ready",
        settingsBridge: presentationSettings,
        userLocales,
        locale: resolvePresentationLanguage(settings.language, userLocales),
        saving: false,
        saveError: false,
      });
    }).catch(() => {
      if (generation === presentationGeneration.current) {
        setPresentation({ status: "unavailable", settingsBridge: presentationSettings, userLocales });
      }
    });
    return () => {
      if (generation === presentationGeneration.current) presentationGeneration.current += 1;
      saveInFlight.current = false;
    };
  }, [presentationSettings, userLocales]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void lifecycle.onCloseRequested(() => lifecycleController.requestClose())
      .then((registeredUnlisten) => {
        if (disposed) registeredUnlisten();
        else unlisten = registeredUnlisten;
      })
      .catch(() => undefined);
    void lifecycle.nativeFaultMode()
      .then((mode) => {
        if (!disposed) shellCoordinator.setNativeFaultMode(mode);
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
      unlisten?.();
      void lifecycleController.drain();
    };
  }, [lifecycle, lifecycleController, shellCoordinator]);

  const changeLocale = async (next: PresentationLanguageTag) => {
    if (presentation.status !== "ready" || presentation.saving
      || presentation.locale === next || saveInFlight.current) return;
    const generation = presentationGeneration.current;
    const previousLocale = presentation.locale;
    saveInFlight.current = true;
    setPresentation({
      status: "ready",
      settingsBridge: presentationSettings,
      userLocales,
      locale: previousLocale,
      saving: true,
      saveError: false,
    });
    try {
      const stored = await presentationSettings.save(next);
      if (stored.language !== next) throw new Error("settings store returned a different language");
      if (generation !== presentationGeneration.current) return;
      setPresentation({
        status: "ready",
        settingsBridge: presentationSettings,
        userLocales,
        locale: next,
        saving: false,
        saveError: false,
      });
    } catch {
      if (generation !== presentationGeneration.current) return;
      setPresentation({
        status: "ready",
        settingsBridge: presentationSettings,
        userLocales,
        locale: previousLocale,
        saving: false,
        saveError: true,
      });
    } finally {
      if (generation === presentationGeneration.current) saveInFlight.current = false;
    }
  };

  if (presentation.settingsBridge !== presentationSettings || presentation.userLocales !== userLocales) {
    return <PresentationStoreGate state="loading" />;
  }
  if (presentation.status !== "ready") return <PresentationStoreGate state={presentation.status} />;

  const modes: readonly ModeRegistration[] = [
    { id: "clean", render: () => <CleanMode bridge={bridge} coordinator={coordinator} /> },
  ];
  return <AppShell
    modes={modes}
    locale={presentation.locale}
    onLocaleChange={changeLocale}
    coordinator={shellCoordinator}
    persistenceError={presentation.saveError}
    localeSaving={presentation.saving}
  />;
}
