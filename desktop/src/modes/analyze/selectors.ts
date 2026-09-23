import type { AnalyzeNodeV1, AnalyzeSnapshotV1 } from "../../api/types.gen";

export const ANALYZE_PAGE_SIZE = 200;

export type AnalyzeSort = "size_desc" | "size_asc" | "name" | "kind";

export interface AnalyzeIndex {
  readonly snapshot: AnalyzeSnapshotV1;
  readonly nodesById: ReadonlyMap<number, AnalyzeNodeV1>;
  readonly childrenByParent: ReadonlyMap<number, readonly AnalyzeNodeV1[]>;
}

export interface DirectorySelection {
  readonly directory: AnalyzeNodeV1;
  readonly children: readonly AnalyzeNodeV1[];
}

export interface PagedChildren {
  readonly rows: readonly AnalyzeNodeV1[];
  readonly totalRows: number;
  readonly page: number;
  readonly pageCount: number;
}

// createAnalyzeIndex freezes one stable child array per directory. Caching by
// that immutable identity keeps page changes, repeated drill-down, and search
// keystrokes from sorting the same wide directory again.
const sortedChildrenCache = new WeakMap<readonly AnalyzeNodeV1[], Map<AnalyzeSort, readonly AnalyzeNodeV1[]>>();

/** Deterministic UTF-16 code-unit ordering without locale/ICU work. */
function compareCodeUnits(left: string, right: string): number {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

export function createAnalyzeIndex(snapshot: AnalyzeSnapshotV1): AnalyzeIndex {
  const nodesById = new Map<number, AnalyzeNodeV1>();
  const mutableChildren = new Map<number, AnalyzeNodeV1[]>();
  for (const node of snapshot.nodes) {
    nodesById.set(node.id, node);
    if (node.parent_id !== null) {
      const children = mutableChildren.get(node.parent_id) ?? [];
      children.push(node);
      mutableChildren.set(node.parent_id, children);
    }
  }
  const childrenByParent = new Map<number, readonly AnalyzeNodeV1[]>();
  for (const [parentId, children] of mutableChildren) {
    childrenByParent.set(parentId, Object.freeze([...children]));
  }
  return { snapshot, nodesById, childrenByParent };
}

export function selectDirectory(index: AnalyzeIndex, nodeId: number): DirectorySelection {
  const requested = index.nodesById.get(nodeId);
  const directory = requested?.kind === "directory" ? requested : index.nodesById.get(0);
  if (!directory || directory.kind !== "directory") throw new Error("Analyze snapshot has no represented root directory");
  return { directory, children: index.childrenByParent.get(directory.id) ?? [] };
}

export function selectBreadcrumbs(index: AnalyzeIndex, nodeId: number): readonly AnalyzeNodeV1[] {
  const selection = selectDirectory(index, nodeId);
  const result: AnalyzeNodeV1[] = [];
  const visited = new Set<number>();
  let cursor: AnalyzeNodeV1 | undefined = selection.directory;
  while (cursor && !visited.has(cursor.id)) {
    result.push(cursor);
    visited.add(cursor.id);
    cursor = cursor.parent_id === null ? undefined : index.nodesById.get(cursor.parent_id);
  }
  return result.reverse();
}

function sortedChildren(children: readonly AnalyzeNodeV1[], sort: AnalyzeSort): readonly AnalyzeNodeV1[] {
  let bySort = sortedChildrenCache.get(children);
  if (!bySort) {
    bySort = new Map();
    sortedChildrenCache.set(children, bySort);
  }
  const cached = bySort.get(sort);
  if (cached) return cached;

  const sorted = [...children];
  sorted.sort((left, right) => {
    let result = 0;
    if (sort === "size_desc") result = right.bytes - left.bytes;
    else if (sort === "size_asc") result = left.bytes - right.bytes;
    else if (sort === "kind") result = compareCodeUnits(left.kind, right.kind);
    else result = compareCodeUnits(left.name, right.name);
    return result || compareCodeUnits(left.name, right.name) || left.id - right.id;
  });
  const immutable = Object.freeze(sorted);
  bySort.set(sort, immutable);
  return immutable;
}

export function selectVisibleChildren(
  children: readonly AnalyzeNodeV1[],
  query: string,
  sort: AnalyzeSort,
): readonly AnalyzeNodeV1[] {
  const sorted = sortedChildren(children, sort);
  const normalized = query.trim().toLowerCase();
  return normalized.length === 0
    ? sorted
    : sorted.filter((node) => node.name.toLowerCase().includes(normalized));
}

export function selectPage(rows: readonly AnalyzeNodeV1[], requestedPage: number): PagedChildren {
  const pageCount = Math.max(1, Math.ceil(rows.length / ANALYZE_PAGE_SIZE));
  const page = Math.max(0, Math.min(Math.trunc(requestedPage), pageCount - 1));
  const start = page * ANALYZE_PAGE_SIZE;
  return {
    rows: rows.slice(start, start + ANALYZE_PAGE_SIZE),
    totalRows: rows.length,
    page,
    pageCount,
  };
}

export function pageForNode(rows: readonly AnalyzeNodeV1[], nodeId: number): number {
  const index = rows.findIndex((node) => node.id === nodeId);
  return index < 0 ? 0 : Math.floor(index / ANALYZE_PAGE_SIZE);
}

export interface MovedProjection {
  readonly index: AnalyzeIndex;
  /** Moved nodes and every node under them. */
  readonly moved: ReadonlySet<number>;
  /** Snapshot bytes of the outermost moved nodes. */
  readonly movedBytes: number;
}

function ancestorsOf(index: AnalyzeIndex, node: AnalyzeNodeV1): AnalyzeNodeV1[] {
  const result: AnalyzeNodeV1[] = [];
  const visited = new Set<number>([node.id]);
  let cursor = node.parent_id === null ? undefined : index.nodesById.get(node.parent_id);
  while (cursor && !visited.has(cursor.id)) {
    result.push(cursor);
    visited.add(cursor.id);
    cursor = cursor.parent_id === null ? undefined : index.nodesById.get(cursor.parent_id);
  }
  return result;
}

/**
 * Display-only projection after a Recycle Bin move. Moved subtrees show zero
 * bytes and their bytes are subtracted from each ancestor. The retained
 * snapshot is never edited.
 */
export function projectMoved(index: AnalyzeIndex, movedIds: readonly number[]): MovedProjection {
  if (movedIds.length === 0) return { index, moved: new Set(), movedBytes: 0 };
  const requested = new Set(movedIds.filter((id) => index.nodesById.has(id)));
  const outermost = [...requested]
    .map((id) => index.nodesById.get(id)!)
    .filter((node) => !ancestorsOf(index, node).some((ancestor) => requested.has(ancestor.id)));
  const moved = new Set<number>();
  const pending = outermost.map((node) => node.id);
  while (pending.length > 0) {
    const id = pending.pop()!;
    if (moved.has(id)) continue;
    moved.add(id);
    for (const child of index.childrenByParent.get(id) ?? []) pending.push(child.id);
  }
  const subtract = new Map<number, number>();
  for (const node of outermost) {
    for (const ancestor of ancestorsOf(index, node)) subtract.set(ancestor.id, (subtract.get(ancestor.id) ?? 0) + node.bytes);
  }
  const nodes = index.snapshot.nodes.map((node) => {
    if (moved.has(node.id)) return { ...node, bytes: 0 };
    const minus = subtract.get(node.id);
    return minus === undefined ? node : { ...node, bytes: Math.max(0, node.bytes - minus) };
  });
  return {
    index: createAnalyzeIndex({ ...index.snapshot, nodes }),
    moved,
    movedBytes: outermost.reduce((sum, node) => sum + node.bytes, 0),
  };
}
