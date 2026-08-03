import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles.css";

async function render() {
  const bridge = import.meta.env.DEV && import.meta.env.VITE_FIXTURE_BRIDGE === "1"
    ? (await import("./api/fixture-bridge")).fixtureBridge
    : undefined;
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode><App bridge={bridge} /></React.StrictMode>,
  );
}

void render();
