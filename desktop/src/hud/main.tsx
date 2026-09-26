import React from "react";
import ReactDOM from "react-dom/client";
import { listenHudStatus, tauriDesktopPreferencesBridge } from "../api/bridge";
import { decodeHudStatusEvent } from "../api/contract";
import fixtureStatus from "../api/fixtures/status/hud-snapshot.json";
import { PreferencesProvider } from "../preferences/PreferencesProvider";
import { PreferencesNotice } from "../preferences/SettingsPage";
import { usePreferences } from "../preferences/context";
import { createFixturePreferencesBridge } from "../preferences/fixture";
import {
  message,
  resolvePresentationLanguage,
  tauriPresentationSettingsBridge,
  type PresentationLanguageTag,
} from "../i18n";
import { Hud, type HudSubscribe } from "./Hud";
import "./hud.css";

function HudContent({ locale, subscribe }: { locale: PresentationLanguageTag; subscribe: HudSubscribe }) {
  const preferences = usePreferences();
  if (preferences.loading) return <p role="status">{message(locale, "shell.v1.store.loading")}</p>;
  return <><PreferencesNotice locale={locale} /><Hud locale={locale} subscribe={subscribe} /></>;
}

async function render() {
  const fixture = import.meta.env.DEV && import.meta.env.VITE_FIXTURE_BRIDGE === "1";
  const settings = fixture ? { language: "en" as const } : await tauriPresentationSettingsBridge
    .load()
    .catch(() => ({ language: null }));
  const locale = resolvePresentationLanguage(
    settings.language,
    globalThis.navigator?.languages ?? [],
  );
  document.documentElement.lang = locale;
  document.title = message(locale, "hud.v1.title");
  const preferences = fixture ? createFixturePreferencesBridge() : tauriDesktopPreferencesBridge;
  const subscribe: HudSubscribe = fixture ? async (onEvent) => {
    onEvent(decodeHudStatusEvent(fixtureStatus));
    return () => undefined;
  } : listenHudStatus;
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <PreferencesProvider bridge={preferences}><HudContent locale={locale} subscribe={subscribe} /></PreferencesProvider>
    </React.StrictMode>,
  );
}

void render();
