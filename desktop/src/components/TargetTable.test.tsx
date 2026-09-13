import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TargetTable } from "./TargetTable";
import type { ScanPreviewTarget } from "../api/types.gen";

const target: ScanPreviewTarget = {
  id: "node.node_modules:C:/fixture/app/node_modules",
  scope: { type: "project", root: "C:/fixture/app" },
  ecosystem: "node",
  kind: "dependency_directory",
  path: "C:/fixture/app/node_modules",
  estimated_bytes: 4096,
  size_complete: true,
  sizing_warnings: [],
  last_modified: null,
  risk: "medium",
  disposition: "candidate",
  evidence: [{ type: "marker_file", path: "C:/fixture/app/package.json" }],
};

describe("TargetTable headers", () => {
  it("uses Chinese catalogue headers instead of English Target Category Capacity Risk Evidence", () => {
    render(<TargetTable locale="zh-CN" mode="preview" targets={[target]} />);
    expect(screen.getByRole("columnheader", { name: "目标" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "容量" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "风险" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "证据" })).toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "类别" })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "Target" })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "Category" })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "Capacity" })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "Risk" })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "Evidence" })).not.toBeInTheDocument();
  });
});
