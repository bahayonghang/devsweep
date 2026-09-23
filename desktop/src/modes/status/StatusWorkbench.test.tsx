import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { fixtureBridge } from "../../api/fixture-bridge";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { StatusWorkbench } from "./StatusWorkbench";

const statusStyles = readFileSync(resolve(process.cwd(), "src/modes/status/styles.css"), "utf8");

describe("StatusWorkbench", () => {
  it("loads a snapshot first, starts live explicitly, and never shows GPU zero cards", async () => {
    const user = userEvent.setup();
    render(<StatusWorkbench bridge={fixtureBridge} coordinator={new OperationCoordinator()} locale="en" />);
    expect(await screen.findByRole("heading", { level: 2, name: /CPU\s*43\.21\s*%/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start live" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Show details" }));
    expect(await screen.findByText(/Status snapshot snapshot-fixture/)).toBeInTheDocument();
    expect(screen.getByText(/CPU: 43.21%/)).toBeInTheDocument();
    expect(screen.getByText(/No battery present/)).toBeInTheDocument();
    expect(screen.getByText(/Truncated by limit/)).toBeInTheDocument();
    expect(screen.getByText(/Status V1 does not include GPU/)).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/gpu 0|cmdline/i);
    expect(screen.getByRole("button", { name: "Start live" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Start live" }));
    expect(await screen.findByRole("status", { name: "Live" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stop live" })).toBeInTheDocument();
    expect(screen.getAllByText(/gap/).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "Stop live" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start live" })).toBeInTheDocument());
  });

  it("renders Simplified Chinese snapshot copy without inventing GPU values", async () => {
    const user = userEvent.setup();
    render(<StatusWorkbench bridge={fixtureBridge} coordinator={new OperationCoordinator()} locale="zh-CN" />);
    await user.click(await screen.findByRole("button", { name: "查看详情" }));
    expect(await screen.findByText(/状态快照 snapshot-fixture/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "开始实时" })).toBeInTheDocument();
    expect(screen.getByText(/无电池/)).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/GPU 0/i);
  });

  it("locks reduced motion, target widths, and high-contrast without GPU zero cards", () => {
    expect(statusStyles).toContain("@media (prefers-reduced-motion: reduce)");
    expect(statusStyles).toContain("@media (max-width: 800px)");
    expect(statusStyles).toContain("@media (max-width: 430px)");
    expect(statusStyles).toContain("@media (forced-colors: active)");
    expect(statusStyles).toContain("animation: none !important");
    expect(statusStyles).toContain("--surface-raised: var(--raised);");
    expect(statusStyles).toContain(".status-mode .secondary-button { color: var(--text); background: var(--canvas);");
    expect(statusStyles).not.toMatch(/#fff|#ffffff/i);
    expect(statusStyles).not.toMatch(/gpu 0/i);
  });
});
