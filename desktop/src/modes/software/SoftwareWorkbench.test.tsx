import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { fixtureBridge } from "../../api/fixture-bridge";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { SoftwareWorkbench } from "./SoftwareWorkbench";

const softwareStyles = readFileSync(resolve(process.cwd(), "src/modes/software/styles.css"), "utf8");

describe("SoftwareWorkbench", () => {
  it("renders manual rows, exact identity detail, preview, and second confirmation", async () => {
    const user = userEvent.setup();
    render(<SoftwareWorkbench bridge={fixtureBridge} coordinator={new OperationCoordinator()} locale="en" />);
    await user.click(screen.getByRole("button", { name: "Refresh inventory" }));
    expect((await screen.findAllByText("Contoso Tools")).length).toBeGreaterThan(0);
    const manual = screen.getByRole("checkbox", { name: /Contoso Tools: MSI uninstall is manual/ });
    expect(manual).toBeDisabled();
    const eligible = screen.getByRole("checkbox", { name: /Contoso Tools: Eligible current-user MSIX/ });
    await user.click(eligible);
    await user.click(screen.getByRole("button", { name: "Review uninstall preview" }));
    expect((await screen.findAllByText(/sha256:555555/)).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "Continue to confirmation" }));
    await waitFor(() => expect(screen.getByRole("dialog")).toHaveAttribute("open"));
    expect(screen.getAllByText(/MSIX:Contoso.Tools_1.0.0.0/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/DevSweep cannot restore or reinstall/).length).toBeGreaterThan(0);
  });

  it("renders Simplified Chinese copy without resetting fixture identity", async () => {
    const user = userEvent.setup();
    render(<SoftwareWorkbench bridge={fixtureBridge} coordinator={new OperationCoordinator()} locale="zh-CN" />);
    await user.click(screen.getByRole("button", { name: "刷新软件清单" }));
    expect((await screen.findAllByText("Contoso Tools")).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Software V1 仅手动处理 MSI 卸载/).length).toBeGreaterThan(0);
  });

  it("locks canvas tokens, sticky summary, and no light surface palette", () => {
    expect(softwareStyles).toContain("--surface-raised: var(--raised);");
    expect(softwareStyles).toContain(".software-summary-bar");
    expect(softwareStyles).toContain("position: sticky");
    expect(softwareStyles).toContain(".software-mode .secondary-button { color: var(--text); background: var(--canvas);");
    expect(softwareStyles).toContain("@media (prefers-reduced-motion: reduce)");
    expect(softwareStyles).toContain("@media (forced-colors: active)");
    expect(softwareStyles).not.toMatch(/#fff|#ffffff/i);
  });
});
