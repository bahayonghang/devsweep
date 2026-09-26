import { PreferencesProvider } from "./preferences/PreferencesProvider";
import { PreferencesNotice } from "./preferences/SettingsPage";
import { usePreferences } from "./preferences/context";
import { tauriDesktopPreferencesBridge, type DesktopPreferencesBridge } from "./api/bridge";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { tauriBridge, type DesktopBridge } from "./api/bridge";
import { AppShell, shippedModeRegistrations, shippedSupportingRegistrations } from "./app-shell";
import { WindowTitlebar } from "./app-shell/WindowTitlebar";
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
  tauriWindowControlBridge,
  type DesktopLifecycleBridge,
  type WindowControlBridge,
} from "./lifecycle";
import { OperationCoordinator } from "./state/operation-coordinator";


interface AppProps {
  desktopPreferences?: DesktopPreferencesBridge;
  bridge?: DesktopBridge;
  presentationSettings?: PresentationSettingsBridge;
  userLocales?: readonly string[];
  coordinator?: OperationCoordinator;
  lifecycle?: DesktopLifecycleBridge;
  windowControls?: WindowControlBridge;
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

export function App(props: AppProps) {
  return <PreferencesProvider bridge={props.desktopPreferences ?? tauriDesktopPreferencesBridge}><AppContent {...props} /></PreferencesProvider>;
}

function AppContent({
  bridge = tauriBridge,
  presentationSettings = tauriPresentationSettingsBridge,
  userLocales = globalThis.navigator?.languages ?? DEFAULT_USER_LOCALES,
  coordinator: coordinatorOverride,
  lifecycle = tauriDesktopLifecycleBridge,
  windowControls = tauriWindowControlBridge,
}: AppProps) {
  const preferences = usePreferences();
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
  const lifecycleGeneration = useRef(0);
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
    const generations = lifecycleGeneration;
    const generation = ++generations.current;
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
      queueMicrotask(() => {
        // A replay increments the ref before this microtask, so this reads the latest value.
        if (generations.current === generation) {
          void lifecycleController.drain();
        }
      });
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

  const windowContent = (content: ReactNode, locale: PresentationLanguageTag | null = null) => <div className="desktop-window">
    <WindowTitlebar bridge={windowControls} locale={locale} />
    {content}
  </div>;

  if (presentation.settingsBridge !== presentationSettings || presentation.userLocales !== userLocales) {
    return windowContent(<PresentationStoreGate state="loading" />);
  }
  if (presentation.status !== "ready") return windowContent(<PresentationStoreGate state={presentation.status} />);

  if (preferences.loading) return windowContent(<PresentationStoreGate state="loading" />, presentation.locale);

  const registrationInput = {
    bridge,
    coordinator,
    locale: presentation.locale,
  };
  const modes = shippedModeRegistrations(registrationInput);
  const supporting = shippedSupportingRegistrations(registrationInput);
  return windowContent(<><PreferencesNotice locale={presentation.locale} /><AppShell
    modes={modes}
    supporting={supporting}
    locale={presentation.locale}
    onLocaleChange={changeLocale}
    coordinator={shellCoordinator}
    persistenceError={presentation.saveError}
    localeSaving={presentation.saving}
  /></>, presentation.locale);
}
