import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TargetTable } from "./TargetTable";
import type { ScanPreviewTarget, UntrustedTarget } from "../api/types.gen";
import reportFixture from "../api/fixtures/scan-report.json";
import { decodeScanReport } from "../api/contract";

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
  it("pages real rows with keyboard controls while preserving complete paths and review exclusions", async () => {
    const user = userEvent.setup();
    const facts = decodeScanReport(reportFixture).plan.targets[0];
    const targets: UntrustedTarget[] = Array.from({ length: 205 }, (_, index) => {
      return { ...facts, id: `target-${index}`, path: `C:/fixture/项目 ${index}/node_modules`,
        rule_id: "node.node_modules", reversible: true, selected_by_default: false,
        intent: { type: index === 204 ? "inspect_only" : "trash_project_artifact", rule_id: "node.node_modules" } };
    });
    const onSelect = vi.fn();
    const props = { mode: "review" as const, targets, selectedIds: new Set(["target-0", "target-201"]), protectedIds: new Set(["target-203"]), allSelected: false, disabled: false, onSelect, onSelectAll: vi.fn() };
    const { rerender } = render(<TargetTable {...props} />);
    expect(document.querySelectorAll("tbody tr")).toHaveLength(100);
    expect(screen.getByRole("checkbox", { name: "Select C:/fixture/项目 0/node_modules" })).toBeChecked();
    const next = screen.getByRole("button", { name: "Next page" });
    next.focus();
    await user.keyboard("{Enter}");
    expect(screen.getByText("Page 2 of 3; targets 101–200 of 205.")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "Select C:/fixture/项目 100/node_modules" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "Last page" }));
    expect(document.querySelectorAll("tbody tr")).toHaveLength(5);
    expect(screen.getByRole("checkbox", { name: "Select C:/fixture/项目 201/node_modules" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select C:/fixture/项目 203/node_modules" })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: "Select C:/fixture/项目 204/node_modules" })).toBeDisabled();
    expect(screen.getByText("C:/fixture/项目 204/node_modules")).toHaveAttribute("title", "C:/fixture/项目 204/node_modules");
    await user.click(screen.getByRole("checkbox", { name: "Select C:/fixture/项目 202/node_modules" }));
    expect(onSelect).toHaveBeenCalledExactlyOnceWith("target-202", true);
    await user.click(screen.getByRole("button", { name: "First page" }));
    expect(screen.getByRole("checkbox", { name: "Select C:/fixture/项目 0/node_modules" })).toBeChecked();
    await user.click(screen.getByRole("button", { name: "Last page" }));
    rerender(<TargetTable {...props} targets={targets.slice(0, 103)} />);
    expect(screen.getByText("Page 2 of 2; targets 101–103 of 103.")).toBeInTheDocument();
    expect(document.querySelectorAll("tbody tr")).toHaveLength(3);
  });

  it("mounts complete evidence and sizing warnings only while their native disclosure is open", () => {
    const warning = "Controlled sizing warning";
    render(<TargetTable mode="preview" targets={[{ ...target, sizing_warnings: [{ kind: "metadata_unavailable", detail: warning }] }]} />);
    const details = document.querySelector("tbody details")!;
    expect(details.querySelector("ul")).toBeNull();
    expect(screen.queryByText(warning)).not.toBeInTheDocument();
    details.setAttribute("open", "");
    fireEvent(details, new Event("toggle"));
    expect(details.querySelectorAll("li")).toHaveLength(1);
    expect(details.querySelector("li")).toHaveTextContent("C:/fixture/app/package.json");
    expect(screen.getByText(warning)).toBeInTheDocument();
    details.removeAttribute("open");
    fireEvent(details, new Event("toggle"));
    expect(details.querySelector("ul")).toBeNull();
    expect(screen.queryByText(warning)).not.toBeInTheDocument();
  });

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
