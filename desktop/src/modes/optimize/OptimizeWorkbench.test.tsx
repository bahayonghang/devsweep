import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { fixtureBridge } from "../../api/fixture-bridge";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { OptimizeWorkbench } from "./OptimizeWorkbench";

const optimizeStyles = readFileSync(resolve(process.cwd(), "src/modes/optimize/styles.css"), "utf8");

describe("OptimizeWorkbench", () => {
  it("renders closed badges, refuses guidance run, and requires Settings confirmation", async () => {
    const user = userEvent.setup();
    render(<OptimizeWorkbench bridge={fixtureBridge} coordinator={new OperationCoordinator()} locale="en" />);
    await user.click(screen.getByRole("button", { name: "Reload catalogue" }));
    expect((await screen.findAllByText("dns.flush")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("Runs here").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Opens Windows Settings").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Guidance only").length).toBeGreaterThan(0);
    expect(screen.getAllByText("No run action").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("radio", { name: /guidance.drive_optimize: Guidance only/ }));
    expect(screen.queryByRole("button", { name: "Review preview" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Run DNS flush" })).not.toBeInTheDocument();
    expect(screen.getAllByText("Guidance shown. This is not an optimization completion.").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("radio", { name: /settings.search: Opens Windows Settings/ }));
    await user.click(screen.getByRole("button", { name: "Review preview" }));
    expect((await screen.findAllByText(/sha256:bbbbbbbb/)).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "Continue to confirmation" }));
    await waitFor(() => expect(screen.getByRole("dialog")).toHaveAttribute("open"));
    expect(screen.getByRole("button", { name: "Open Windows Settings" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Run DNS flush" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Open Windows Settings" }));
    expect(await screen.findByText(/Launched \(Settings page opened\)/)).toBeInTheDocument();
    expect(screen.getAllByText("Settings launched. This is not a completed optimization.").length).toBeGreaterThan(0);
    expect(document.body.textContent?.toLowerCase()).not.toContain("optimization complete");
  });

  it("renders Simplified Chinese copy without inventing a run action", async () => {
    const user = userEvent.setup();
    render(<OptimizeWorkbench bridge={fixtureBridge} coordinator={new OperationCoordinator()} locale="zh-CN" />);
    await user.click(screen.getByRole("button", { name: "重新加载目录" }));
    expect((await screen.findAllByText("dns.flush")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("在此运行").length).toBeGreaterThan(0);
    expect(screen.getAllByText("打开 Windows 设置").length).toBeGreaterThan(0);
    expect(screen.getAllByText("仅指引").length).toBeGreaterThan(0);
    expect(screen.getAllByText("无运行操作").length).toBeGreaterThan(0);
  });

  it("locks reduced motion, target widths, and high-contrast without claiming completion", () => {
    expect(optimizeStyles).toContain("@media (prefers-reduced-motion: reduce)");
    expect(optimizeStyles).toContain("@media (max-width: 800px)");
    expect(optimizeStyles).toContain("@media (max-width: 430px)");
    expect(optimizeStyles).toContain("@media (forced-colors: active)");
    expect(optimizeStyles).toContain("animation: none !important");
    expect(optimizeStyles).toContain("--surface-raised: var(--raised);");
    expect(optimizeStyles).toContain(".optimize-mode .secondary-button { color: var(--text); background: var(--canvas);");
    expect(optimizeStyles).toContain(".optimize-confirm-dialog::backdrop");
    expect(optimizeStyles).not.toMatch(/#fff|#ffffff/i);
    expect(optimizeStyles).not.toMatch(/optimization complete/i);
  });
});
