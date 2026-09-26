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

async function openBrandMenu(user: ReturnType<typeof userEvent.setup>, name = "DevSweep menu") {
  await user.click(screen.getByRole("button", { name }));
  return screen.getByRole("menu");
}

describe("AppShell", () => {
  beforeEach(() => window.history.replaceState(null, "", "#/clean"));

  it("renders one capsule with the brand button and mode tabs and no sidebar or page header", () => {
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    const capsule = document.querySelector(".capsule");
    expect(capsule).toBeInTheDocument();
    const brand = screen.getByRole("button", { name: "DevSweep menu" });
    expect(capsule).toContainElement(brand);
    expect(brand).toHaveAttribute("aria-haspopup", "menu");
    expect(brand).toHaveAttribute("aria-expanded", "false");
    expect(brand).toHaveTextContent("DevSweep");
    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual(["Clean", "Software", "Optimize", "Analyze", "Status"]);
    expect(screen.getByRole("tablist")).not.toContainElement(brand);
    expect(document.querySelector(".shell-sidebar")).not.toBeInTheDocument();
    expect(document.querySelector(".page-header")).not.toBeInTheDocument();
    expect(document.querySelector(".page-header-slot")).not.toBeInTheDocument();
    expect(document.querySelector(".sweep-body")).not.toBeInTheDocument();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("keeps the frozen five-mode identity but omits unavailable registrations", () => {
    render(<AppShell modes={[registrations[0]]} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    expect(MODE_IDS).toEqual(["clean", "software", "optimize", "analyze", "status"]);
    expect(screen.getAllByRole("tab")).toHaveLength(1);
    expect(screen.getByRole("tab", { name: "Clean" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Software" })).not.toBeInTheDocument();
    expect(SUPPORTING_DESTINATION_IDS).toEqual(["protection", "rules", "history"]);
    expect(screen.queryByText("More")).not.toBeInTheDocument();
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-mode", "clean");
    expect(document.querySelector(".capsule")).toBeInTheDocument();
  });

  it("omits unavailable supporting destinations from the brand menu", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={[registrations[0]]} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    await openBrandMenu(user);
    expect(screen.queryByRole("menuitem", { name: "Protection" })).not.toBeInTheDocument();
    expect(screen.getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Settings", "Help"]);
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

  it("shows every mode in the capsule and every supporting entry in the brand menu without a More disclosure", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    for (const name of ["Clean", "Software", "Optimize", "Analyze", "Status"]) {
      expect(screen.getByRole("tab", { name })).toBeInTheDocument();
    }
    expect(screen.getByRole("tablist", { name: "Modes" })).toHaveAttribute("aria-orientation", "horizontal");
    expect(screen.getByRole("heading", { level: 1, name: "Clean" })).toHaveClass("sr-only");
    const menu = await openBrandMenu(user);
    expect(menu).toHaveAccessibleName("Supporting destinations");
    expect(screen.getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Protection", "Rules", "History", "Settings", "Help"]);
    expect(screen.getByRole("menuitem", { name: "Help" })).toHaveAttribute("href", "https://github.com/bahayonghang/devsweep#readme");
    expect(screen.queryByText("More")).not.toBeInTheDocument();
    expect(screen.queryByText("更多")).not.toBeInTheDocument();
  });

  it("opens and closes the brand menu by mouse and keyboard and restores focus", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    const brand = screen.getByRole("button", { name: "DevSweep menu" });
    await user.click(brand);
    expect(brand).toHaveAttribute("aria-expanded", "true");
    expect(brand).toHaveAttribute("aria-controls", "brand-menu");
    expect(screen.getByRole("menuitem", { name: "Protection" })).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: "Rules" })).toHaveFocus();
    await user.keyboard("{End}");
    expect(screen.getByRole("menuitem", { name: "Help" })).toHaveFocus();
    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: "Protection" })).toHaveFocus();
    await user.keyboard("{ArrowUp}");
    expect(screen.getByRole("menuitem", { name: "Help" })).toHaveFocus();
    await user.keyboard("{Home}");
    expect(screen.getByRole("menuitem", { name: "Protection" })).toHaveFocus();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(brand).toHaveAttribute("aria-expanded", "false");
    expect(brand).toHaveFocus();

    await user.keyboard("{ArrowDown}");
    await user.keyboard("{Tab}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(brand).toHaveFocus();

    await user.keyboard("{ArrowDown}");
    expect(screen.getByRole("menu")).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Protection" })).toHaveFocus();
    await user.click(document.body);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    await user.click(brand);
    await user.click(brand);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("renders a supporting page in the stage with a back control to the last mode", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    await user.click(screen.getByRole("tab", { name: "Analyze" }));
    expect(await screen.findByText("analyze content")).toBeInTheDocument();
    await openBrandMenu(user);
    await user.click(screen.getByRole("menuitem", { name: "Rules" }));
    expect(await screen.findByText("rules support content")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 1, name: "Rules" })).not.toHaveClass("sr-only");
    await user.click(screen.getByRole("button", { name: "Back to Analyze" }));
    expect(await screen.findByText("analyze content")).toBeInTheDocument();
    expect(window.location.hash).toBe("#/analyze");
    expect(screen.getByRole("tab", { name: "Analyze" })).toHaveFocus();
  });

  it("restores the actual primary and supporting activators after composition", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);

    const status = screen.getByRole("tab", { name: "Status" });
    await user.click(status);
    expect(await screen.findByText("status content")).toBeInTheDocument();
    expect(status).toHaveFocus();

    await openBrandMenu(user);
    await user.click(screen.getByRole("menuitem", { name: "History" }));
    expect(await screen.findByText("history support content")).toBeInTheDocument();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "DevSweep menu" })).toHaveFocus();
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-mode", "shell");
  });

  it("restores a supporting deep link onto the brand button", async () => {
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    window.history.pushState(null, "", "#/history");
    fireEvent.popState(window);
    expect(await screen.findByText("history support content")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "DevSweep menu" })).toHaveFocus();
    expect(document.querySelector(".app-shell")).toHaveAttribute("data-mode", "shell");
    expect(screen.getByRole("heading", { level: 1, name: "History" })).toBeVisible();
  });

  it("moves capsule tab focus with all four arrow keys", async () => {
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

  it("names every mode route and shows a visible title and subtitle on every supporting route", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="en" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    for (const name of ["Clean", "Software", "Optimize", "Analyze", "Status"]) {
      await user.click(screen.getByRole("tab", { name }));
      expect(await screen.findByRole("heading", { level: 1, name })).toHaveClass("sr-only");
    }
    const routes = [
      { name: "Protection", title: "Protection", subtitle: "Inspect protection policy without changing cleanup authority." },
      { name: "Rules", title: "Rules", subtitle: "Inspect the rule catalogue." },
      { name: "History", title: "History", subtitle: "Inspect past cleanup records." },
      { name: "Settings", title: "Settings", subtitle: "Appearance, language, and sampling preferences." },
    ];
    for (const route of routes) {
      await openBrandMenu(user);
      await user.click(screen.getByRole("menuitem", { name: route.name }));
      expect(await screen.findByRole("heading", { level: 1, name: route.title })).toBeVisible();
      expect(screen.getByText(route.subtitle)).toBeVisible();
      expect(screen.getByRole("button", { name: "Back to Status" })).toBeInTheDocument();
    }
  });

  it("names every zh-CN mode route and shows a visible title and subtitle on every zh-CN supporting route", async () => {
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="zh-CN" onLocaleChange={() => undefined} coordinator={new OperationCoordinator()} />);
    for (const name of ["清理", "软件", "优化", "分析", "状态"]) {
      await user.click(screen.getByRole("tab", { name }));
      expect(await screen.findByRole("heading", { level: 1, name })).toHaveClass("sr-only");
    }
    const routes = [
      { name: "保护", title: "保护", subtitle: "查看保护策略，不改变清理权限。" },
      { name: "规则", title: "规则", subtitle: "查看规则目录。" },
      { name: "历史", title: "历史", subtitle: "查看既往清理记录。" },
      { name: "设置", title: "设置", subtitle: "外观、语言和采样配置。" },
    ];
    for (const route of routes) {
      await openBrandMenu(user, "DevSweep 菜单");
      await user.click(screen.getByRole("menuitem", { name: route.name }));
      expect(await screen.findByRole("heading", { level: 1, name: route.title })).toBeVisible();
      expect(screen.getByText(route.subtitle)).toBeVisible();
      expect(screen.getByRole("button", { name: "返回状态" })).toBeInTheDocument();
    }
  });

  it("renders Chinese canonical labels and writes language selection through the adapter", async () => {
    const save = vi.fn();
    const user = userEvent.setup();
    render(<AppShell modes={registrations} supporting={supporting} locale="zh-CN" onLocaleChange={save} coordinator={new OperationCoordinator()} />);
    expect(screen.getByRole("tab", { name: "清理" })).not.toHaveAttribute("title");
    expect(screen.getByRole("tab", { name: "软件" })).toHaveAttribute("title", "Alt+R");
    expect(screen.getByRole("tablist", { name: "模式" })).toBeInTheDocument();
    expect(screen.queryByText("更多")).not.toBeInTheDocument();
    expect(screen.queryByText("More")).not.toBeInTheDocument();
    for (const name of ["清理", "软件", "优化", "分析", "状态"]) {
      expect(screen.getByRole("tab", { name })).toBeInTheDocument();
    }
    expect(screen.getByRole("heading", { level: 1, name: "清理" })).toBeInTheDocument();
    expect(await openBrandMenu(user, "DevSweep 菜单")).toHaveAccessibleName("支持目的地");
    expect(screen.getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["保护", "规则", "历史", "设置", "帮助"]);
    await user.click(screen.getByRole("menuitem", { name: "设置" }));
    expect(await screen.findByRole("heading", { level: 1, name: "设置" })).toBeVisible();
    expect(screen.getByText("外观、语言和采样配置。")).toBeVisible();
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

    await openBrandMenu(user);
    await user.click(screen.getByRole("menuitem", { name: "History" }));
    expect(await screen.findByText("history support content")).toBeInTheDocument();
    expect(window.location.hash).toBe("#/history");
    const settingsOpener = screen.getByRole("button", { name: "DevSweep menu" });
    await openBrandMenu(user);
    await user.click(screen.getByRole("menuitem", { name: "Settings" }));
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

    const language = screen.getByRole("button", { name: "DevSweep menu" });
    await openBrandMenu(user);
    await user.click(screen.getByRole("menuitem", { name: "Settings" }));
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
    await openBrandMenu(user);
    await user.click(screen.getByRole("menuitem", { name: "Settings" }));
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
