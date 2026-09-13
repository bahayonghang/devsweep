import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it, vi } from "vitest";
import type { DesktopBridge } from "../api/bridge";
import { fixtureBridge } from "../api/fixture-bridge";
import { HistoryPage } from "./HistoryPage";
import { ProtectionPage } from "./ProtectionPage";
import { RulesPage } from "./RulesPage";
import { decodeHistoryList } from "./decode";

function bridge(overrides: Partial<DesktopBridge> = {}): DesktopBridge {
  return { ...fixtureBridge, ...overrides };
}

describe("support views", () => {
  it("keeps protection add behind confirmation", async () => {
    const user = userEvent.setup();
    const protectionAdd = vi.fn();
    render(<ProtectionPage bridge={bridge({
      protectionList: vi.fn().mockResolvedValue(["C:/keep"]),
      protectionAdd,
    })} locale="en" />);
    expect(await screen.findByText("Protected paths: 1.")).toBeInTheDocument();
    await user.type(screen.getByLabelText("Add path"), "C:/other");
    await user.click(screen.getByRole("button", { name: "Add path" }));
    expect(await screen.findByRole("heading", { name: /Add this exact path/ })).toBeInTheDocument();
    expect(protectionAdd).not.toHaveBeenCalled();
  });

  it("renders Chinese protection unavailable copy", async () => {
    render(<ProtectionPage bridge={bridge({
      protectionList: vi.fn().mockRejectedValue({ code: "protection_store_unavailable", message: "corrupt" }),
    })} locale="zh-CN" />);
    expect(await screen.findByText(/保护存储不可用/)).toBeInTheDocument();
  });

  it("shows inspect-only rules without execute controls", async () => {
    render(<RulesPage bridge={bridge({
      rulesList: vi.fn().mockResolvedValue([{
        id: "rust.target",
        source: "shipped_registry",
        ecosystem: "rust",
        scope: "project",
        safety_class: "command",
        risk: "medium",
        platform_applicability: "all",
        inspect_only: false,
        rationale: "cargo clean",
      }]),
    })} locale="en" />);
    expect(await screen.findByText("Inspect only. Rules cannot be edited, imported, or executed here.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /execute/i })).not.toBeInTheDocument();
  });

  it("proves history rows cannot execute or replay and redacts forbidden fields", async () => {
    const listed = {
      operations: [{
        domain: "clean" as const,
        operation_id: "op-protect-v1-fixture",
        first_timestamp_unix_ms: 1,
        last_timestamp_unix_ms: 1,
        outcome_code: "committed",
        stable_codes: ["committed"],
        partial: false,
        unknown: false,
        unsupported: false,
      }],
      stores: [{ domain: "clean" as const, state: "available" as const, reason_code: null }],
    };
    expect(() => decodeHistoryList({
      operations: [{ ...listed.operations[0], path: "C:/secret" }],
      stores: listed.stores,
    })).toThrow("forbidden");
    render(<HistoryPage bridge={bridge({
      historyList: vi.fn().mockResolvedValue(listed),
      historyShow: vi.fn().mockResolvedValue({
        summary: listed.operations[0],
        records: [{ history_kind: "clean", record: { record_kind: "protection_mutation" } }],
      }),
    })} locale="en" />);
    expect(await screen.findByText("History is inspect-only. DevSweep cannot execute or replay a listed operation.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /execute|replay|run/i })).not.toBeInTheDocument();
  });

  it("renders Chinese history inspect-only copy without replay controls", async () => {
    render(<HistoryPage bridge={bridge({
      historyList: vi.fn().mockResolvedValue({ operations: [], stores: [] }),
    })} locale="zh-CN" />);
    expect(await screen.findByText(/历史仅供检查/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /执行|重放|execute|replay/i })).not.toBeInTheDocument();
  });
});

describe("support styles", () => {
  it("declares keyboard-visible forced-colors, reduced motion, and narrow/wide layouts", () => {
    const css = readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), "styles.css"), "utf8");
    expect(css).toContain("@media (forced-colors: active)");
    expect(css).toContain("@media (prefers-reduced-motion: reduce)");
    expect(css).toContain("@media (max-width: 800px)");
    expect(css).toContain("@media (min-width: 1440px)");
    expect(css).toContain(":focus-visible");
  });
});
