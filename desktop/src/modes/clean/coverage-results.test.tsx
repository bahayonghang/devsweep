import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { decodeExecutionReport, decodeScanReport } from "../../api/contract";
import coverageFixture from "../../api/fixtures/clean/scan-coverage.json";
import executionFixture from "../../api/fixtures/clean/execution-report.json";
import { ExecutePage } from "../../pages/ExecutePage";
import { ReviewPage } from "../../pages/ReviewPage";
import { initialState } from "../../state/app-state";
import { ScanPage } from "../../pages/ScanPage";

describe("truthful Clean coverage and outcomes", () => {
  it("shows coverage and diagnostics for partial zero-target reports", () => {
    const report = decodeScanReport(coverageFixture);
    render(<ReviewPage state={{ ...initialState, phase: "reviewed", scan: report }} onSelect={() => undefined} onSelectAll={() => undefined} onDryRun={() => undefined} />);
    expect(screen.getByText("Controlled timeout coverage fixture")).toBeVisible();
    expect(screen.getByText("Controlled ownership_unknown coverage fixture")).toBeVisible();
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(/incomplete/i);
  });
  it("requires explicit roots for project scans while global-only remains valid", () => {
    const props = { activeScan: null, busy: false, onOptions: () => undefined, onScan: () => undefined, onCancel: () => undefined };
    const view = render(<ScanPage {...props} options={{ include_projects: true, include_global: true, roots: [] }} />);
    expect(screen.getByRole("button", { name: "Scan" })).toBeDisabled();
    view.rerender(<ScanPage {...props} options={{ include_projects: false, include_global: true, roots: [] }} />);
    expect(screen.getByRole("button", { name: "Scan" })).toBeEnabled();
  });
  it("does not label successful owner commands as bytes moved to the Recycle Bin", () => {
    const report = decodeExecutionReport(executionFixture);
    const command = report.outcomes.find((outcome) => outcome.action.type === "command")!;
    report.outcomes = [command]; report.selected = 1; report.attempted = 1; report.succeeded = 1;
    render(<ExecutePage report={decodeExecutionReport(report)} final onConfirm={() => undefined} onReturn={() => undefined} />);
    expect(screen.getByText(/Owner commands completed: 1/)).toBeInTheDocument();
    expect(screen.queryByText("Moved to trash (not yet freed)")).not.toBeInTheDocument();
    expect(document.querySelector(".display-capacity")).toBeNull();
  });
  it("keeps all-skipped outcomes and their reason visible", () => {
    const report = decodeExecutionReport(executionFixture);
    report.outcomes = [{ ...report.outcomes[0], status: { type: "skipped", reason: "owner process busy" } }];
    report.selected = 1; report.attempted = 1; report.succeeded = 0; report.skipped = 1;
    render(<ExecutePage report={decodeExecutionReport(report)} final onConfirm={() => undefined} onReturn={() => undefined} />);
    expect(screen.getByRole("heading")).toHaveTextContent("No successful cleanup actions");
    expect(screen.getByText(/owner process busy/)).toBeInTheDocument();
    expect(document.querySelector(".display-capacity")).toBeNull();
  });
});
