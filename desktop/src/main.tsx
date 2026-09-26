import { createFixturePreferencesBridge } from "./preferences/fixture";
import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import type { PresentationLanguageTag, PresentationSettingsBridge } from "./i18n";
import { createFixtureWindowBridge } from "./lifecycle";
import "./styles.css";

async function render() {
  const fixture = import.meta.env.DEV && import.meta.env.VITE_FIXTURE_BRIDGE === "1";
  const bridge = fixture
    ? (await import("./api/fixture-bridge")).fixtureBridge
    : undefined;
  const desktopPreferences = fixture ? createFixturePreferencesBridge() : undefined;
  const fixtureWindow = fixture ? createFixtureWindowBridge() : undefined;
  let language: PresentationLanguageTag | null = "en";
  const presentationSettings: PresentationSettingsBridge | undefined = fixture
    ? {
        async load() {
          return { language };
        },
        async save(next) {
          language = next;
          return { language };
        },
      }
    : undefined;
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode><App desktopPreferences={desktopPreferences} bridge={bridge} presentationSettings={presentationSettings} userLocales={["en"]} lifecycle={fixtureWindow} windowControls={fixtureWindow} /></React.StrictMode>,
  );
}

void render();
