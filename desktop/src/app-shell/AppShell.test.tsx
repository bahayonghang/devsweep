import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { OperationCoordinator } from "../state/operation-coordinator";
import {
  AccessibleUserData,
  AppShell,
  MODE_IDS,
  SUPPORTING_DESTINATION_IDS,
  parseModeRoute,
  parseShellRoute,
  type ModeRegistration,
  type SupportingDestinationRegistration,
} from "./AppShell";

const registrations: ModeRegistration[] = MODE_IDS.map((id) => ({ id, render: () => <p>{id} content</p> }));
const supporting: SupportingDestinationRegistration[] = SUPPORTING_DESTINATION_IDS.map((id) => ({
  id,
  render: () => <p>{id} support content</p>,
}));

describe("AppShell", () => {
  beforeEach(() => window.history.replaceState(null, "", "#/clean"));

  it("keeps the frozen five-mode identity but omits unavailable registrations", () => {
    render(<AppShell modes={[registrations[0]]} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    expect(MODE_IDS).toEqual(["clean", "software", "optimize", "analyze", "status"]);
    expect(screen.getAllByRole("tab")).toHaveLength(1);
    expect(screen.getByRole("tab", { name: "Clean" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Software" })).not.toBeInTheDocument();
    expect(SUPPORTING_DESTINATION_IDS).toEqual(["protection", "rules", "history"]);
    expect(screen.queryByRole("button", { name: "Protection" })).not.toBeInTheDocument();
    expect(screen.queryByText("More")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Language" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Help" })).toBeInTheDocument();
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-mode", "clean");
    expect(document.querySelector(".shell-sidebar")).toBeInTheDocument();
  });

  it("supports exact deep links, keyboard navigation, focus restoration, and unique accelerators", async () => {
    window.history.replaceState(null, "", "#/status");
    const user = userEvent.setup();
    render(<AppShell modes={registrations} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    expect(screen.getByText("status content")).toBeInTheDocument();
    expect(document.body).toHaveFocus();
    const status = screen.getByRole("tab", { name: "Status" });
    status.focus();
    await user.keyboard("{Home}");
    expect(screen.getByRole("tab", { name: "Clean" })).toHaveFocus();
    fireEvent.keyDown(window, { key: "s", altKey: true });
    await waitFor(() => expect(screen.getByText("software content")).toBeInTheDocument());
    expect(screen.getByRole("tab", { name: "Software" })).toHaveFocus();
    expect(window.location.hash).toBe("#/software");
  });

  it("shows every destination in the sidebar without a More disclosure", () => {
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    for (const name of ["Clean", "Software", "Optimize", "Analyze", "Status"]) {
      expect(screen.getByRole("tab", { name })).toBeInTheDocument();
    }
    for (const name of ["Protection", "Rules", "History", "Language"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    expect(screen.getByRole("link", { name: "Help" })).toBeInTheDocument();
    expect(screen.getByText("Modes")).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "Supporting destinations" })).toBeInTheDocument();
    expect(screen.getByRole("tablist")).toHaveAttribute("aria-orientation", "vertical");
    expect(screen.queryByText("More")).not.toBeInTheDocument();
    expect(screen.queryByText("更多")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: "Clean" })).toBeVisible();
    expect(screen.getByText("Review every Cleanup Target before anything moves.")).toBeVisible();
  });

  it("restores the actual primary and supporting activators after composition", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);

    const status = screen.getByRole("tab", { name: "Status" });
    await user.click(status);
    expect(await screen.findByText("status content")).toBeInTheDocument();
    expect(status).toHaveFocus();

    const history = screen.getByRole("button", { name: "History" });
    await user.click(history);
    expect(await screen.findByText("history support content")).toBeInTheDocument();
    expect(history).toHaveFocus();
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-mode", "shell");
  });

  it("restores a supporting deep link onto the History button", async () => {
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    window.history.pushState(null, "", "#/history");
    fireEvent.popState(window);
    expect(await screen.findByText("history support content")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "History" })).toHaveFocus();
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-mode", "shell");
    expect(screen.getByRole("heading", { level: 1, name: "History" })).toBeVisible();
  });

  it("moves vertical tab focus with ArrowDown and ArrowUp", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    const clean = screen.getByRole("tab", { name: "Clean" });
    clean.focus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("tab", { name: "Software" })).toHaveFocus();
    await user.keyboard("{ArrowUp}");
    expect(clean).toHaveFocus();
    await user.keyboard("{ArrowRight}");
    expect(screen.getByRole("tab", { name: "Software" })).toHaveFocus();
    await user.keyboard("{ArrowLeft}");
    expect(clean).toHaveFocus();
  });

  it("shows a visible title and subtitle on every route", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    const routes = [
      { name: "Clean", role: "tab" as const, title: "Clean", subtitle: "Review every Cleanup Target before anything moves." },
      { name: "Software", role: "tab" as const, title: "Software", subtitle: "Review current-user software inventory before uninstall." },
      { name: "Optimize", role: "tab" as const, title: "Optimize", subtitle: "Review catalogued actions before any run." },
      { name: "Analyze", role: "tab" as const, title: "Analyze", subtitle: "Map disk use. This mode is read-only." },
      { name: "Status", role: "tab" as const, title: "Status", subtitle: "Host facts from collectors. No score." },
      { name: "Protection", role: "button" as const, title: "Protection", subtitle: "Inspect protection policy without changing cleanup authority." },
      { name: "Rules", role: "button" as const, title: "Rules", subtitle: "Inspect the rule catalogue." },
      { name: "History", role: "button" as const, title: "History", subtitle: "Inspect past cleanup records." },
      { name: "Language", role: "button" as const, title: "Language settings", subtitle: "Choose English or Simplified Chinese for this window." },
    ];
    for (const route of routes) {
      await user.click(screen.getByRole(route.role, { name: route.name }));
      expect(await screen.findByRole("heading", { level: 1, name: route.title })).toBeVisible();
      expect(screen.getByText(route.subtitle)).toBeVisible();
    }
  });

  it("shows a visible title and subtitle on every zh-CN route", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="zh-CN" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    const routes = [
      { name: "清理", role: "tab" as const, title: "清理", subtitle: "先审查每个清理目标，再移动任何内容。" },
      { name: "软件", role: "tab" as const, title: "软件", subtitle: "审查当前用户软件清单后再卸载。" },
      { name: "优化", role: "tab" as const, title: "优化", subtitle: "审查目录中的操作后再运行。" },
      { name: "分析", role: "tab" as const, title: "分析", subtitle: "映射磁盘占用。此模式只读。" },
      { name: "状态", role: "tab" as const, title: "状态", subtitle: "来自采集器的主机事实。无评分。" },
      { name: "保护", role: "button" as const, title: "保护", subtitle: "查看保护策略，不改变清理权限。" },
      { name: "规则", role: "button" as const, title: "规则", subtitle: "查看规则目录。" },
      { name: "历史", role: "button" as const, title: "历史", subtitle: "查看既往清理记录。" },
      { name: "语言", role: "button" as const, title: "语言设置", subtitle: "为本窗口选择英语或简体中文。" },
    ];
    for (const route of routes) {
      await user.click(screen.getByRole(route.role, { name: route.name }));
      expect(await screen.findByRole("heading", { level: 1, name: route.title })).toBeVisible();
      expect(screen.getByText(route.subtitle)).toBeVisible();
    }
  });

  it("renders Chinese canonical labels and writes language selection through the adapter", async () => {
    const save = vi.fn();
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="zh-CN" onLocaleChange={save} coordinator={new OperationCoordinator()} />);
    expect(screen.getByRole("tab", { name: "清理" })).not.toHaveAttribute("title");
    expect(screen.getByRole("tab", { name: "软件" })).toHaveAttribute("title", "Alt+R");
    expect(screen.getByText("模式")).toBeInTheDocument();
    expect(screen.queryByText("更多")).not.toBeInTheDocument();
    expect(screen.queryByText("More")).not.toBeInTheDocument();
    for (const name of ["清理", "软件", "优化", "分析", "状态"]) {
      expect(screen.getByRole("tab", { name })).toBeInTheDocument();
    }
    for (const name of ["保护", "规则", "历史", "语言"]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    expect(screen.getByRole("link", { name: "帮助" })).toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "支持目的地" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: "清理" })).toBeVisible();
    expect(screen.getByText("先审查每个清理目标，再移动任何内容。")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "语言" }));
    expect(await screen.findByRole("heading", { level: 1, name: "语言设置" })).toBeVisible();
    expect(screen.getByText("为本窗口选择英语或简体中文。")).toBeVisible();
    await user.selectOptions(screen.getByRole("combobox", { name: "语言" }), "en");
    expect(save).toHaveBeenCalledWith("en");
  });

  it("normalizes an unavailable initial deep link without exposing a placeholder", () => {
    window.history.replaceState(null, "", "#/software");
    render(<AppShell modes={[registrations[0]]} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    expect(screen.getByText("clean content")).toBeInTheDocument();
    expect(window.location.hash).toBe("#/clean");
    expect(screen.queryByRole("tab", { name: "Software" })).not.toBeInTheDocument();
  });

  it("accepts only registered closed mode and supporting routes", () => {
    expect(parseModeRoute("#/status", ["clean", "status"])).toBe("status");
    expect(parseModeRoute("#/status", ["clean"])).toBeNull();
    expect(parseModeRoute("#/unknown", MODE_IDS)).toBeNull();
    expect(parseModeRoute("javascript:alert(1)", MODE_IDS)).toBeNull();
    expect(parseShellRoute("#/history", ["clean"], ["history"])).toBe("support:history");
    expect(parseShellRoute("#/history", ["clean"], [])).toBeNull();
    expect(parseShellRoute("#/protection", ["clean"], ["rules"])).toBeNull();
    expect(parseShellRoute("#/settings", ["clean"], [])).toBe("settings");
  });

  it("pushes user routes and restores the Settings opener through actual Back", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);

    await user.click(screen.getByRole("button", { name: "History" }));
    expect(await screen.findByText("history support content")).toBeInTheDocument();
    expect(window.location.hash).toBe("#/history");
    const settingsOpener = screen.getByRole("button", { name: "Language" });
    await user.click(settingsOpener);
    expect(await screen.findByRole("combobox", { name: "Language" })).toBeInTheDocument();
    expect(window.location.hash).toBe("#/settings");

    act(() => window.history.back());
    await waitFor(() => expect(window.location.hash).toBe("#/history"));
    expect(await screen.findByText("history support content")).toBeInTheDocument();
    expect(settingsOpener).toHaveFocus();
  });

  it("uses the destination control when a deep-linked Settings route has no opener", async () => {
    window.history.replaceState(null, "", "#/settings");
    render(<AppShell modes={registrations} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    expect(screen.getByRole("combobox", { name: "Language" })).toBeInTheDocument();

    window.history.pushState(null, "", "#/clean");
    fireEvent.popState(window);

    expect(await screen.findByText("clean content")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Clean" })).toHaveFocus();
  });

  it("cancels and joins before activating a route and rejects a superseded route event", async () => {
    let release: () => void = () => undefined;
    const join = new Promise<void>((resolve) => { release = resolve; });
    const cancel = vi.fn();
    const coordinator = new OperationCoordinator();
    await coordinator.start({ kind: "clean.scan", id: "scan", cancel, start: () => join });
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={coordinator} />);

    await user.click(screen.getByRole("tab", { name: "Software" }));
    expect(cancel).toHaveBeenCalledOnce();
    expect(screen.getByText("clean content")).toBeInTheDocument();
    window.history.pushState(null, "", "#/status");
    fireEvent.popState(window);
    release();

    expect(await screen.findByText("status content")).toBeInTheDocument();
    expect(screen.queryByText("software content")).not.toBeInTheDocument();
    expect(window.location.hash).toBe("#/status");
    const status = screen.getByRole("tab", { name: "Status" });
    expect(status).toHaveFocus();

    const historyLength = window.history.length;
    window.history.pushState(null, "", "#/unavailable");
    fireEvent.popState(window);
    await waitFor(() => expect(window.location.hash).toBe("#/clean"));
    expect(window.history.state).toEqual({ route: "mode:clean" });
    expect(screen.getByText("clean content")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Clean" })).toHaveFocus();
    expect(window.history.length).toBe(historyLength + 1);
  });

  it("normalizes a runtime invalid hash through cancel-and-join without growing history", async () => {
    const events: string[] = [];
    let release: () => void = () => undefined;
    const coordinator = {
      cancelAndJoin: vi.fn(() => {
        events.push("cancel-and-join");
        return new Promise<void>((resolve) => {
          release = () => {
            events.push("joined");
            resolve();
          };
        });
      }),
    };
    render(<AppShell modes={registrations} locale="en" onLocaleChange={() => undefined} coordinator={coordinator} />);
    await userEvent.setup().click(screen.getByRole("tab", { name: "Status" }));
    release();
    expect(await screen.findByText("status content")).toBeInTheDocument();

    const historyLength = window.history.length;
    window.history.replaceState(null, "", "#/unavailable");
    fireEvent.popState(window);
    expect(screen.getByText("status content")).toBeInTheDocument();
    expect(window.location.hash).toBe("#/unavailable");
    release();

    await waitFor(() => expect(window.location.hash).toBe("#/clean"));
    expect(screen.getByText("clean content")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Clean" })).toHaveFocus();
    expect(window.history.length).toBe(historyLength);
    expect(events).toEqual(["cancel-and-join", "joined", "cancel-and-join", "joined"]);
  });

  it("restores the current canonical URL and focus when invalid-hash cancellation fails", async () => {
    let reject: (reason: Error) => void = () => undefined;
    const coordinator = {
      cancelAndJoin: vi.fn(() => new Promise<void>((_, rejectPromise) => { reject = rejectPromise; })),
    };
    render(<AppShell modes={registrations} locale="en" onLocaleChange={() => undefined} coordinator={coordinator} />);
    const clean = screen.getByRole("tab", { name: "Clean" });
    clean.focus();
    const historyLength = window.history.length;

    window.history.replaceState(null, "", "#/unavailable");
    fireEvent.popState(window);
    document.body.tabIndex = -1;
    document.body.focus();
    await act(async () => {
      reject(new Error("cancel failed"));
      await Promise.resolve();
    });

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not change destination. The current page remains active.");
    expect(screen.getByText("clean content")).toBeInTheDocument();
    expect(window.location.hash).toBe("#/clean");
    expect(window.history.state).toEqual({ route: "mode:clean" });
    expect(window.history.length).toBe(historyLength);
    expect(clean).toHaveFocus();
    document.body.removeAttribute("tabindex");
  });

  it("does not let a stale invalid-hash request replace a newer history intent", async () => {
    const settlements: Array<() => void> = [];
    const coordinator = {
      cancelAndJoin: vi.fn(() => new Promise<void>((resolve) => { settlements.push(resolve); })),
    };
    render(<AppShell modes={registrations} locale="en" onLocaleChange={() => undefined} coordinator={coordinator} />);

    window.history.replaceState(null, "", "#/unavailable");
    fireEvent.popState(window);
    window.history.replaceState(null, "", "#/status");
    fireEvent.popState(window);
    expect(settlements).toHaveLength(2);

    settlements[0]();
    await act(async () => { await Promise.resolve(); });
    expect(window.location.hash).toBe("#/status");
    expect(screen.getByText("clean content")).toBeInTheDocument();

    settlements[1]();
    expect(await screen.findByText("status content")).toBeInTheDocument();
    expect(window.location.hash).toBe("#/status");
    expect(screen.getByRole("tab", { name: "Status" })).toHaveFocus();
  });

  it("does not commit an invalid-hash fallback after unmount", async () => {
    let release: () => void = () => undefined;
    const coordinator = {
      cancelAndJoin: vi.fn(() => new Promise<void>((resolve) => { release = resolve; })),
    };
    const view = render(<AppShell modes={registrations} locale="en" onLocaleChange={() => undefined} coordinator={coordinator} />);

    window.history.replaceState(null, "", "#/unavailable");
    fireEvent.popState(window);
    view.unmount();
    release();
    await act(async () => { await Promise.resolve(); });

    expect(window.location.hash).toBe("#/unavailable");
  });

  it("keeps route, history, and focus stable when cancel-and-join rejects", async () => {
    let rejectFirst: (reason: Error) => void = () => undefined;
    const coordinator = {
      cancelAndJoin: vi.fn()
        .mockImplementationOnce(() => new Promise<void>((_, reject) => { rejectFirst = reject; }))
        .mockResolvedValue(undefined),
    };
    const user = userEvent.setup();
    render(<AppShell modes={registrations} locale="en" onLocaleChange={() => undefined} coordinator={coordinator} />);

    const language = screen.getByRole("button", { name: "Language" });
    await user.click(language);
    await waitFor(() => expect(language).toBeDisabled());

    // Chromium transfers focus to BODY when the active button becomes disabled.
    // Reproduce that real DOM lifecycle before the coordinator rejects.
    document.body.tabIndex = -1;
    document.body.focus();
    expect(document.body).toHaveFocus();
    await act(async () => {
      rejectFirst(new Error("cancel failed"));
      await Promise.resolve();
    });

    expect(await screen.findByRole("alert")).toHaveTextContent("Could not change destination. The current page remains active.");
    expect(screen.getByText("clean content")).toBeInTheDocument();
    expect(window.location.hash).toBe("#/clean");
    expect(language).toBeEnabled();
    expect(language).toHaveFocus();

    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    await user.click(language);
    expect(await screen.findByRole("combobox", { name: "Language" })).toHaveValue("en");
    expect(window.location.hash).toBe("#/settings");
    expect(language).toHaveFocus();
    expect(coordinator.cancelAndJoin).toHaveBeenCalledTimes(2);
    document.body.removeAttribute("tabindex");
  });

  it("keeps visually truncated user data fully accessible and copyable", () => {
    const value = `C:/${"very-long-directory/".repeat(30)}`;
    render(<AccessibleUserData value={value} />);
    const data = screen.getByLabelText(value);
    expect(data).toHaveTextContent(value);
    expect(data).toHaveAttribute("title", value);
    expect(data).toHaveClass("user-data-ellipsis");
  });
});
