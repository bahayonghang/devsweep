import { Fragment, useCallback, useEffect, useLayoutEffect, useMemo, useReducer, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import type { DesktopBridge } from "../../api/bridge";
import { decodeCommandError } from "../../api/contract";
import type { AnalyzeNodeV1, AnalyzeTrashRefusalCode, AnalyzeTrashReportV1, CommandError, DesktopAnalyzeResult } from "../../api/types.gen";
import { AccessibleUserData } from "../../app-shell/AppShell";
import { ConfirmDialog } from "../../components/ConfirmDialog";
import { formatBinaryBytes, message, type PresentationLanguageTag } from "../../i18n";
import { DetailView, Stage, StageResult } from "../../stage";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { AnalyzeContextMenu } from "./ContextMenu";
import { createAnalyzeIndex, pageForNode, projectMoved, selectBreadcrumbs, selectDirectory, selectPage, selectVisibleChildren, type AnalyzeSort } from "./selectors";
import { analyzeReducer, initialAnalyzeState } from "./state";
import { layoutTreemap, type TreemapTile } from "./treemap";
import "./styles.css";

export interface AnalyzePerformanceSample {
  readonly kind: "layout" | "commit";
  readonly durationMs: number;
  readonly rectangles: number;
}

interface AnalyzePageProps {
  readonly bridge: DesktopBridge;
  readonly coordinator: OperationCoordinator;
  readonly locale: PresentationLanguageTag;
  /** Root path. When absent, the page asks the host for the system drive. */
  readonly initialRoot?: string;
  readonly onPerformanceSample?: (sample: AnalyzePerformanceSample) => void;
}

let operationSequence = 0;
function createOperationId(): string {
  operationSequence += 1;
  const randomId = globalThis.crypto?.randomUUID?.();
  return randomId ? `analyze-${randomId}` : `analyze-${Date.now()}-${operationSequence}`;
}

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected Analyze IPC error" }; }
}

function evidenceLabel(locale: PresentationLanguageTag, node: AnalyzeNodeV1): string {
  return message(locale, `analyze.v1.evidence.${node.evidence}`);
}

function kindLabel(locale: PresentationLanguageTag, node: AnalyzeNodeV1): string {
  return message(locale, `analyze.v1.kind.${node.kind}`);
}

function warningLabel(locale: PresentationLanguageTag, node: AnalyzeNodeV1): string | null {
  if (node.warnings.includes("access_denied")) return message(locale, "analyze.v1.warning.permission");
  if (node.kind === "reparse" || node.warnings.includes("reparse")) return message(locale, "analyze.v1.warning.unsupported");
  return null;
}

function nodeLabel(locale: PresentationLanguageTag, node: AnalyzeNodeV1, moved = false): string {
  const bytes = node.evidence === "unknown" ? evidenceLabel(locale, node) : formatBinaryBytes(node.bytes);
  const warning = warningLabel(locale, node);
  const movedLabel = moved ? message(locale, "analyze.v1.moved.label") : null;
  return [node.name, kindLabel(locale, node), bytes, evidenceLabel(locale, node), warning, movedLabel].filter(Boolean).join(" — ");
}

function refusalLabel(locale: PresentationLanguageTag, code: AnalyzeTrashRefusalCode | "moved"): string {
  return message(locale, `analyze.v1.refusal.${code}`);
}

function movedBytes(report: AnalyzeTrashReportV1, itemBytes: ReadonlyMap<number, number>): number {
  return report.moved_node_ids.reduce((sum, nodeId) => sum + (itemBytes.get(nodeId) ?? 0), 0);
}

interface MenuState {
  readonly nodeId: number;
  readonly x: number;
  readonly y: number;
}

