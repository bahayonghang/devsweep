import { describe, expect, it } from "vitest";
import type { AnalyzeNodeV1 } from "../../api/types.gen";
import { layoutTreemap, MAX_TREEMAP_CHILDREN } from "./treemap";

function node(id: number, bytes: number, evidence: AnalyzeNodeV1["evidence"] = "complete"): AnalyzeNodeV1 {
  return { id, parent_id: 0, kind: "file", name: `node-${id}`, bytes, immediate_count: 0, recursive_count: 0, evidence, warnings: [], mtime_ms: null };
}

describe("bounded squarified Analyze treemap", () => {
  it("is deterministic for equal values, rounded bounds, and resize", () => {
    const children = Array.from({ length: 8 }, (_, index) => node(index + 1, 100));
    const first = layoutTreemap(children, { width: 801.4, height: 399.7 });
    const second = layoutTreemap(children, { width: 801.4, height: 399.7 });
    const resized = layoutTreemap(children, { width: 400, height: 800 });

    expect(first).toEqual(second);
    expect(first).toHaveLength(8);
    expect(first.every((tile) => tile.width > 0 && tile.height > 0)).toBe(true);
    expect(resized).not.toEqual(first);
    expect(first.reduce((sum, tile) => sum + tile.bytes, 0)).toBe(800);
  });

  it("handles zero, unknown, extreme, and unrenderably tiny sizes without invented area", () => {
    expect(layoutTreemap([node(1, 0), node(2, 10, "unknown")], { width: 600, height: 400 })).toEqual([]);
    const tiles = layoutTreemap([node(1, Number.MAX_SAFE_INTEGER - 10), node(2, 1), node(3, 9)], { width: 600, height: 400 });
    const other = tiles.find((tile) => tile.other);
    expect(other).toMatchObject({ bytes: 10, representedCount: 2 });
    expect(tiles.reduce((sum, tile) => sum + tile.bytes, 0)).toBe(Number.MAX_SAFE_INTEGER);
  });

  it("caps a 10,000-child directory at 512 nodes plus exact Other", () => {
    const children = Array.from({ length: 10_000 }, (_, index) => node(index + 1, 10_000 - index));
    const tiles = layoutTreemap(children, { width: 1200, height: 700 });
    const other = tiles.find((tile) => tile.other)!;
    const expectedOtherBytes = children.slice(MAX_TREEMAP_CHILDREN).reduce((sum, item) => sum + item.bytes, 0);

    expect(tiles.length).toBeLessThanOrEqual(513);
    expect(other).toMatchObject({ other: true, representedCount: 10_000 - MAX_TREEMAP_CHILDREN, bytes: expectedOtherBytes });
    expect(tiles.reduce((sum, tile) => sum + tile.bytes, 0)).toBe(children.reduce((sum, item) => sum + item.bytes, 0));
  });
});
