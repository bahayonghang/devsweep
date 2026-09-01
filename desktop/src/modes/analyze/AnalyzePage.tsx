import { Fragment, useEffect, useLayoutEffect, useMemo, useReducer, useRef, useState, type KeyboardEvent } from "react";
import type { DesktopBridge } from "../../api/bridge";
import { decodeCommandError } from "../../api/contract";
import type { AnalyzeNodeV1, CommandError, DesktopAnalyzeResult } from "../../api/types.gen";
import { AccessibleUserData } from "../../app-shell/AppShell";
import { formatBinaryBytes, message, type PresentationLanguageTag } from "../../i18n";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { createAnalyzeIndex, pageForNode, selectBreadcrumbs, selectDirectory, selectPage, selectVisibleChildren, type AnalyzeSort } from "./selectors";
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

function nodeLabel(locale: PresentationLanguageTag, node: AnalyzeNodeV1): string {
  const bytes = node.evidence === "unknown" ? evidenceLabel(locale, node) : formatBinaryBytes(node.bytes);
  const warning = warningLabel(locale, node);
  return [node.name, kindLabel(locale, node), bytes, evidenceLabel(locale, node), warning].filter(Boolean).join(" — ");
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
  initialRoot = ".",
  onPerformanceSample,
}: AnalyzePageProps) {
  const [state, dispatch] = useReducer(analyzeReducer, initialAnalyzeState);
  const [root, setRoot] = useState(initialRoot);
  const [bounds, setBounds] = useState({ width: 900, height: 460 });
  const mounted = useRef(true);
  const treemapHost = useRef<HTMLDivElement>(null);
  const listbox = useRef<HTMLSelectElement>(null);
  const commitStarted = useRef<number | null>(null);

  useEffect(() => () => { mounted.current = false; }, []);
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

  const index = useMemo(() => state.snapshot ? createAnalyzeIndex(state.snapshot) : null, [state.snapshot]);
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

  const handleListKey = (event: KeyboardEvent<HTMLSelectElement>) => {
    if (event.key === "Enter" || event.key === "ArrowRight") {
      event.preventDefault();
      openNode(undefined, event.timeStamp);
    } else if (event.key === "Backspace" || event.key === "ArrowLeft") {
      event.preventDefault();
      goUp(event.timeStamp);
    }
  };

  const handleTileKey = (event: KeyboardEvent<SVGRectElement>, tile: TreemapTile) => {
    if (tile.nodeId === null) return;
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

  return <div className="analyze-mode" data-status={state.status}>
    <section className="analyze-toolbar" aria-label={message(locale, "command.analyze")}>
      <label className="analyze-root-input">
        <span>{message(locale, "analyze.v1.path.label")}</span>
        <input value={root} disabled={["loading", "canceling"].includes(state.status)} onChange={(event) => setRoot(event.target.value)} />
      </label>
      {["loading", "canceling"].includes(state.status)
        ? <button type="button" className="secondary-button" disabled={state.status === "canceling"} onClick={() => void cancelAnalysis()}>{message(locale, "analyze.v1.action.cancel")}</button>
        : <button type="button" className="primary-button" disabled={root.trim().length === 0} onClick={() => void runAnalysis()}>{message(locale, "analyze.v1.action.start")}</button>}
      {statusCopy ? <p className="analyze-live-status" role="status" aria-live="polite">{statusCopy}</p> : null}
    </section>

    {state.error ? <div className="error-banner" role="alert">
      <span>{message(locale, "analyze.v1.state.error")} {state.error.code === "analyze_failed" || state.error.code === "io" ? state.error.message : ""}</span>
      <button type="button" onClick={() => dispatch({ type: "error_dismissed" })}>×</button>
    </div> : null}

    {!state.snapshot ? <section className="analyze-empty">
      <h2>{message(locale, "analyze.v1.state.empty.title")}</h2>
      <p>{message(locale, "analyze.v1.state.empty.detail")}</p>
    </section> : <div className="analyze-workspace">
      <header className="analyze-summary">
        <div>
          <strong>{state.status === "complete"
            ? message(locale, "analyze.v1.scan.complete", { count: String(state.snapshot.nodes.length), bytes: formatBinaryBytes(state.snapshot.nodes[0]?.bytes ?? 0) })
            : state.status === "partial"
              ? message(locale, "analyze.v1.scan.partial", { count: String(state.snapshot.nodes.length), bytes: formatBinaryBytes(state.snapshot.nodes[0]?.bytes ?? 0) })
              : message(locale, "analyze.v1.scan.canceled", { count: String(state.snapshot.nodes.length), bytes: formatBinaryBytes(state.snapshot.nodes[0]?.bytes ?? 0) })}</strong>
          <span>{message(locale, "analyze.v1.scan.root", { path: state.snapshot.root.normalized })}</span>
        </div>
        <nav className="analyze-breadcrumbs" aria-label={message(locale, "analyze.v1.breadcrumbs.label")}>
          {breadcrumbs.map((node, indexValue) => <span key={node.id}>
            {indexValue > 0 ? <span aria-hidden="true">›</span> : null}
            <button type="button" aria-current={node.id === state.currentNodeId ? "location" : undefined} onClick={(event) => openNode(node.id, event.timeStamp)} title={node.name}>{node.id === 0 ? state.snapshot!.root.normalized : node.name}</button>
          </span>)}
        </nav>
      </header>

      <section className="analyze-controls">
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
        <section className="analyze-treemap-panel" aria-labelledby="analyze-treemap-title">
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
                  onKeyDown={(event) => handleTileKey(event, tile)}
                />
                {tileIndex < 24 && tile.width >= 70 && tile.height >= 24
                  ? <text x={tile.x + 6} y={tile.y + 17} aria-hidden="true">{tile.name}</text>
                  : null}
              </Fragment>)}
            </svg>
          </div>
        </section>

        <section className="analyze-list-panel" aria-labelledby="analyze-list-title">
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
            onKeyDown={handleListKey}
          >
            {state.focusedNodeId === null ? <option value="" disabled>{message(locale, "analyze.v1.list.title")}</option> : null}
            {page.rows.map((node) => <option key={node.id} value={node.id} title={nodeLabel(locale, node)}>{nodeLabel(locale, node)}</option>)}
          </select>
          <div className="analyze-list-actions">
            <button type="button" className="secondary-button" disabled={state.focusedNodeId === null || index?.nodesById.get(state.focusedNodeId)?.kind !== "directory"} onClick={(event) => openNode(undefined, event.timeStamp)}>{message(locale, "analyze.v1.action.open")}</button>
            <span>{message(locale, "analyze.v1.list.page", { page: String(page.page + 1), pages: String(page.pageCount), count: String(page.totalRows) })}</span>
            <button type="button" className="secondary-button" disabled={page.page === 0} onClick={(event) => { commitStarted.current = event.timeStamp; dispatch({ type: "page_changed", page: page.page - 1 }); }}>{message(locale, "analyze.v1.action.previous")}</button>
            <button type="button" className="secondary-button" disabled={page.page + 1 >= page.pageCount} onClick={(event) => { commitStarted.current = event.timeStamp; dispatch({ type: "page_changed", page: page.page + 1 }); }}>{message(locale, "analyze.v1.action.next")}</button>
          </div>
          {state.focusedNodeId !== null && index?.nodesById.get(state.focusedNodeId) ? <p className="analyze-focus-detail">
            <AccessibleUserData value={nodeLabel(locale, index.nodesById.get(state.focusedNodeId)!)} />
          </p> : null}
        </section>
      </div>
    </div>}
  </div>;
}