function tileLabel(locale: PresentationLanguageTag, tile: TreemapTile): string {
  if (tile.other) {
    return message(locale, "analyze.v1.treemap.other", {
      count: String(tile.representedCount),
      bytes: formatBinaryBytes(tile.bytes),
    });
  }
  const evidence = tile.evidence === "aggregated" ? "" : message(locale, `analyze.v1.evidence.${tile.evidence}`);
  return `${tile.name} — ${formatBinaryBytes(tile.bytes)} — ${evidence}`;
}

export function AnalyzePage({
  bridge,
  coordinator,
  locale,
  initialRoot,
  onPerformanceSample,
}: AnalyzePageProps) {
  const [state, dispatch] = useReducer(analyzeReducer, initialAnalyzeState);
  const [root, setRoot] = useState(initialRoot ?? "");
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [revealError, setRevealError] = useState<CommandError | null>(null);
  const menuOrigin = useRef<Element | null>(null);
  const [bounds, setBounds] = useState({ width: 900, height: 460 });
  const [overview, setOverview] = useState(false);
  const mounted = useRef(true);
  const treemapHost = useRef<HTMLDivElement>(null);
  const listbox = useRef<HTMLSelectElement>(null);
  const commitStarted = useRef<number | null>(null);

  useEffect(() => () => { mounted.current = false; }, []);
  useEffect(() => {
    if (initialRoot !== undefined) return;
    void bridge.analyzeDefaultRoot()
      .then((value) => { if (mounted.current) setRoot((current) => current.length === 0 ? value : current); })
      .catch(() => undefined);
  }, [bridge, initialRoot]);
  useLayoutEffect(() => {
    const host = treemapHost.current;
    if (!host || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => {
      const next = entry.contentRect;
      setBounds({ width: Math.max(1, Math.round(next.width)), height: Math.max(220, Math.round(next.height)) });
    });
    observer.observe(host);
    return () => observer.disconnect();
  }, []);

  const snapshotIndex = useMemo(() => state.snapshot ? createAnalyzeIndex(state.snapshot) : null, [state.snapshot]);
  const projection = useMemo(() => snapshotIndex ? projectMoved(snapshotIndex, state.movedIds) : null, [snapshotIndex, state.movedIds]);
  const index = projection?.index ?? null;
  const directorySelection = useMemo(
    () => index ? selectDirectory(index, state.currentNodeId) : null,
    [index, state.currentNodeId],
  );
  const visibleChildren = useMemo(
    () => directorySelection ? selectVisibleChildren(directorySelection.children, state.query, state.sort) : [],
    [directorySelection, state.query, state.sort],
  );
  const page = useMemo(() => selectPage(visibleChildren, state.page), [visibleChildren, state.page]);
  const breadcrumbs = useMemo(
    () => index ? selectBreadcrumbs(index, state.currentNodeId) : [],
    [index, state.currentNodeId],
  );
  const tiles = useMemo(() => {
    return directorySelection ? layoutTreemap(directorySelection.children, bounds) : [];
  }, [bounds, directorySelection]);

  useLayoutEffect(() => {
    if (!onPerformanceSample || !directorySelection) return;
    const started = performance.now();
    const measured = layoutTreemap(directorySelection.children, bounds);
    onPerformanceSample({ kind: "layout", durationMs: performance.now() - started, rectangles: measured.length });
  }, [bounds, directorySelection, onPerformanceSample]);

  useLayoutEffect(() => {
    const navigationStarted = commitStarted.current;
    if (navigationStarted !== null) listbox.current?.focus();
    if (navigationStarted !== null) {
      onPerformanceSample?.({ kind: "commit", durationMs: performance.now() - navigationStarted, rectangles: tiles.length });
      commitStarted.current = null;
    }
  }, [onPerformanceSample, state.currentNodeId, state.focusedNodeId, state.page, tiles.length]);

  const runAnalysis = async () => {
    if (["loading", "canceling"].includes(state.status) || root.trim().length === 0) return;
    const operationId = createOperationId();
    let progressError: CommandError | null = null;
    let started = false;
    try {
      const lease = await coordinator.start<DesktopAnalyzeResult>({
        kind: "analyze",
        id: operationId,
        cancel: async () => {
          if (mounted.current) dispatch({ type: "analysis_cancel_requested", operationId });
          await bridge.analyzeCancel(operationId);
        },
        start: () => {
          started = true;
          if (mounted.current) dispatch({ type: "analysis_requested", operationId });
          return bridge.analyzeStart(
            operationId,
            root.trim(),
            (progress) => { if (mounted.current) dispatch({ type: "analysis_progressed", progress }); },
            (error) => {
              if (progressError) return;
              progressError = commandError(error);
              if (mounted.current) dispatch({ type: "analysis_cancel_requested", operationId });
              void bridge.analyzeCancel(operationId).catch(() => undefined);
            },
          );
        },
      });
      if (!lease) return;
      try {
        const result = await lease.result;
        if (!mounted.current) return;
        if (progressError) dispatch({ type: "analysis_failed", operationId, error: progressError });
        else if (result.type === "completed") dispatch({ type: "analysis_completed", operationId, snapshot: result.snapshot });
        else dispatch({ type: "analysis_canceled", operationId, snapshot: result.snapshot });
      } finally {
        await lease.complete().catch(() => false);
      }
    } catch (error) {
      if (started && mounted.current) dispatch({ type: "analysis_failed", operationId, error: progressError ?? commandError(error) });
    }
  };

  const cancelAnalysis = async () => {
    const operationId = state.operationId;
    if (!operationId || state.status !== "loading") return;
    dispatch({ type: "analysis_cancel_requested", operationId });
    try { await bridge.analyzeCancel(operationId); }
    catch (error) { dispatch({ type: "analysis_failed", operationId, error: commandError(error) }); }
  };

  const focusNode = (nodeId: number, startedAt?: number) => {
    if (startedAt !== undefined) commitStarted.current = startedAt;
    dispatch({ type: "node_focused", nodeId, page: pageForNode(visibleChildren, nodeId) });
  };

  const openNode = (nodeId: number | null = state.focusedNodeId, startedAt?: number) => {
    if (nodeId === null || !index) return;
    const node = index.nodesById.get(nodeId);
    if (node?.kind !== "directory") return;
    if (startedAt !== undefined) commitStarted.current = startedAt;
    const anchor = state.focusAnchors[String(nodeId)];
    const target = selectDirectory(index, nodeId);
    const targetRows = selectVisibleChildren(target.children, state.query, state.sort);
    const targetPage = anchor === undefined ? 0 : pageForNode(targetRows, anchor);
    dispatch({ type: "directory_opened", nodeId, page: targetPage });
  };

  const goUp = (startedAt?: number) => {
    if (!index || !directorySelection || directorySelection.directory.parent_id === null) return;
    if (startedAt !== undefined) commitStarted.current = startedAt;
    const parentId = directorySelection.directory.parent_id;
    const parent = selectDirectory(index, parentId);
    const parentRows = selectVisibleChildren(parent.children, state.query, state.sort);
    dispatch({
      type: "directory_up",
      parentId,
      childId: directorySelection.directory.id,
      page: pageForNode(parentRows, directorySelection.directory.id),
    });
  };

  const closeMenu = useCallback(() => {
    setMenu(null);
    const origin = menuOrigin.current;
    menuOrigin.current = null;
    if (origin instanceof HTMLElement || origin instanceof SVGElement) {
      if (origin.isConnected) origin.focus();
      else listbox.current?.focus();
    }
  }, []);

  const openMenu = (nodeId: number, origin: Element, point?: { x: number; y: number }) => {
    if (!index?.nodesById.has(nodeId) || !state.snapshotOperationId || state.trash.phase !== "idle") return;
    const rect = origin.getBoundingClientRect();
    menuOrigin.current = origin;
    if (state.focusedNodeId !== nodeId) dispatch({ type: "node_focused", nodeId, page: pageForNode(visibleChildren, nodeId) });
    setMenu({ nodeId, x: Math.round(point?.x ?? rect.left + 8), y: Math.round(point?.y ?? rect.top + Math.min(rect.height, 24)) });
  };

  const isMenuKey = (event: KeyboardEvent) => event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey);

  const moveDisabledReason = (nodeId: number): string | null => {
    if (projection?.moved.has(nodeId)) return refusalLabel(locale, "moved");
    const hint = state.refusalHints[String(nodeId)];
    if (hint) return refusalLabel(locale, hint);
    const node = index?.nodesById.get(nodeId);
    if (!node || node.parent_id === null) return refusalLabel(locale, "analysis_root");
    if (node.kind === "reparse") return refusalLabel(locale, "reparse_point");
    return null;
  };

  const reveal = async (nodeId: number) => {
    const operationId = state.snapshotOperationId;
    closeMenu();
    if (!operationId) return;
    setRevealError(null);
    try { await bridge.analyzeReveal(operationId, nodeId); }
    catch (error) { if (mounted.current) setRevealError(commandError(error)); }
  };

  const previewTrash = async (nodeId: number) => {
    const operationId = state.snapshotOperationId;
    setMenu(null);
    menuOrigin.current = null;
    if (!operationId) return;
    dispatch({ type: "trash_preview_requested", operationId, nodeIds: [nodeId] });
    try {
      const preview = await bridge.analyzeTrashPreview(operationId, [nodeId]);
      if (mounted.current) dispatch({ type: "trash_preview_loaded", preview });
    } catch (error) {
      if (mounted.current) dispatch({ type: "trash_preview_failed", operationId, error: commandError(error) });
    }
  };

  const executeTrash = async () => {
    const operationId = state.snapshotOperationId;
    const preview = state.trash.preview;
    if (!operationId || state.trash.phase !== "confirming" || !preview || preview.items.length === 0) return;
    const nodeIds = [...state.trash.nodeIds];
    const digest = preview.digest;
    dispatch({ type: "trash_execute_requested" });
    try {
      const lease = await coordinator.start<AnalyzeTrashReportV1>({
        kind: "analyze.trash",
        id: `${operationId}:trash:${createOperationId()}`,
        cancel: () => undefined,
        start: () => bridge.analyzeTrashExecute(operationId, nodeIds, digest, true),
      });
      if (!lease) {
        if (mounted.current) dispatch({ type: "trash_execute_failed", operationId, error: { code: "io", message: "Operation closed" } });
        return;
      }
      try {
        const report = await lease.result;
        if (mounted.current) dispatch({ type: "trash_executed", report });
      } finally {
        await lease.complete().catch(() => false);
      }
    } catch (error) {
      if (mounted.current) dispatch({ type: "trash_execute_failed", operationId, error: commandError(error) });
    }
  };

  const handleListKey = (event: KeyboardEvent<HTMLSelectElement>) => {
    if (isMenuKey(event)) {
      event.preventDefault();
      if (state.focusedNodeId !== null) openMenu(state.focusedNodeId, event.currentTarget);
      return;
    }
    if (event.key === "Enter" || event.key === "ArrowRight") {
      event.preventDefault();
      openNode(undefined, event.timeStamp);
    } else if (event.key === "Backspace" || event.key === "ArrowLeft") {
      event.preventDefault();
      goUp(event.timeStamp);
    }
  };

  const handleListContextMenu = (event: MouseEvent<HTMLSelectElement>) => {
    event.preventDefault();
    const option = event.target instanceof Element ? event.target.closest("option") : null;
    const nodeId = option && option.value !== "" ? Number(option.value) : state.focusedNodeId;
    if (nodeId !== null) openMenu(nodeId, event.currentTarget, { x: event.clientX, y: event.clientY });
  };

  const handleTileKey = (event: KeyboardEvent<SVGRectElement>, tile: TreemapTile) => {
    if (tile.nodeId === null) return;
    if (isMenuKey(event)) {
      event.preventDefault();
      openMenu(tile.nodeId, event.currentTarget);
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      focusNode(tile.nodeId, event.timeStamp);
      if (event.key === "Enter") openNode(tile.nodeId, event.timeStamp);
    }
  };

  const statusCopy = state.status === "loading"
    ? message(locale, "analyze.v1.state.loading", { count: String(state.storedNodes) })
    : state.status === "canceling"
      ? message(locale, "analyze.v1.state.canceling")
      : null;

  const busy = ["loading", "canceling"].includes(state.status);
  const statusChip = statusCopy ? <span className="status-chip"><span className="status-chip-dot" aria-hidden="true" />{statusCopy}</span> : null;
  const liveStatus = statusCopy ? <p className="analyze-live-status" role="status" aria-live="polite">{statusCopy}</p> : null;
  const rootInput = <label className="analyze-root-input">
    <span>{message(locale, "analyze.v1.path.label")}</span>
    <input value={root} disabled={busy} onChange={(event) => setRoot(event.target.value)} />
  </label>;
  const errorBanner = state.error ? <div className="error-banner" role="alert">
    <span>{message(locale, "analyze.v1.state.error")} {state.error.code === "analyze_failed" || state.error.code === "io" ? state.error.message : ""}</span>
    <button type="button" onClick={() => dispatch({ type: "error_dismissed" })}>×</button>
  </div> : revealError ? <div className="error-banner" role="alert">
    <span>{message(locale, "analyze.v1.reveal.failed")} {"message" in revealError ? revealError.message : ""}</span>
    <button type="button" onClick={() => setRevealError(null)}>×</button>
  </div> : null;
  const movedSummary = projection && projection.movedBytes > 0
    ? <p className="analyze-moved-summary" role="status">{message(locale, "analyze.v1.moved.summary", { bytes: formatBinaryBytes(projection.movedBytes) })}</p>
    : null;
  const rootNode = index?.nodesById.get(0) ?? state.snapshot?.nodes[0];
  const summaryText = state.snapshot
    ? message(locale, `analyze.v1.scan.${state.status === "complete" ? "complete" : state.status === "partial" ? "partial" : "canceled"}`, { count: String(state.snapshot.nodes.length), bytes: formatBinaryBytes(rootNode?.bytes ?? 0) })
    : "";

  const trashPreview = state.trash.preview;
  const trashItemBytes = new Map(trashPreview?.items.map((item) => [item.node_id, item.bytes]) ?? []);
  const trashError = state.trash.error;
  const trashView = <DetailView
    locale={locale}
    className="analyze-trash-view"
    onBack={["previewing", "executing"].includes(state.trash.phase) ? undefined : () => dispatch({ type: "trash_dismissed" })}
  >
    <section className="analyze-workspace" aria-labelledby="analyze-trash-title">
      <header className="card analyze-summary">
        <h2 id="analyze-trash-title">{message(locale, "analyze.v1.trash.title")}</h2>
        {state.trash.phase === "previewing" ? <p role="status">{message(locale, "analyze.v1.trash.loading")}</p> : null}
        {trashPreview ? <>
          <strong>{message(locale, "analyze.v1.trash.items", { count: String(trashPreview.items.length), bytes: formatBinaryBytes(trashPreview.items.reduce((sum, item) => sum + item.bytes, 0)) })}</strong>
          {trashPreview.refused.length > 0 ? <span>{message(locale, "analyze.v1.trash.refused", { count: String(trashPreview.refused.length) })}</span> : null}
          {trashPreview.items.length === 0 ? <span>{message(locale, "analyze.v1.trash.none")}</span> : null}
        </> : null}
      </header>
      {trashPreview && trashPreview.items.length > 0 ? <ul className="card analyze-trash-list">
        {trashPreview.items.map((item) => <li key={item.node_id}>
          <AccessibleUserData value={item.path} />
          <span>{formatBinaryBytes(item.bytes)}</span>
        </li>)}
      </ul> : null}
      {trashPreview && trashPreview.refused.length > 0 ? <ul className="card analyze-trash-list">
        {trashPreview.refused.map((refusal) => <li key={refusal.node_id}>
          <AccessibleUserData value={index?.nodesById.get(refusal.node_id)?.name ?? String(refusal.node_id)} />
          <span>{message(locale, "analyze.v1.menu.view_only", { reason: refusalLabel(locale, refusal.reason_code) })}</span>
        </li>)}
      </ul> : null}
      {state.trash.report ? <p className="analyze-trash-result" role="status">
        {message(locale, "analyze.v1.trash.reported", { count: String(state.trash.report.moved_node_ids.length), bytes: formatBinaryBytes(movedBytes(state.trash.report, trashItemBytes)) })}
        {state.trash.report.report.failed > 0 ? ` ${message(locale, "analyze.v1.trash.failed")}` : ""}
      </p> : null}
      {trashError ? <div className="error-banner" role="alert">
        <span>{trashError.code === "stale_confirmation" || trashError.code === "analyze_stale_operation"
          ? message(locale, "analyze.v1.trash.stale")
          : `${message(locale, "analyze.v1.trash.failed")} ${"message" in trashError ? trashError.message : ""}`}</span>
      </div> : null}
      <div className="analyze-list-actions">
        {state.trash.phase === "reviewed" || state.trash.phase === "confirming" || state.trash.phase === "executing"
          ? <button type="button" className="primary-button" disabled={state.trash.phase !== "reviewed" || !trashPreview || trashPreview.items.length === 0} onClick={() => dispatch({ type: "trash_confirm_opened" })}>{message(locale, "analyze.v1.trash.review")}</button>
          : null}
        <button type="button" className="secondary-button" disabled={["previewing", "executing"].includes(state.trash.phase)} onClick={() => dispatch({ type: "trash_dismissed" })}>{message(locale, "analyze.v1.trash.close")}</button>
      </div>
    </section>
    {trashPreview && (state.trash.phase === "confirming" || state.trash.phase === "executing") ? <ConfirmDialog
      locale={locale}
      open
      digest={trashPreview.digest}
      irreversible={false}
      busy={state.trash.phase === "executing"}
      onCancel={() => dispatch({ type: "trash_confirm_closed" })}
      onConfirm={() => void executeTrash()}
    /> : null}
  </DetailView>;

  const menuNode = menu ? index?.nodesById.get(menu.nodeId) : undefined;

  return <div className="analyze-mode" data-status={state.status}>
    {!state.snapshot || overview ? <>
      {!state.snapshot ? <Stage
        mode="analyze"
        label={message(locale, "command.analyze")}
        className="analyze-stage"
        busy={busy}
        title={message(locale, "analyze.v1.state.empty.title")}
        meta={<><p>{message(locale, "analyze.v1.state.empty.detail")}</p>{liveStatus}</>}
        controls={rootInput}
        primary={busy
          ? <button type="button" className="stage-action" disabled={state.status === "canceling"} onClick={() => void cancelAnalysis()}>{message(locale, "analyze.v1.action.cancel")}</button>
          : <button type="button" className="stage-action" disabled={root.trim().length === 0} onClick={() => void runAnalysis()}>{message(locale, "analyze.v1.action.start")}</button>}
      /> : <StageResult
        mode="analyze"
        label={message(locale, "command.analyze")}
        caption={message(locale, "stage.v1.caption.analyze")}
        value={rootNode?.evidence === "unknown" ? message(locale, "analyze.v1.evidence.unknown") : formatBinaryBytes(rootNode?.bytes ?? 0)}
        meta={<><p>{summaryText}</p><p><AccessibleUserData value={message(locale, "analyze.v1.scan.root", { path: state.snapshot.root.normalized })} /></p>{movedSummary}{liveStatus}</>}
        action={<button type="button" className="stage-action" onClick={() => setOverview(false)}>{message(locale, "stage.v1.action.details")}</button>}
      />}
      {errorBanner}
    </> : state.trash.phase !== "idle" ? trashView : <DetailView locale={locale} onBack={() => setOverview(true)} status={statusChip}>
    <section className="analyze-toolbar" aria-label={message(locale, "command.analyze")}>
      {rootInput}
      {busy
        ? <button type="button" className="secondary-button" disabled={state.status === "canceling"} onClick={() => void cancelAnalysis()}>{message(locale, "analyze.v1.action.cancel")}</button>
        : state.snapshot
          ? <button type="button" className="primary-button" disabled={root.trim().length === 0} onClick={() => void runAnalysis()}>{message(locale, "analyze.v1.action.start")}</button>
          : null}
      {liveStatus}
    </section>

    {errorBanner}

    <div className="analyze-workspace">
      <header className="card analyze-summary">
        <div>
          <strong>{summaryText}</strong>
          <span>{message(locale, "analyze.v1.scan.root", { path: state.snapshot.root.normalized })}</span>
          {movedSummary}
        </div>
        <nav className="analyze-breadcrumbs" aria-label={message(locale, "analyze.v1.breadcrumbs.label")}>
          {breadcrumbs.map((node, indexValue) => <span key={node.id}>
            {indexValue > 0 ? <span aria-hidden="true">›</span> : null}
            <button type="button" aria-current={node.id === state.currentNodeId ? "location" : undefined} onClick={(event) => openNode(node.id, event.timeStamp)} title={node.name}>{node.id === 0 ? state.snapshot!.root.normalized : node.name}</button>
          </span>)}
        </nav>
      </header>

      <section className="card analyze-controls">
        <label>{message(locale, "analyze.v1.search.label")}<input value={state.query} onChange={(event) => dispatch({ type: "query_changed", query: event.target.value })} /></label>
        <label>{message(locale, "analyze.v1.sort.label")}<select value={state.sort} onChange={(event) => dispatch({ type: "sort_changed", sort: event.target.value as AnalyzeSort })}>
          <option value="size_desc">{message(locale, "analyze.v1.sort.size_desc")}</option>
          <option value="size_asc">{message(locale, "analyze.v1.sort.size_asc")}</option>
          <option value="name">{message(locale, "analyze.v1.sort.name")}</option>
          <option value="kind">{message(locale, "analyze.v1.sort.kind")}</option>
        </select></label>
        <button type="button" className="secondary-button" disabled={directorySelection?.directory.parent_id === null} onClick={(event) => goUp(event.timeStamp)}>{message(locale, "analyze.v1.action.up")}</button>
      </section>

      <div className="analyze-columns">
        <section className="card analyze-treemap-panel" aria-labelledby="analyze-treemap-title">
          <header><h2 id="analyze-treemap-title">{message(locale, "analyze.v1.treemap.title")}</h2><p>{message(locale, "analyze.v1.treemap.description")}</p></header>
          <div className="analyze-treemap-host" ref={treemapHost}>
            <svg className="analyze-treemap" viewBox={`0 0 ${bounds.width} ${bounds.height}`} role="group" aria-labelledby="analyze-treemap-title">
              {tiles.map((tile, tileIndex) => <Fragment key={tile.key}>
                <rect
                  x={tile.x} y={tile.y} width={tile.width} height={tile.height}
                  className={`analyze-tile evidence-${tile.evidence}${state.focusedNodeId === tile.nodeId ? " is-focused" : ""}`}
                  role={tile.nodeId === null ? "img" : "button"}
                  tabIndex={tile.nodeId === null ? undefined : 0}
                  aria-label={tileLabel(locale, tile)}
                  onClick={(event) => { if (tile.nodeId !== null) focusNode(tile.nodeId, event.timeStamp); }}
                  onDoubleClick={(event) => openNode(tile.nodeId, event.timeStamp)}
                  onContextMenu={(event) => {
                    event.preventDefault();
                    if (tile.nodeId !== null) openMenu(tile.nodeId, event.currentTarget, { x: event.clientX, y: event.clientY });
                  }}
                  onKeyDown={(event) => handleTileKey(event, tile)}
                />
                {tileIndex < 24 && tile.width >= 70 && tile.height >= 24
                  ? <text x={tile.x + 6} y={tile.y + 17} aria-hidden="true">{tile.name}</text>
                  : null}
              </Fragment>)}
            </svg>
          </div>
        </section>

        <section className="card analyze-list-panel" aria-labelledby="analyze-list-title">
          <header><h2 id="analyze-list-title">{message(locale, "analyze.v1.list.title")}</h2><p id="analyze-list-instruction">{message(locale, "analyze.v1.list.instruction")}</p></header>
          <select
            ref={listbox}
            className="analyze-listbox"
            size={Math.min(15, Math.max(4, page.rows.length))}
            aria-label={message(locale, "analyze.v1.list.title")}
            aria-describedby="analyze-list-instruction"
            value={state.focusedNodeId === null ? "" : String(state.focusedNodeId)}
            onChange={(event) => focusNode(Number(event.target.value), event.timeStamp)}
            onDoubleClick={(event) => openNode(undefined, event.timeStamp)}
            onContextMenu={handleListContextMenu}
            onKeyDown={handleListKey}
          >
            {state.focusedNodeId === null ? <option value="" disabled>{message(locale, "analyze.v1.list.title")}</option> : null}
            {page.rows.map((node) => {
              const label = nodeLabel(locale, node, projection?.moved.has(node.id));
              return <option key={node.id} value={node.id} title={label}>{label}</option>;
            })}
          </select>
          <div className="analyze-list-actions">
            <button type="button" className="secondary-button" disabled={state.focusedNodeId === null || index?.nodesById.get(state.focusedNodeId)?.kind !== "directory"} onClick={(event) => openNode(undefined, event.timeStamp)}>{message(locale, "analyze.v1.action.open")}</button>
            <span>{message(locale, "analyze.v1.list.page", { page: String(page.page + 1), pages: String(page.pageCount), count: String(page.totalRows) })}</span>
            <button type="button" className="secondary-button" disabled={page.page === 0} onClick={(event) => { commitStarted.current = event.timeStamp; dispatch({ type: "page_changed", page: page.page - 1 }); }}>{message(locale, "analyze.v1.action.previous")}</button>
            <button type="button" className="secondary-button" disabled={page.page + 1 >= page.pageCount} onClick={(event) => { commitStarted.current = event.timeStamp; dispatch({ type: "page_changed", page: page.page + 1 }); }}>{message(locale, "analyze.v1.action.next")}</button>
          </div>
          {state.focusedNodeId !== null && index?.nodesById.get(state.focusedNodeId) ? <p className="analyze-focus-detail">
            <AccessibleUserData value={nodeLabel(locale, index.nodesById.get(state.focusedNodeId)!, projection?.moved.has(state.focusedNodeId))} />
          </p> : null}
        </section>
      </div>
    </div>
    </DetailView>}
    {menu && menuNode ? <AnalyzeContextMenu
      locale={locale}
      name={menuNode.name}
      x={menu.x}
      y={menu.y}
      moveDisabledReason={moveDisabledReason(menu.nodeId)}
      onReveal={() => void reveal(menu.nodeId)}
      onMove={() => void previewTrash(menu.nodeId)}
      onClose={closeMenu}
    /> : null}
  </div>;
}
