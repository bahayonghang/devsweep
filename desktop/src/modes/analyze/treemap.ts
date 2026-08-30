import type { AnalyzeNodeV1 } from "../../api/types.gen";

export const MAX_TREEMAP_CHILDREN = 512;

export interface TreemapBounds {
  readonly width: number;
  readonly height: number;
}

export interface TreemapTile {
  readonly key: string;
  readonly nodeId: number | null;
  readonly name: string;
  readonly bytes: number;
  readonly representedCount: number;
  readonly evidence: AnalyzeNodeV1["evidence"] | "aggregated";
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly other: boolean;
}

interface Datum {
  readonly key: string;
  readonly nodeId: number | null;
  readonly name: string;
  readonly bytes: number;
  readonly representedCount: number;
  readonly evidence: TreemapTile["evidence"];
  readonly other: boolean;
  readonly area: number;
}

interface FloatRect { x: number; y: number; width: number; height: number }

/** Deterministic UTF-16 code-unit ordering without locale/ICU work. */
function compareCodeUnits(left: string, right: string): number {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

function compareKnown(left: AnalyzeNodeV1, right: AnalyzeNodeV1): number {
  return right.bytes - left.bytes || compareCodeUnits(left.name, right.name) || left.id - right.id;
}

function finiteDimension(value: number): number {
  return Number.isFinite(value) && value > 0 ? Math.max(1, Math.round(value)) : 1;
}

function roundCoordinate(value: number): number {
  return Math.round(value * 1_000_000) / 1_000_000;
}

function worst(row: readonly Datum[], side: number): number {
  if (row.length === 0 || side <= 0) return Number.POSITIVE_INFINITY;
  const sum = row.reduce((total, item) => total + item.area, 0);
  const largest = row[0].area;
  const smallest = row[row.length - 1].area;
  if (sum <= 0 || smallest <= 0) return Number.POSITIVE_INFINITY;
  const sideSquared = side * side;
  const sumSquared = sum * sum;
  return Math.max((sideSquared * largest) / sumSquared, sumSquared / (sideSquared * smallest));
}

function layoutRow(row: readonly Datum[], remaining: FloatRect, output: TreemapTile[]): FloatRect {
  const rowArea = row.reduce((total, item) => total + item.area, 0);
  if (remaining.width >= remaining.height) {
    const rowHeight = remaining.width <= 0 ? 0 : rowArea / remaining.width;
    let x = remaining.x;
    for (const [index, item] of row.entries()) {
      const width = index === row.length - 1 ? remaining.x + remaining.width - x : item.area / rowHeight;
      output.push(toTile(item, { x, y: remaining.y, width, height: rowHeight }));
      x += width;
    }
    return { x: remaining.x, y: remaining.y + rowHeight, width: remaining.width, height: Math.max(0, remaining.height - rowHeight) };
  }
  const rowWidth = remaining.height <= 0 ? 0 : rowArea / remaining.height;
  let y = remaining.y;
  for (const [index, item] of row.entries()) {
    const height = index === row.length - 1 ? remaining.y + remaining.height - y : item.area / rowWidth;
    output.push(toTile(item, { x: remaining.x, y, width: rowWidth, height }));
    y += height;
  }
  return { x: remaining.x + rowWidth, y: remaining.y, width: Math.max(0, remaining.width - rowWidth), height: remaining.height };
}

function toTile(item: Datum, rect: FloatRect): TreemapTile {
  return {
    key: item.key,
    nodeId: item.nodeId,
    name: item.name,
    bytes: item.bytes,
    representedCount: item.representedCount,
    evidence: item.evidence,
    x: roundCoordinate(rect.x),
    y: roundCoordinate(rect.y),
    width: roundCoordinate(Math.max(0, rect.width)),
    height: roundCoordinate(Math.max(0, rect.height)),
    other: item.other,
  };
}

export function layoutTreemap(
  children: readonly AnalyzeNodeV1[],
  bounds: TreemapBounds,
  maxChildren = MAX_TREEMAP_CHILDREN,
): readonly TreemapTile[] {
  const width = finiteDimension(bounds.width);
  const height = finiteDimension(bounds.height);
  const known = children
    .filter((node) => node.evidence !== "unknown" && node.bytes > 0)
    .sort(compareKnown);
  if (known.length === 0) return [];

  const boundedCount = Math.max(0, Math.min(MAX_TREEMAP_CHILDREN, Math.trunc(maxChildren)));
  const totalBytes = known.reduce((total, node) => total + node.bytes, 0);
  if (!Number.isSafeInteger(totalBytes) || totalBytes <= 0) return [];

  // Shapes smaller than a quarter CSS pixel cannot remain a truthful linked
  // rectangle after deterministic rounding. Reconcile them into Other.
  const canvasArea = width * height;
  const visible: AnalyzeNodeV1[] = [];
  let otherBytes = 0;
  let omittedCount = 0;
  for (const [index, node] of known.entries()) {
    const renderable = index < boundedCount && node.bytes / totalBytes * canvasArea >= 0.25;
    if (renderable) visible.push(node);
    else {
      otherBytes += node.bytes;
      omittedCount += 1;
    }
  }
  const data: Omit<Datum, "area">[] = visible.map((node) => ({
    key: `node:${node.id}`,
    nodeId: node.id,
    name: node.name,
    bytes: node.bytes,
    representedCount: 1,
    evidence: node.evidence,
    other: false,
  }));
  if (omittedCount > 0 && otherBytes > 0) {
    data.push({
      key: "other",
      nodeId: null,
      name: "Other",
      bytes: otherBytes,
      representedCount: omittedCount,
      evidence: "aggregated",
      other: true,
    });
  }
  const areaScale = canvasArea / totalBytes;
  const remainingData: Datum[] = data
    .map((item) => ({ ...item, area: item.bytes * areaScale }))
    .sort((left, right) => right.area - left.area || compareCodeUnits(left.key, right.key));
  const output: TreemapTile[] = [];
  let remainingRect: FloatRect = { x: 0, y: 0, width, height };
  let row: Datum[] = [];
  while (remainingData.length > 0) {
    const next = remainingData[0];
    const side = Math.min(remainingRect.width, remainingRect.height);
    if (row.length === 0 || worst([...row, next], side) <= worst(row, side)) {
      row.push(next);
      remainingData.shift();
    } else {
      remainingRect = layoutRow(row, remainingRect, output);
      row = [];
    }
  }
  if (row.length > 0) layoutRow(row, remainingRect, output);
  return output;
}
