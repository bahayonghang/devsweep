import { createContext, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { createPortal } from "react-dom";
import iconUrl from "../assets/devsweep-icon-master.png";
import {
  message,
  uniqueAccelerators,
  type MessageKey,
  type PresentationLanguageTag,
} from "../i18n";
import type { ShellRouteCoordinator } from "../lifecycle";
import { DestinationGlyph, type GlyphName } from "./glyphs";

export const MODE_IDS = ["clean", "software", "optimize", "analyze", "status"] as const;
export type ModeId = typeof MODE_IDS[number];
export const SUPPORTING_DESTINATION_IDS = ["protection", "rules", "history"] as const;
export type SupportingDestinationId = typeof SUPPORTING_DESTINATION_IDS[number];
export type ShellRoute = `mode:${ModeId}` | `support:${SupportingDestinationId}` | "settings";

const MODE_MESSAGE_KEYS: Readonly<Record<ModeId, MessageKey>> = {
  clean: "command.clean",
  software: "command.software",
  optimize: "command.optimize",
  analyze: "command.analyze",
  status: "command.status",
};

const SUPPORTING_MESSAGE_KEYS: Readonly<Record<SupportingDestinationId, MessageKey>> = {
  protection: "shell.v1.supporting.protection",
  rules: "shell.v1.supporting.rules",
  history: "shell.v1.supporting.history",
};

const MODE_SUBTITLE_KEYS: Readonly<Record<ModeId, MessageKey>> = {
  clean: "shell.v1.subtitle.clean",
  software: "shell.v1.subtitle.software",
  optimize: "shell.v1.subtitle.optimize",
  analyze: "shell.v1.subtitle.analyze",
  status: "shell.v1.subtitle.status",
};

const SUPPORTING_SUBTITLE_KEYS: Readonly<Record<SupportingDestinationId, MessageKey>> = {
  protection: "shell.v1.subtitle.protection",
  rules: "shell.v1.subtitle.rules",
  history: "shell.v1.subtitle.history",
};

const PageHeaderSlotContext = createContext<HTMLElement | null>(null);

/** Mode pages render a chip into the shell page-header slot. A portal keeps the
 * chip out of shell state: writing the element into shell state made every mode
 * render produce a new element, which re-ran the effect, which re-rendered the
 * mode, until React aborted the tree (error #185). */
export function PageHeaderSlot({ children }: { readonly children?: ReactNode }) {
  const container = useContext(PageHeaderSlotContext);
  return container ? createPortal(children ?? null, container) : null;
}

function DestinationMark({ name }: { readonly name: GlyphName }) {
  return <span className="glyph-tile"><DestinationGlyph name={name} /></span>;
}

export interface ModeRegistration {
  readonly id: ModeId;
  readonly render: () => ReactNode;
}

export interface SupportingDestinationRegistration {
  readonly id: SupportingDestinationId;
  readonly render: () => ReactNode;
}

export interface AppShellProps {
  readonly modes: readonly ModeRegistration[];
  readonly supporting?: readonly SupportingDestinationRegistration[];
  readonly locale: PresentationLanguageTag;
  readonly onLocaleChange: (locale: PresentationLanguageTag) => void | Promise<void>;
  readonly coordinator: ShellRouteCoordinator;
  readonly persistenceError?: boolean;
  readonly localeSaving?: boolean;
}

type FocusRestoreIntent =
  | { readonly kind: "activator"; readonly element: HTMLElement }
  | { readonly kind: "route-control"; readonly route: ShellRoute };

interface PendingFocusRestore {
  readonly request: number;
  readonly route: ShellRoute;
  readonly intent: FocusRestoreIntent;
}

interface FailedFocusRestore {
  readonly request: number;
  readonly intent: FocusRestoreIntent;
}

function canRestoreFocus(element: HTMLElement | null): element is HTMLElement {
  if (!element?.isConnected || element.closest("[hidden], [aria-hidden='true']")) return false;
  if (element.matches(":disabled")) return false;
  const style = globalThis.getComputedStyle?.(element);
  return style?.display !== "none" && style?.visibility !== "hidden";
}

function revealFocusTarget(element: HTMLElement | null): HTMLElement | null {
  const disclosure = element?.closest("details");
  if (disclosure && !disclosure.open) disclosure.open = true;
  return element;
}

/** Visual ellipsis is allowed only for user data that remains available in
 * full to assistive technology and native hover text. */
export function AccessibleUserData({ value }: { readonly value: string }) {
  return <span className="user-data-ellipsis" title={value} aria-label={value}>{value}</span>;
}

/** CSS-native sweep body. Non-informational; still under reduced motion. Compact brand mark only. */
export function SweepBody() {
  return <span className="sweep-body" aria-hidden="true">
    <span className="sweep-body-ring" />
    <span className="sweep-body-core" />
  </span>;
}

export function parseModeRoute(hash: string, availableModes: readonly ModeId[]): ModeId | null {
  const route = parseShellRoute(hash, availableModes, []);
  return route?.startsWith("mode:") ? route.slice(5) as ModeId : null;
}

export function parseShellRoute(
  hash: string,
  availableModes: readonly ModeId[],
  availableSupporting: readonly SupportingDestinationId[],
): ShellRoute | null {
  if (hash === "#/settings") return "settings";
  const match = /^#\/(clean|software|optimize|analyze|status|protection|rules|history)$/.exec(hash);
  const id = match?.[1];
  if (id && (MODE_IDS as readonly string[]).includes(id)) {
    return availableModes.includes(id as ModeId) ? `mode:${id as ModeId}` : null;
  }
  return id && availableSupporting.includes(id as SupportingDestinationId)
    ? `support:${id as SupportingDestinationId}`
    : null;
}

function routeHash(route: ShellRoute): string {
  return `#/${route === "settings" ? "settings" : route.slice(route.indexOf(":") + 1)}`;
}

export function AppShell({
  modes,
  supporting = [],
  locale,
  onLocaleChange,
  coordinator,
  persistenceError = false,
  localeSaving = false,
}: AppShellProps) {
  if (modes.length === 0) throw new Error("the app shell requires at least one available mode");
  const availableIds = modes.map((mode) => mode.id);
  const availableSupporting = supporting.map((destination) => destination.id);
  const availableRoutes: readonly ShellRoute[] = [
    ...availableIds.map((id): ShellRoute => `mode:${id}`),
    ...availableSupporting.map((id): ShellRoute => `support:${id}`),
    "settings",
  ];
  const requestedInitialRoute = parseShellRoute(globalThis.location?.hash ?? "", availableIds, availableSupporting);
  const initial = requestedInitialRoute ?? `mode:${availableIds[0]}`;
  const [activeRoute, setActiveRoute] = useState<ShellRoute>(initial);
  const [transitioning, setTransitioning] = useState(false);
  const [routeError, setRouteError] = useState(false);
  const [focusCommitRequest, setFocusCommitRequest] = useState(0);
  const [headerSlot, setHeaderSlot] = useState<HTMLElement | null>(null);
  const contentHeading = useRef<HTMLHeadingElement>(null);
  const navButtons = useRef(new Map<ModeId, HTMLButtonElement>());
  const supportingButtons = useRef(new Map<SupportingDestinationId, HTMLButtonElement>());
  const settingsButton = useRef<HTMLButtonElement>(null);
  const settingsOpener = useRef<HTMLElement | null>(null);
  const pendingFocusRestore = useRef<PendingFocusRestore | null>(null);
  const failedFocusRestore = useRef<FailedFocusRestore | null>(null);
  const requestSequence = useRef(0);
  const observedHash = useRef(globalThis.location?.hash ?? "");
  const initialHashWasRegistered = useRef(requestedInitialRoute !== null);
  const initialFallbackRoute = useRef(initial);
  const initialFallbackHash = useRef(routeHash(initial));
  const activeMode = activeRoute.startsWith("mode:") ? activeRoute.slice(5) as ModeId : null;
  const activeRegistration = activeMode ? modes.find((mode) => mode.id === activeMode) : null;
  const activeSupportingId = activeRoute.startsWith("support:") ? activeRoute.slice(8) as SupportingDestinationId : null;
  const activeSupporting = activeSupportingId ? supporting.find((destination) => destination.id === activeSupportingId) : null;
  const messageKeys = useMemo(() => modes.map((mode) => MODE_MESSAGE_KEYS[mode.id]), [modes]);
  const accelerators = uniqueAccelerators(locale, messageKeys);

  useEffect(() => {
    const handleHistoryNavigation = () => {
      if (window.location.hash === observedHash.current) return;
      observedHash.current = window.location.hash;
      const routed = parseShellRoute(window.location.hash, availableIds, availableSupporting);
      const route = routed ?? `mode:${availableIds[0]}`;
      const intent: FocusRestoreIntent = routed && activeRoute === "settings" && settingsOpener.current
        ? { kind: "activator", element: settingsOpener.current }
        : { kind: "route-control", route };
      void navigate(route, routed ? "none" : "replace", intent);
    };
    window.addEventListener("popstate", handleHistoryNavigation);
    window.addEventListener("hashchange", handleHistoryNavigation);
    return () => {
      window.removeEventListener("popstate", handleHistoryNavigation);
      window.removeEventListener("hashchange", handleHistoryNavigation);
    };
  });

  useEffect(() => () => { requestSequence.current += 1; }, []);

  useEffect(() => {
    if (initialHashWasRegistered.current) return;
    const hash = initialFallbackHash.current;
    observedHash.current = hash;
    window.history.replaceState({ route: initialFallbackRoute.current }, "", hash);
  }, []);

  useLayoutEffect(() => {
    const pending = pendingFocusRestore.current;
    if (!pending || pending.route !== activeRoute || pending.request !== requestSequence.current) return;
    pendingFocusRestore.current = null;
    let target: HTMLElement | null = null;
    if (pending.intent.kind === "activator") target = pending.intent.element;
    else if (pending.intent.route.startsWith("mode:")) {
      target = navButtons.current.get(pending.intent.route.slice(5) as ModeId) ?? null;
    } else if (pending.intent.route.startsWith("support:")) {
      target = supportingButtons.current.get(pending.intent.route.slice(8) as SupportingDestinationId) ?? null;
    } else target = settingsButton.current;
    target = revealFocusTarget(target);
    if (canRestoreFocus(target)) target.focus();
    else contentHeading.current?.focus();
  }, [activeRoute, focusCommitRequest]);

  useLayoutEffect(() => {
    const failed = failedFocusRestore.current;
    if (transitioning || !routeError || !failed || failed.request !== requestSequence.current) return;
    failedFocusRestore.current = null;
    const target = revealFocusTarget(failed.intent.kind === "activator"
      ? failed.intent.element
      : failed.intent.route.startsWith("mode:")
        ? navButtons.current.get(failed.intent.route.slice(5) as ModeId) ?? null
        : failed.intent.route.startsWith("support:")
          ? supportingButtons.current.get(failed.intent.route.slice(8) as SupportingDestinationId) ?? null
          : settingsButton.current);
    if (canRestoreFocus(target)) target.focus();
    else contentHeading.current?.focus();
  }, [routeError, transitioning]);

  useEffect(() => {
    if (!availableRoutes.includes(activeRoute)) {
      const route: ShellRoute = `mode:${availableIds[0]}`;
      void navigate(route, "replace", { kind: "route-control", route });
    }
  });

  useEffect(() => {
    const handleAccelerator = (event: globalThis.KeyboardEvent) => {
      if (!event.altKey || event.ctrlKey || event.metaKey) return;
      const target = modes.find((mode) => accelerators.get(MODE_MESSAGE_KEYS[mode.id]) === event.key.toLowerCase());
      if (!target) return;
      event.preventDefault();
      const route: ShellRoute = `mode:${target.id}`;
      const element = navButtons.current.get(target.id);
      void navigate(route, "push", element
        ? { kind: "activator", element }
        : { kind: "route-control", route });
    };
    window.addEventListener("keydown", handleAccelerator);
    return () => window.removeEventListener("keydown", handleAccelerator);
  });

  async function navigate(
    route: ShellRoute,
    history: "push" | "replace" | "none",
    focus: FocusRestoreIntent,
  ) {
    const canonicalReplace = history === "replace" && window.location.hash !== routeHash(route);
    if ((route === activeRoute && !canonicalReplace) || !availableRoutes.includes(route)) return;
    const request = ++requestSequence.current;
    failedFocusRestore.current = null;
    setTransitioning(true);
    try {
      await coordinator.cancelAndJoin();
      if (request !== requestSequence.current) return;
      setRouteError(false);
      if (route === "settings" && focus.kind === "activator") settingsOpener.current = focus.element;
      pendingFocusRestore.current = { request, route, intent: focus };
      setActiveRoute(route);
      const hash = routeHash(route);
      observedHash.current = hash;
      if (history === "push") window.history.pushState({ route }, "", hash);
      else if (history === "replace") window.history.replaceState({ route }, "", hash);
      setFocusCommitRequest(request);
    } catch {
      if (request === requestSequence.current) {
        pendingFocusRestore.current = null;
        failedFocusRestore.current = { request, intent: focus };
        if (history !== "push") {
          const hash = routeHash(activeRoute);
          observedHash.current = hash;
          window.history.replaceState({ route: activeRoute }, "", hash);
        }
        setRouteError(true);
      }
    } finally {
      if (request === requestSequence.current) setTransitioning(false);
    }
  }

  function moveFocus(event: KeyboardEvent<HTMLElement>, current: ModeId) {
    const index = availableIds.indexOf(current);
    let next = index;
    if (event.key === "ArrowRight" || event.key === "ArrowDown") next = (index + 1) % availableIds.length;
    else if (event.key === "ArrowLeft" || event.key === "ArrowUp") next = (index - 1 + availableIds.length) % availableIds.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = availableIds.length - 1;
    else return;
    event.preventDefault();
    navButtons.current.get(availableIds[next])?.focus();
  }

  const activeLabel = activeRegistration
    ? message(locale, MODE_MESSAGE_KEYS[activeRegistration.id])
    : activeSupporting
      ? message(locale, SUPPORTING_MESSAGE_KEYS[activeSupporting.id])
      : message(locale, "shell.v1.settings.title");
  const activeSubtitle = activeRegistration
    ? message(locale, MODE_SUBTITLE_KEYS[activeRegistration.id])
    : activeSupporting
      ? message(locale, SUPPORTING_SUBTITLE_KEYS[activeSupporting.id])
      : message(locale, "shell.v1.subtitle.settings");
  const headingId = "mode-heading";
  const canvasMode = activeMode ?? "shell";
  const productTitle = message(locale, "app.title");
  return <PageHeaderSlotContext.Provider value={headerSlot}>
    <div className="app-shell" data-locale={locale} data-mode={canvasMode}>
      <aside className="shell-sidebar" aria-label={productTitle}>
        <div className="shell-brand">
          <SweepBody />
          <img src={iconUrl} alt="" aria-hidden="true" className="shell-brand-icon" />
          <span className="shell-brand-name">{productTitle}</span>
        </div>
        <p className="shell-section-label">{message(locale, "shell.v1.section.modes")}</p>
        <nav
          className="mode-navigation"
          role="tablist"
          aria-orientation="vertical"
          aria-label={`${productTitle} ${message(locale, "shell.v1.workbench")}`}
        >
          {modes.map((mode) => {
            const key = MODE_MESSAGE_KEYS[mode.id];
            const accelerator = accelerators.get(key);
            const label = message(locale, key);
            return <button
              key={mode.id}
              ref={(element) => { if (element) navButtons.current.set(mode.id, element); else navButtons.current.delete(mode.id); }}
              type="button"
              className="mode-tab"
              role="tab"
              aria-selected={activeRoute === `mode:${mode.id}`}
              aria-controls="mode-panel"
              tabIndex={activeRoute === `mode:${mode.id}` ? 0 : -1}
              disabled={transitioning}
              title={accelerator ? `Alt+${accelerator.toUpperCase()}` : undefined}
              onClick={(event) => void navigate(
                `mode:${mode.id}`,
                "push",
                { kind: "activator", element: event.currentTarget },
              )}
              onKeyDown={(event) => moveFocus(event, mode.id)}
            ><DestinationMark name={mode.id} /><span className="mode-tab-label">{label}</span></button>;
          })}
        </nav>
        {supporting.length > 0 && <>
          <p className="shell-section-label">{message(locale, "shell.v1.supporting")}</p>
          <nav className="support-navigation" aria-label={message(locale, "shell.v1.supporting")}>
            {supporting.map((destination) => <button
              key={destination.id}
              ref={(element) => {
                if (element) supportingButtons.current.set(destination.id, element);
                else supportingButtons.current.delete(destination.id);
              }}
              type="button"
              className="support-link"
              aria-current={activeRoute === `support:${destination.id}` ? "page" : undefined}
              disabled={transitioning}
              onClick={(event) => void navigate(
                `support:${destination.id}`,
                "push",
                { kind: "activator", element: event.currentTarget },
              )}
            ><DestinationMark name={destination.id} /><span>{message(locale, SUPPORTING_MESSAGE_KEYS[destination.id])}</span></button>)}
          </nav>
        </>}
        <div className="shell-footer">
          <button
            ref={settingsButton}
            type="button"
            aria-current={activeRoute === "settings" ? "page" : undefined}
            disabled={transitioning}
            onClick={(event) => void navigate(
              "settings",
              "push",
              { kind: "activator", element: event.currentTarget },
            )}
          ><DestinationMark name="language" /><span>{message(locale, "shell.v1.settings.action")}</span></button>
          <a href="https://github.com/bahayonghang/devsweep#readme" target="_blank" rel="noreferrer">
            <DestinationMark name="help" /><span>{message(locale, "shell.v1.help")}</span>
          </a>
        </div>
      </aside>
      <main id="mode-workbench" className="mode-workbench">
        <header className="page-header">
          <div className="page-header-text">
            <h1 id={headingId} tabIndex={-1} ref={contentHeading}>{activeLabel}</h1>
            <p className="page-subtitle">{activeSubtitle}</p>
          </div>
          <div className="page-header-slot" ref={setHeaderSlot} />
        </header>
        {localeSaving && <p className="persistence-warning" role="status">{message(locale, "shell.v1.persistence.saving")}</p>}
        {persistenceError && <p className="persistence-warning" role="alert">{message(locale, "shell.v1.persistence.unavailable")}</p>}
        {routeError && <div className="persistence-warning" role="alert">
          <span>{message(locale, "shell.v1.route.error")}</span>{" "}
          <button type="button" onClick={() => setRouteError(false)}>{message(locale, "shell.v1.route.dismiss")}</button>
        </div>}
        <section id="mode-panel" className="mode-panel" role={activeRegistration ? "tabpanel" : undefined} aria-labelledby={headingId}>
          {activeRegistration?.render()}
          {activeSupporting?.render()}
          {activeRoute === "settings" && <div className="settings-panel">
            <label>
              <span>{message(locale, "shell.v1.settings.action")}</span>
              <select
                aria-label={message(locale, "shell.v1.settings.action")}
                value={locale}
                disabled={localeSaving}
                aria-busy={localeSaving}
                onChange={(event) => void onLocaleChange(event.target.value as PresentationLanguageTag)}
              >
                <option value="en">{message(locale, "shell.v1.settings.option.en")}</option>
                <option value="zh-CN">{message(locale, "shell.v1.settings.option.zh_cn")}</option>
              </select>
            </label>
          </div>}
        </section>
      </main>
    </div>
  </PageHeaderSlotContext.Provider>;
}
