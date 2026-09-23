import React from "react";
import ReactDOM from "react-dom/client";
import { listenHudStatus } from "../api/bridge";
import {
  message,
  resolvePresentationLanguage,
  tauriPresentationSettingsBridge,
} from "../i18n";
import { Hud } from "./Hud";
import "./hud.css";

async function render() {
  const settings = await tauriPresentationSettingsBridge
    .load()
    .catch(() => ({ language: null }));
  const locale = resolvePresentationLanguage(
    settings.language,
    globalThis.navigator?.languages ?? [],
  );
  document.documentElement.lang = locale;
  document.title = message(locale, "hud.v1.title");
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <Hud locale={locale} subscribe={listenHudStatus} />
    </React.StrictMode>,
  );
}

void render();
