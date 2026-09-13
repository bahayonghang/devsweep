import { render, screen } from "@testing-library/react";
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
  it("groups by kind then scope, keeps Inspect Only unselectable, and uses one page-level select-all", () => {
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
    expect(kinds).toEqual(["Build artifacts", "Build artifacts", "Package caches"]);
    expect(scopes).toEqual(["Projects", "Global caches", "Global caches"]);
    expect(screen.getAllByRole("checkbox", { name: "Select all executable targets" })).toHaveLength(1);
    expect(screen.getByRole("checkbox", { name: /C:\/Users\/dev\/\.cargo/ })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: /C:\/Users\/dev\/\.cargo/ })).toHaveAttribute("title", "Inspect-only targets cannot be selected.");
    expect(document.querySelector(".capacity-meter [style]")).toBeNull();
    expect(document.querySelector(".capacity-meter svg")).toHaveAttribute("preserveAspectRatio", "none");
  });

  it("uses Chinese kind and risk labels instead of raw keys", () => {
    render(<ReviewPage locale="zh-CN" state={reviewedState()} onSelect={noOp} onSelectAll={noOp} onDryRun={noOp} />);
    const kinds = [...document.querySelectorAll(".preview-group h3")].map((node) => node.childNodes[0].textContent?.trim());
    expect(kinds).toEqual(["构建产物", "包缓存"]);
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
      scan: { ...state.scan!, plan: { ...state.scan!.plan, targets: [] } },
    };
    render(<ReviewPage state={empty} onSelect={noOp} onSelectAll={noOp} onDryRun={noOp} />);
    expect(screen.getByText("Scan complete; no cleanup targets found.")).toBeInTheDocument();
    expect(screen.queryByText("No scan results")).not.toBeInTheDocument();
    expect(document.querySelector(".sweep-body-hero")).not.toBeInTheDocument();
    expect(document.querySelector(".clean-stage")).toBeInTheDocument();
  });
});
