import { useEffect, useMemo, useSyncExternalStore, type MouseEvent } from "react";
import { message, type MessageKey, type PresentationLanguageTag } from "../i18n";
import { WindowControlController, type WindowControlBridge, type WindowControlFailure } from "../lifecycle";

const ERROR_KEYS: Record<WindowControlFailure, MessageKey> = {
  minimize: "window.v1.error.minimize",
  toggleMaximize: "window.v1.error.maximize",
  startDragging: "window.v1.error.drag",
  close: "window.v1.error.close",
  state: "window.v1.error.state",
  subscription: "window.v1.error.subscription",
};

function WindowGlyph({ kind }: { readonly kind: "minimize" | "maximize" | "restore" | "close" }) {
  return <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.25" aria-hidden="true" focusable="false">
    {kind === "minimize" && <path d="M3 8.5h10" />}
    {kind === "maximize" && <path d="M3.5 3.5h9v9h-9z" />}
    {kind === "restore" && <path d="M5.5 5.5h7v7h-7zM3.5 10.5v-7h7" />}
    {kind === "close" && <path d="m3.5 3.5 9 9m0-9-9 9" />}
  </svg>;
}

export function WindowTitlebar({ bridge, locale }: {
  readonly bridge: WindowControlBridge;
  readonly locale: PresentationLanguageTag | null;
}) {
  const controller = useMemo(() => new WindowControlController(bridge), [bridge]);
  const state = useSyncExternalStore(controller.subscribe, controller.getSnapshot);
  useEffect(() => controller.connect(), [controller]);
  const label = (key: MessageKey) => locale
    ? message(locale, key)
    : `${message("en", key)} / ${message("zh-CN", key)}`;
  const maximized = state.maximized === true;
  const maximizeLabel = label(maximized ? "window.v1.restore" : "window.v1.maximize");
  const busy = state.pending !== null;

  function onBlankMouseDown(event: MouseEvent<HTMLDivElement>) {
    if (event.button !== 0 || event.target !== event.currentTarget) return;
    event.preventDefault();
    if (event.detail === 2) void controller.perform("toggleMaximize");
    else void controller.perform("startDragging");
  }

  return <div className="window-chrome">
    <div className="window-titlebar">
      <div className="window-drag-region" onMouseDown={onBlankMouseDown}>DevSweep</div>
      <div className="window-controls" role="group" aria-label={label("window.v1.controls")} aria-busy={busy}>
        <button type="button" className="window-control" aria-label={label("window.v1.minimize")} title={label("window.v1.minimize")} disabled={busy} onClick={() => void controller.perform("minimize")}>
          <WindowGlyph kind="minimize" />
        </button>
        <button type="button" className="window-control" aria-label={maximizeLabel} title={maximizeLabel} disabled={busy || state.maximized === null} onClick={() => void controller.perform("toggleMaximize")}>
          <WindowGlyph kind={maximized ? "restore" : "maximize"} />
        </button>
        <button type="button" className="window-control window-control-close" aria-label={label("window.v1.close")} title={label("window.v1.close")} disabled={busy} onClick={() => void controller.perform("close")}>
          <WindowGlyph kind="close" />
        </button>
      </div>
    </div>
    {state.error && <div className="window-control-error" role="alert">
      <span>{label(ERROR_KEYS[state.error])}</span>
      <button type="button" className="secondary-button" disabled={busy} onClick={() => void controller.retry()}>{label("window.v1.retry")}</button>
    </div>}
  </div>;
}
