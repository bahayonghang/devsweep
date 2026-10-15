import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import fixture from "../api/fixtures/scan-report.json";
import { decodeScanReport } from "../api/contract";
import { cleanReducer, initialCleanState } from "../modes/clean/reducer";
import { ReviewPage } from "./ReviewPage";

const report = decodeScanReport(fixture);
const noOp = () => undefined;

function reviewedState() {
  const scanned = cleanReducer(initialCleanState, { type: "scan_requested", scanId: "s1" });
  return cleanReducer(scanned, { type: "scan_completed", scanId: "s1", report });
}

describe("ReviewPage", () => {
  it("bounds 10k distinct owner groups to 100 rows while searching and paging the full report", async () => {
    const original = reviewedState();
    const template = original.scan!.plan.targets[0];
    const targets = Array.from({ length: 10000 }, (_, index) => ({ ...template, id: `target-${String(index).padStart(5, "0")}`, path: `C:/work/project-${index}/target`, scope: { type: "project" as const, root: `C:/work/project-${index}` } }));
    const state = { ...original, scan: { ...original.scan!, plan: { ...original.scan!.plan, targets } }, selectedIds: new Set([targets[9999].id]) };
    const view = render(<ReviewPage state={state} onSelect={noOp} onSelectAll={noOp} onDryRun={noOp} />);
    expect(document.querySelectorAll("tbody tr")).toHaveLength(100);
    expect(document.querySelectorAll(".preview-group")).toHaveLength(100);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Last page" }));
    expect(screen.getByRole("checkbox", { name: "Select C:/work/project-9999/target" })).toBeChecked();
    view.rerender(<ReviewPage state={state} query="project-9999/target" onSelect={noOp} onSelectAll={noOp} onDryRun={noOp} />);
    expect(document.querySelectorAll("tbody tr")).toHaveLength(1);
    expect(screen.getByRole("checkbox", { name: "Select C:/work/project-9999/target" })).toBeChecked();
    expect(screen.getByText("Scan complete. 10000 targets.")).toBeInTheDocument();
  });
  it("orders groups by impact rank then size, keeps Inspect Only unselectable, and uses one page-level select-all", () => {
    const state = reviewedState();
    const [build, inspect] = state.scan!.plan.targets;
    const splitKind = {
      ...state,
      scan: {
        ...state.scan!,
        plan: {
          ...state.scan!.plan,
          targets: [build, inspect, { ...build, id: "cargo.target:global", scope: { type: "global" as const } }],
        },
      },
    };
    render(<ReviewPage state={splitKind} onSelect={noOp} onSelectAll={noOp} onDryRun={noOp} />);
    const kinds = [...document.querySelectorAll(".preview-group h3")].map((node) => node.childNodes[0].textContent?.trim());
    const scopes = [...document.querySelectorAll(".preview-group header p")].map((node) => node.textContent);
    expect(kinds).toEqual(["cargo.home.inspect", "cargo.target", "C:/work/app"]);
    expect(scopes).toEqual(["Package caches · Global caches", "Build artifacts · Global caches", "Build artifacts · Projects"]);
    expect(screen.getAllByRole("checkbox", { name: "Select all executable targets" })).toHaveLength(1);
    expect(screen.getByRole("checkbox", { name: /C:\/Users\/dev\/\.cargo/ })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: /C:\/Users\/dev\/\.cargo/ })).toHaveAttribute("title", "Inspect-only targets cannot be selected.");
    expect(document.querySelector(".capacity-meter [style]")).toBeNull();
    expect(document.querySelector(".capacity-meter svg")).toHaveAttribute("preserveAspectRatio", "none");
  });

  it("uses Chinese kind and risk labels instead of raw keys", () => {
    render(<ReviewPage locale="zh-CN" state={reviewedState()} onSelect={noOp} onSelectAll={noOp} onDryRun={noOp} />);
    const kinds = [...document.querySelectorAll(".preview-group h3")].map((node) => node.childNodes[0].textContent?.trim());
    expect(kinds).toEqual(["npm.cache.clean", "cargo.home.inspect", "C:/work/app"]);
    expect(screen.getByText("构建产物 · 项目")).toBeInTheDocument();
    expect(screen.getByText("低")).toBeInTheDocument();
    expect(screen.getByText("高")).toBeInTheDocument();
    expect(screen.getByText("中")).toBeInTheDocument();
    expect(screen.queryByText("build_artifacts")).not.toBeInTheDocument();
    expect(screen.queryByText("dangerous")).not.toBeInTheDocument();
    expect(screen.queryByText("high")).not.toBeInTheDocument();
  });

  it("renders an empty completed report on the stage card without the light empty heading", () => {
    const state = reviewedState();
    const empty = {
      ...state,
      scan: { ...state.scan!, plan: { ...state.scan!.plan, targets: [] }, health: { completeness: "complete" as const, diagnostics: [], totals: { verified_bytes: 0, partial_lower_bound_bytes: 0, unknown_target_count: 0 } } },
    };
    render(<ReviewPage state={empty} onSelect={noOp} onSelectAll={noOp} onDryRun={noOp} />);
    expect(screen.getByText("Scan complete; no cleanup targets found.")).toBeInTheDocument();
    expect(screen.queryByText("No scan results")).not.toBeInTheDocument();
    expect(document.querySelector(".sweep-body-hero")).not.toBeInTheDocument();
    expect(document.querySelector(".clean-stage")).toBeInTheDocument();
  });
});
