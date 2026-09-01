import { describe, expect, it } from "vitest";
import type { AnalyzeNodeV1, AnalyzeSnapshotV1 } from "../../api/types.gen";
import { ANALYZE_PAGE_SIZE, createAnalyzeIndex, pageForNode, selectBreadcrumbs, selectDirectory, selectPage, selectVisibleChildren } from "./selectors";

function node(id: number, parentId: number | null, name: string, bytes: number, kind: AnalyzeNodeV1["kind"] = "file"): AnalyzeNodeV1 {
  return { id, parent_id: parentId, kind, name, bytes, immediate_count: kind === "directory" ? 0 : 0, recursive_count: 0, evidence: "complete", warnings: [], mtime_ms: null };
}

function snapshot(nodes: AnalyzeNodeV1[]): AnalyzeSnapshotV1 {
  return { version: 1, root: { input: "C:/root", normalized: "C:/root", volume: "vol" }, nodes, warnings: [], completeness: "complete", accounted_owned_bytes: 0 };
}

describe("Analyze selectors", () => {
  it("selects one immutable directory, deterministic search/sort, and 200-row pages", () => {
    const children = Array.from({ length: 450 }, (_, index) => node(index + 1, 0, `file-${String(index).padStart(3, "0")}`, index));
    const root = { ...node(0, null, "root", 0, "directory"), immediate_count: children.length };
    const source = snapshot([root, ...children]);
    const index = createAnalyzeIndex(source);
    const selection = selectDirectory(index, 0);
    const sorted = selectVisibleChildren(selection.children, "file-", "size_desc");
    const cached = selectVisibleChildren(selection.children, "", "size_desc");
    const third = selectPage(sorted, 2);

    expect(selection.children).toHaveLength(450);
    expect(sorted[0].bytes).toBe(449);
    expect(third.rows).toHaveLength(50);
    expect(third.pageCount).toBe(3);
    expect(pageForNode(sorted, third.rows[0].id)).toBe(2);
    expect(source.nodes[1].name).toBe("file-000");
    expect(ANALYZE_PAGE_SIZE).toBe(200);
    expect(selectVisibleChildren(selection.children, "", "size_desc")).toBe(cached);
    expect(sorted.map((item) => item.id)).toEqual(cached.map((item) => item.id));
  });

  it("builds root-to-directory breadcrumbs and falls back to the root", () => {
    const root = { ...node(0, null, "root", 3, "directory"), immediate_count: 1 };
    const child = { ...node(1, 0, "child", 2, "directory"), immediate_count: 1 };
    const leaf = node(2, 1, "leaf", 2);
    const index = createAnalyzeIndex(snapshot([root, child, leaf]));

    expect(selectBreadcrumbs(index, 1).map((item) => item.name)).toEqual(["root", "child"]);
    expect(selectDirectory(index, 2).directory.id).toBe(0);
  });
});
