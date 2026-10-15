import { createFixturePreferencesBridge } from "./preferences/fixture";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode, useState, type ComponentProps } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import scanReplays from "./api/fixtures/clean/scan-advisory-replays.json";
import dryRunJson from "./api/fixtures/dry-run-outcome.json";
import npmDryRunJson from "./api/fixtures/dry-run-outcome-npm.json";
import progressJson from "./api/fixtures/scan-progress.json";
import { decodeDesktopScanProgress, decodeDryRunOutcome, decodeScanReport } from "./api/contract";
import type { DesktopBridge } from "./api/bridge";
import { fixtureBridge } from "./api/fixture-bridge";
import type { DesktopScanProgress, DesktopScanResult, ExecutionReport } from "./api/types.gen";
import { App as DesktopApp } from "./App";
import { formatBytes } from "./components/format";
import type { PresentationSettings, PresentationSettingsBridge } from "./i18n";
import { createFixtureWindowBridge, type DesktopLifecycleBridge } from "./lifecycle";
import { OperationCoordinator } from "./state/operation-coordinator";

const scanReport = decodeScanReport(scanReplays.projects.report);
const dryRun = decodeDryRunOutcome(dryRunJson);
const npmDryRun = decodeDryRunOutcome(npmDryRunJson);
const execution: ExecutionReport = { ...dryRun.report, dry_run: false, audit_log: "C:/fixture-audit.jsonl" };
const progress = decodeDesktopScanProgress(progressJson);

function App(props: ComponentProps<typeof DesktopApp>) {
  const [windowControls] = useState(createFixtureWindowBridge);
  const [desktopPreferences] = useState(createFixturePreferencesBridge);
  return <DesktopApp desktopPreferences={desktopPreferences} windowControls={windowControls} {...props} />;
}

function fakeBridge(overrides: Partial<DesktopBridge> = {}): DesktopBridge {
  return {
    ...fixtureBridge,
    analyzeStart: vi.fn().mockRejectedValue({ code: "analyze_failed", message: "Analyze is not exercised by this Clean fixture" }),
    analyzeCancel: vi.fn().mockResolvedValue(undefined),
    scanStart: vi.fn().mockImplementation(async (scanId: string, _options, onProgress: (progress: DesktopScanProgress) => void) => {
      onProgress({ ...progress, scan_id: scanId });
      return { type: "completed", scan_id: scanId, report: scanReport };
    }),
    scanCancel: vi.fn().mockResolvedValue(undefined),
    planDryRun: vi.fn().mockResolvedValue(dryRun),
    planExecute: vi.fn().mockResolvedValue(execution),
    ...overrides,
  };
}

function fakePresentationSettings(language: "en" | "zh-CN" | null = "en") {
  const bridge: PresentationSettingsBridge = {
    load: vi.fn().mockResolvedValue({ language }),
    save: vi.fn().mockImplementation(async (next) => ({ language: next })),
  };
  return bridge;
}

function fakeLifecycle(overrides: Partial<DesktopLifecycleBridge> = {}): DesktopLifecycleBridge {
  return {
    onCloseRequested: vi.fn().mockResolvedValue(() => undefined),
    nativeFaultMode: vi.fn().mockResolvedValue("disabled"),
    ...overrides,
  };
}

async function openLanguageSettings(
  user: ReturnType<typeof userEvent.setup>,
  language = "Settings",
) {
  await user.click(await screen.findByRole("tab", { name: language }));
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

async function armActiveOperation(
  coordinator: OperationCoordinator,
  events: string[],
  label: string,
  cancelFailure = false,
) {
  const joined = deferred<void>();
  const joinedWithTrace = joined.promise.then(() => { events.push(`${label}.join`); });
  await coordinator.start({
    kind: "status",
    id: `${label}-active`,
    start: () => joinedWithTrace,
    cancel: () => {
      events.push(`${label}.cancel`);
      joined.resolve();
      if (cancelFailure) throw new Error(`${label} cancellation failed`);
    },
  });
}

describe("desktop workflow", () => {
  beforeEach(() => window.history.replaceState(null, "", "#/clean"));

  it("uses canonical IEC byte units at exact boundaries", () => {
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1024)).toBe("1.0 KiB");
    expect(formatBytes(1_048_576)).toBe("1.0 MiB");
    expect(formatBytes(1_073_741_824)).toBe("1.0 GiB");
    expect(formatBytes(1_099_511_627_776)).toBe("1.0 TiB");
  });

  it("zh-CN Clean workbench does not show English Projects Scan Search Ready", async () => {
    render(<App bridge={fakeBridge()} presentationSettings={fakePresentationSettings("zh-CN")} userLocales={["zh-CN"]} />);
    expect(await screen.findByRole("button", { name: "扫描项目产物" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "项目产物" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "全局缓存" })).toBeInTheDocument();
    expect(screen.getAllByText("尚未扫描")).toHaveLength(2);
    expect(screen.queryByText("Projects")).not.toBeInTheDocument();
    expect(screen.queryByText("Scan")).not.toBeInTheDocument();
    expect(screen.queryByText("Search")).not.toBeInTheDocument();
    expect(screen.queryByText("Ready")).not.toBeInTheDocument();
    expect(screen.queryByText("没有扫描结果")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "扫描全局缓存" })).toBeEnabled();
    expect(screen.queryByText("Found so far")).not.toBeInTheDocument();
    expect(screen.queryByText("目前已发现")).not.toBeInTheDocument();
    expect(document.querySelector(".sweep-body-hero")).not.toBeInTheDocument();
    expect(document.querySelector(".clean-workbench")).toBeInTheDocument();
    expect(document.querySelector(".clean-workbench .planet")).not.toBeInTheDocument();
  });

  it("starts a Clean scan after StrictMode replays the lifecycle effect", async () => {
    const scanStart = vi.fn().mockImplementation(async (scanId: string, _options, onProgress: (value: DesktopScanProgress) => void) => {
      onProgress({ ...progress, scan_id: scanId });
      return { type: "completed", scan_id: scanId, report: scanReport };
    });
    const user = userEvent.setup();
    render(<StrictMode>
      <App bridge={fakeBridge({ scanStart })} presentationSettings={fakePresentationSettings("zh-CN")} userLocales={["zh-CN"]} />
    </StrictMode>);

    expect(await screen.findByRole("button", { name: "扫描项目产物" })).toBeInTheDocument();
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    await user.click(screen.getByRole("button", { name: "扫描项目产物" }));
    expect(scanStart).toHaveBeenCalledOnce();
    expect(scanStart.mock.calls[0][1]).toEqual({ include_projects: true, include_global: false, roots: ["C:/work/app"] });
  });

  it("starts Status live after StrictMode replays the lifecycle effect", async () => {
    const statusLiveStart = vi.fn().mockResolvedValue({ type: "canceled", operation_id: "strict-status-live" });
    const user = userEvent.setup();
    render(<StrictMode>
      <App bridge={fakeBridge({ statusLiveStart })} presentationSettings={fakePresentationSettings()} />
    </StrictMode>);

    expect(await screen.findByRole("button", { name: "Scan project artifacts" })).toBeInTheDocument();
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    await user.click(screen.getByRole("tab", { name: "Status" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start live" })).toBeEnabled());
    await user.click(screen.getByRole("button", { name: "Start live" }));
    expect(statusLiveStart).toHaveBeenCalledOnce();
  });

  it("idle Clean home exposes independent global and project actions without a decorative planet", async () => {
    render(<App bridge={fakeBridge()} presentationSettings={fakePresentationSettings()} />);
    expect(await screen.findByRole("button", { name: "Scan project artifacts" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Project artifacts" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Global caches" })).toBeInTheDocument();
    expect(screen.getAllByText("Not scanned")).toHaveLength(2);
    expect(screen.getByRole("button", { name: "Scan global caches" })).toBeEnabled();
    expect(screen.queryByText("No scan results")).not.toBeInTheDocument();
    expect(screen.queryByText("Found so far")).not.toBeInTheDocument();
    expect(document.querySelector(".sweep-body-hero")).not.toBeInTheDocument();
    expect(document.querySelector(".clean-workbench")).toBeInTheDocument();
    expect(document.querySelector(".clean-workbench .planet")).not.toBeInTheDocument();
    expect(document.querySelectorAll(".clean-scope-panel")).toHaveLength(2);
  });

  it("keeps global scanning independent of unavailable project-root settings", async () => {
    const report = decodeScanReport(scanReplays.global.report);
    const scanStart = vi.fn(async (scanId: string) => ({ type: "completed" as const, scan_id: scanId, report }));
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({
      cleanScanPreferencesGet: vi.fn().mockRejectedValue(new Error("project roots unavailable")),
      scanStart,
    })} presentationSettings={fakePresentationSettings()} />);
    expect(await screen.findByRole("button", { name: "Scan global caches" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Scan project artifacts" })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Scan global caches" }));
    expect(scanStart).toHaveBeenCalledWith(expect.any(String), { include_projects: false, include_global: true, roots: [] }, expect.any(Function), expect.any(Function));
    expect(await screen.findByRole("button", { name: "Review global targets" })).toBeEnabled();
    expect(screen.queryByRole("button", { name: "Review targets" })).not.toBeInTheDocument();
  });

  it("opens Settings without starting Status sampling", async () => {
    window.history.replaceState(null, "", "#/settings");
    const statusSnapshot = vi.fn(fixtureBridge.statusSnapshot);
    const statusLiveStart = vi.fn(fixtureBridge.statusLiveStart);
    render(<App bridge={fakeBridge({ statusSnapshot, statusLiveStart })} presentationSettings={fakePresentationSettings()} />);
    expect(await screen.findByRole("tabpanel", { name: "Settings" })).toBeInTheDocument();
    expect(statusSnapshot).not.toHaveBeenCalled();
    expect(statusLiveStart).not.toHaveBeenCalled();
  });

  it("keeps the shell absent while presentation settings are loading", () => {
    const load = deferred<PresentationSettings>();
    const presentationSettings: PresentationSettingsBridge = {
      load: vi.fn(() => load.promise),
      save: vi.fn(),
    };

    render(<App bridge={fakeBridge()} presentationSettings={presentationSettings} userLocales={["zh-CN"]} />);

    expect(screen.getByRole("status")).toHaveTextContent("Loading presentation settings");
    expect(document.querySelector(".app-shell")).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Clean" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "清理" })).not.toBeInTheDocument();
  });

  it("fails load closed without an OS or English locale fallback", async () => {
    const presentationSettings: PresentationSettingsBridge = {
      load: vi.fn().mockRejectedValue(new Error("unknown presentation schema")),
      save: vi.fn(),
    };

    render(<App bridge={fakeBridge()} presentationSettings={presentationSettings} userLocales={["zh-CN"]} />);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("Presentation settings unavailable");
    expect(alert).toHaveTextContent("显示设置不可用");
    expect(alert).toHaveTextContent("Existing bytes were preserved");
    expect(document.querySelector(".app-shell")).not.toBeInTheDocument();
    expect(document.querySelector("[data-locale]")).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Clean" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "清理" })).not.toBeInTheDocument();
    expect(presentationSettings.save).not.toHaveBeenCalled();
  });

  it("gates a ready shell immediately when the settings bridge identity changes", async () => {
    const userLocales = ["en-US"] as const;
    const nextLoad = deferred<PresentationSettings>();
    const nextSettings: PresentationSettingsBridge = {
      load: vi.fn(() => nextLoad.promise),
      save: vi.fn(),
    };
    const view = render(<App
      bridge={fakeBridge()}
      presentationSettings={fakePresentationSettings("en")}
      userLocales={userLocales}
    />);
    expect(await screen.findByRole("tab", { name: "Clean" })).toBeInTheDocument();

    view.rerender(<App bridge={fakeBridge()} presentationSettings={nextSettings} userLocales={userLocales} />);

    expect(screen.getByRole("status")).toHaveTextContent("Loading presentation settings");
    expect(document.querySelector(".app-shell")).not.toBeInTheDocument();
    expect(document.querySelector("[data-locale]")).not.toBeInTheDocument();

    await act(async () => {
      nextLoad.reject(new Error("new store is unreadable"));
      await nextLoad.promise.catch(() => undefined);
    });
    expect(await screen.findByRole("alert")).toHaveTextContent("Presentation settings unavailable");
    expect(document.querySelector(".app-shell")).not.toBeInTheDocument();
    expect(document.querySelector("[data-locale]")).not.toBeInTheDocument();
  });

  it("rejects an old load settlement after the settings bridge changes", async () => {
    const userLocales = ["en-US"] as const;
    const oldLoad = deferred<PresentationSettings>();
    const newLoad = deferred<PresentationSettings>();
    const oldSettings: PresentationSettingsBridge = {
      load: vi.fn(() => oldLoad.promise),
      save: vi.fn(),
    };
    const newSettings: PresentationSettingsBridge = {
      load: vi.fn(() => newLoad.promise),
      save: vi.fn(),
    };
    const view = render(<App bridge={fakeBridge()} presentationSettings={oldSettings} userLocales={userLocales} />);
    view.rerender(<App bridge={fakeBridge()} presentationSettings={newSettings} userLocales={userLocales} />);

    await act(async () => {
      oldLoad.resolve({ language: "zh-CN" });
      await oldLoad.promise;
    });
    expect(screen.getByRole("status")).toHaveTextContent("Loading presentation settings");
    expect(document.querySelector(".app-shell")).not.toBeInTheDocument();
    expect(document.querySelector("[data-locale]")).not.toBeInTheDocument();

    act(() => newLoad.resolve({ language: "en" }));
    expect(await screen.findByRole("tab", { name: "Clean" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "清理" })).not.toBeInTheDocument();
  });

  it("rejects an old save settlement while a replacement store is loading", async () => {
    const userLocales = ["en-US"] as const;
    const oldSave = deferred<PresentationSettings>();
    const oldSettings: PresentationSettingsBridge = {
      load: vi.fn().mockResolvedValue({ language: "en" }),
      save: vi.fn(() => oldSave.promise),
    };
    const newLoad = deferred<PresentationSettings>();
    const newSettings: PresentationSettingsBridge = {
      load: vi.fn(() => newLoad.promise),
      save: vi.fn(),
    };
    const user = userEvent.setup();
    const view = render(<App bridge={fakeBridge()} presentationSettings={oldSettings} userLocales={userLocales} />);
    await openLanguageSettings(user);
    await user.click(screen.getByRole("combobox", { name: "Language" }));
    await user.click(await screen.findByRole("option", { name: "Simplified Chinese" }));
    expect(oldSettings.save).toHaveBeenCalledWith("zh-CN");

    view.rerender(<App bridge={fakeBridge()} presentationSettings={newSettings} userLocales={userLocales} />);
    expect(screen.getByRole("status")).toHaveTextContent("Loading presentation settings");
    expect(document.querySelector(".app-shell")).not.toBeInTheDocument();

    await act(async () => {
      oldSave.resolve({ language: "zh-CN" });
      await oldSave.promise;
    });
    expect(screen.getByRole("status")).toHaveTextContent("Loading presentation settings");
    expect(document.querySelector("[data-locale]")).not.toBeInTheDocument();

    act(() => newLoad.resolve({ language: "en" }));
    expect(await screen.findByRole("tab", { name: "Clean" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "清理" })).not.toBeInTheDocument();
  });

  it("gates a user-locale identity change until its new load succeeds", async () => {
    const nextLoad = deferred<PresentationSettings>();
    const presentationSettings: PresentationSettingsBridge = {
      load: vi.fn()
        .mockResolvedValueOnce({ language: null })
        .mockImplementationOnce(() => nextLoad.promise),
      save: vi.fn(),
    };
    const view = render(<App
      bridge={fakeBridge()}
      presentationSettings={presentationSettings}
      userLocales={["en-US"]}
    />);
    expect(await screen.findByRole("tab", { name: "Clean" })).toBeInTheDocument();

    view.rerender(<App
      bridge={fakeBridge()}
      presentationSettings={presentationSettings}
      userLocales={["zh-CN"]}
    />);
    expect(screen.getByRole("status")).toHaveTextContent("Loading presentation settings");
    expect(document.querySelector(".app-shell")).not.toBeInTheDocument();
    expect(document.querySelector("[data-locale]")).not.toBeInTheDocument();

    act(() => nextLoad.resolve({ language: null }));
    expect(await screen.findByRole("tab", { name: "清理" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Clean" })).not.toBeInTheDocument();
  });

  it("loads and persists the shared language setting without registering unavailable modes", async () => {
    const presentationSettings = fakePresentationSettings("zh-CN");
    const user = userEvent.setup();
    render(<App bridge={fakeBridge()} presentationSettings={presentationSettings} userLocales={["en-US"]} />);

    expect(await screen.findByRole("tab", { name: "清理" })).toBeInTheDocument();
    const softwareTab = screen.getByRole("tab", { name: "软件" });
    expect(softwareTab).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "优化" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "状态" })).toBeInTheDocument();
    await user.click(softwareTab);
    expect(screen.getByRole("region", { name: "软件" })).toBeInTheDocument();
    expect(document.querySelector(".shell-brand-icon")).toHaveAttribute("src", "/src/assets/devsweep-icon-master.png");
    await openLanguageSettings(user, "设置");
    await user.click(screen.getByRole("combobox", { name: "语言" }));
    await user.click(screen.getByRole("option", { name: "英语" }));
    await waitFor(() => expect(presentationSettings.save).toHaveBeenCalledWith("en"));
    expect(screen.getByRole("tab", { name: "Clean" })).toHaveAttribute("title", "Alt+C");
  });

  it("keeps the prior locale visible until a matching save response succeeds", async () => {
    const save = deferred<PresentationSettings>();
    const presentationSettings: PresentationSettingsBridge = {
      load: vi.fn().mockResolvedValue({ language: "en" }),
      save: vi.fn(() => save.promise),
    };
    const user = userEvent.setup();
    render(<App bridge={fakeBridge()} presentationSettings={presentationSettings} />);

    await openLanguageSettings(user);
    const select = screen.getByRole("combobox", { name: "Language" });
    await user.click(select);
    await user.click(await screen.findByRole("option", { name: "Simplified Chinese" }));
    expect(presentationSettings.save).toHaveBeenCalledWith("zh-CN");
    expect(select).toBeDisabled();
    expect(select).toHaveTextContent("English");
    expect(screen.getByRole("tab", { name: "Clean" })).toBeInTheDocument();
    expect(screen.getByText("Saving language preference…")).toHaveAttribute("role", "status");

    act(() => save.resolve({ language: "zh-CN" }));
    expect(await screen.findByRole("tab", { name: "清理" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "语言" })).toHaveTextContent("简体中文");
  });

  it("preserves the prior locale and selection when save rejects", async () => {
    const presentationSettings: PresentationSettingsBridge = {
      load: vi.fn().mockResolvedValue({ language: "en" }),
      save: vi.fn().mockRejectedValue(new Error("store unavailable")),
    };
    const user = userEvent.setup();
    render(<App bridge={fakeBridge()} presentationSettings={presentationSettings} />);

    await openLanguageSettings(user);
    await user.click(screen.getByRole("combobox", { name: "Language" }));
    await user.click(await screen.findByRole("option", { name: "Simplified Chinese" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Language preference was not changed");
    expect(screen.getByRole("combobox", { name: "Language" })).toHaveTextContent("English");
    expect(screen.getByRole("tab", { name: "Clean" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "清理" })).not.toBeInTheDocument();
  });

  it("preserves the prior locale when the save response tag mismatches", async () => {
    const presentationSettings: PresentationSettingsBridge = {
      load: vi.fn().mockResolvedValue({ language: "en" }),
      save: vi.fn().mockResolvedValue({ language: "en" }),
    };
    const user = userEvent.setup();
    render(<App bridge={fakeBridge()} presentationSettings={presentationSettings} />);

    await openLanguageSettings(user);
    await user.click(screen.getByRole("combobox", { name: "Language" }));
    await user.click(await screen.findByRole("option", { name: "Simplified Chinese" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Language preference was not changed");
    expect(screen.getByRole("combobox", { name: "Language" })).toHaveTextContent("English");
    expect(screen.getByRole("tab", { name: "Clean" })).toBeInTheDocument();
  });

  it("ignores pending load and save settlement after unmount", async () => {
    const pendingLoad = deferred<PresentationSettings>();
    const loadSettings: PresentationSettingsBridge = {
      load: vi.fn(() => pendingLoad.promise),
      save: vi.fn(),
    };
    const loadingView = render(<App bridge={fakeBridge()} presentationSettings={loadSettings} />);
    expect(screen.getByRole("status")).toBeInTheDocument();
    loadingView.unmount();
    await act(async () => {
      pendingLoad.reject(new Error("late load failure"));
      await pendingLoad.promise.catch(() => undefined);
    });

    const pendingSave = deferred<PresentationSettings>();
    const saveSettings: PresentationSettingsBridge = {
      load: vi.fn().mockResolvedValue({ language: "en" }),
      save: vi.fn(() => pendingSave.promise),
    };
    const user = userEvent.setup();
    const readyView = render(<App bridge={fakeBridge()} presentationSettings={saveSettings} />);
    await openLanguageSettings(user);
    await user.click(screen.getByRole("combobox", { name: "Language" }));
    await user.click(await screen.findByRole("option", { name: "Simplified Chinese" }));
    readyView.unmount();
    await act(async () => {
      pendingSave.reject(new Error("late save failure"));
      await pendingSave.promise.catch(() => undefined);
    });
    expect(document.querySelector(".app-shell")).not.toBeInTheDocument();
  });

  it("keeps scan IPC arguments locale-neutral across presentation changes", async () => {
    const presentationSettings = fakePresentationSettings("en");
    const scanStart = vi.fn().mockImplementation(async (scanId: string) => ({ type: "completed", scan_id: scanId, report: scanReport }));
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart })} presentationSettings={presentationSettings} />);

    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    await screen.findByText("Scan complete. 1 targets.");
    await openLanguageSettings(user);
    await user.click(screen.getByRole("combobox", { name: "Language" }));
    await user.click(await screen.findByRole("option", { name: "Simplified Chinese" }));
    await user.click(screen.getByRole("tab", { name: "清理" }));
    expect(await screen.findByText("扫描完成。1 个目标。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "审核项目目标" })).toBeEnabled();
    expect(scanStart).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: "扫描项目产物" }));
    await waitFor(() => expect(scanStart).toHaveBeenCalledTimes(2));
    expect(scanStart.mock.calls[0][1]).toEqual(scanStart.mock.calls[1][1]);
    expect(scanStart.mock.calls[0][1]).toEqual({ include_projects: true, include_global: false, roots: ["C:/work/app"] });
    expect(scanReport).toEqual(decodeScanReport(scanReplays.projects.report));
  });

  it("keeps a canceled preview read-only and replaces it on rescan", async () => {
    let finishFirst: (result: DesktopScanResult) => void = () => undefined;
    let invocation = 0;
    const scanStart = vi.fn().mockImplementation((scanId: string, _options, onProgress: (progress: DesktopScanProgress) => void) => {
      invocation += 1;
      onProgress({ ...progress, scan_id: scanId });
      if (invocation === 1) return new Promise<DesktopScanResult>((resolve) => { finishFirst = resolve; });
      return Promise.resolve<DesktopScanResult>({ type: "completed", scan_id: scanId, report: scanReport });
    });
    const scanCancel = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart, scanCancel })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    await user.click(screen.getByRole("button", { name: "Show details" }));
    expect(screen.getByText("Scan completed with partial evidence. Totals are not exact.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Found so far" })).toBeInTheDocument();
    expect(document.querySelector(".found-so-far")).toHaveAttribute("aria-busy", "true");
    expect(screen.queryByRole("checkbox", { name: /Select node.node_modules/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Review dry run" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(scanCancel).toHaveBeenCalledWith(expect.any(String));
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    const firstScanId = scanStart.mock.calls[0][0] as string;
    act(() => finishFirst({ type: "canceled", scan_id: firstScanId }));
    expect(await screen.findByText("Scan canceled. Results stay non-selectable.")).toBeInTheDocument();
    expect(screen.getByRole("tabpanel", { name: "Clean" }).querySelector(".planet")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Projects 1" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Back to cleanup" }));
    await user.click(screen.getByRole("button", { name: "Scan project artifacts" }));
    expect(await screen.findByText("Scan complete. 1 targets.")).toBeInTheDocument();
    expect(scanStart).toHaveBeenCalledTimes(2);
  });

  it("distinguishes an active empty scan from a completed empty report", async () => {
    let finish: (result: DesktopScanResult) => void = () => undefined;
    const scanStart = vi.fn().mockImplementation(() => new Promise<DesktopScanResult>((resolve) => { finish = resolve; }));
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    expect(screen.getByText("Scanning. Targets appear as they are found.")).toHaveAttribute("role", "status");
    await user.click(screen.getByRole("button", { name: "Show details" }));
    expect(screen.getByRole("heading", { name: "Found so far" })).toBeInTheDocument();
    expect(screen.queryByText("No scan results")).not.toBeInTheDocument();
    const scanId = scanStart.mock.calls[0][0] as string;
    act(() => finish({ type: "completed", scan_id: scanId, report: { ...scanReport, plan: { ...scanReport.plan, targets: [] }, health: { completeness: "complete", diagnostics: [], totals: { verified_bytes: 0, partial_lower_bound_bytes: 0, unknown_target_count: 0 } } } }));
    await user.click(screen.getByRole("button", { name: "Back to cleanup" }));
    expect(await screen.findByText("No cleanup targets found. See coverage for missing or unsupported locations.")).toBeInTheDocument();
  });

  it("offers an explicit return to the previous completed report after a stopped rescan", async () => {
    let finishRescan: (result: DesktopScanResult) => void = () => undefined;
    let invocation = 0;
    const scanStart = vi.fn().mockImplementation((scanId: string, _options, onProgress: (value: DesktopScanProgress) => void) => {
      invocation += 1;
      if (invocation === 1) return Promise.resolve<DesktopScanResult>({ type: "completed", scan_id: scanId, report: scanReport });
      onProgress({ ...progress, scan_id: scanId });
      return new Promise<DesktopScanResult>((resolve) => { finishRescan = resolve; });
    });
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    await screen.findByText("Scan complete. 1 targets.");
    await user.click(screen.getByRole("button", { name: "Scan project artifacts" }));
    const rescanId = scanStart.mock.calls[1][0] as string;
    act(() => finishRescan({ type: "canceled", scan_id: rescanId }));

    await user.click(await screen.findByRole("button", { name: "Show details" }));
    const returnButton = await screen.findByRole("button", { name: "Cancel" });
    expect(screen.queryByRole("button", { name: "Review dry run" })).not.toBeInTheDocument();
    await user.click(returnButton);
    expect(await screen.findByText("Scan complete. 1 targets.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Review dry run" })).toBeDisabled();
    await user.click(screen.getByRole("checkbox", { name: "Select C:/work/app/target" }));
    expect(screen.getByRole("button", { name: "Review dry run" })).toBeEnabled();
  });

  it("requests cancellation on a malformed progress payload and waits for the terminal result", async () => {
    let finish: (result: DesktopScanResult) => void = () => undefined;
    const scanCancel = vi.fn().mockResolvedValue(undefined);
    const scanStart = vi.fn().mockImplementation((scanId: string, _options, onProgress: (value: DesktopScanProgress) => void, onProgressError: (error: unknown) => void) => {
      onProgress({ ...progress, scan_id: scanId });
      onProgressError(new Error("Malformed scan progress"));
      return new Promise<DesktopScanResult>((resolve) => { finish = resolve; });
    });
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart, scanCancel })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    await waitFor(() => expect(scanCancel).toHaveBeenCalledOnce());
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent("Malformed scan progress");
    const scanId = scanStart.mock.calls[0][0] as string;
    act(() => finish({ type: "canceled", scan_id: scanId }));
    expect(await screen.findByText("Scan stopped with an error. Found targets stay non-selectable.")).toBeInTheDocument();
  });

  it("keeps cumulative global preview rows read-only and rejects stale project updates", async () => {
    let finish: (result: DesktopScanResult) => void = () => undefined;
    const scanStart = vi.fn().mockImplementation((scanId: string, _options, onProgress: (value: DesktopScanProgress) => void) => {
      const projectTarget = progress.preview!.targets[0];
      const globalTarget = {
        ...projectTarget,
        id: "fixture-global",
        scope: { type: "global" as const },
        kind: "package_cache" as const,
        path: "C:/fixture/global-cache",
        estimated_bytes: 8192,
      };
      onProgress({
        scan_id: scanId,
        sequence: 2,
        phase: "global",
        message: "Scanning controlled global providers",
        preview: {
          targets: [globalTarget],
          totals: { target_count: 1, verified_bytes: 8192, partial_lower_bound_bytes: 0, unknown_target_count: 0 },
        },
      });
      onProgress({ ...progress, scan_id: scanId, sequence: 1, message: "Stale project message" });
      return new Promise<DesktopScanResult>((resolve) => { finish = resolve; });
    });
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan global caches" }));
    await user.click(screen.getByRole("button", { name: "Show details" }));
    expect(screen.getByRole("heading", { name: "Found so far" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Projects 1" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Global caches 1" })).toBeInTheDocument();
    expect(screen.queryByText("Stale project message")).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
    expect(screen.getAllByRole("table")).toHaveLength(1);
    expect(screen.getAllByRole("table").every((table) => !table.hasAttribute("aria-live"))).toBe(true);
    const scanId = scanStart.mock.calls[0][0] as string;
    act(() => finish({ type: "canceled", scan_id: scanId }));
  });

  it("invalidates a dry run after selection changes, then confirms and reports execution", async () => {
    const refreshed = { ...dryRun, digest: "fixture-refreshed-live-preview", report: { ...dryRun.report, confirmation_digest: "fixture-refreshed-live-preview" } };
    const planDryRun = vi.fn().mockResolvedValueOnce(dryRun).mockResolvedValueOnce(refreshed);
    const planExecute = vi.fn().mockResolvedValue({ ...execution, confirmation_digest: refreshed.digest });
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ planDryRun, planExecute })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    expect(await screen.findByText("Scan complete. 1 targets.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Clean 500.0 MiB" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Review targets" }));
    expect(screen.getByText(/^Build artifacts/)).toBeInTheDocument();
    expect(screen.queryByText("build_artifacts")).not.toBeInTheDocument();
    expect(screen.queryByText("dangerous")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Review dry run" })).toBeDisabled();
    const target = screen.getByRole("checkbox", { name: "Select C:/work/app/target" });
    expect(target).not.toBeChecked();
    await user.click(target);
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(await screen.findByText("Dry-run preview")).toBeInTheDocument();
    expect(screen.getByText("Selected 1")).toBeInTheDocument();
    expect(screen.getAllByText("500.0 MiB").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Back to review" }));
    await user.click(screen.getByRole("checkbox", { name: "Select C:/work/app/target" }));
    expect(screen.queryByText("Dry-run preview")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Confirm cleanup" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Review dry run" })).toBeDisabled();
    expect(planExecute).not.toHaveBeenCalled();
    await user.click(screen.getByRole("checkbox", { name: "Select C:/work/app/target" }));
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(await screen.findByText("Dry-run preview")).toBeInTheDocument();
    expect(planDryRun).toHaveBeenCalledTimes(2);
    expect(screen.getByText("Selected 1")).toBeInTheDocument();
    expect(screen.getAllByText(refreshed.digest, { exact: false }).length).toBeGreaterThan(0);
    expect(refreshed.digest).not.toBe(dryRun.digest);

    await user.click(screen.getByRole("button", { name: "Confirm cleanup" }));
    expect(screen.getByRole("dialog")).toHaveTextContent(refreshed.digest);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Execute" }));
    const result = await screen.findByRole("tabpanel", { name: "Clean" });
    expect(await within(result).findByRole("heading", { name: "Execution completed." })).toBeInTheDocument();
    expect(within(result).getByText("1 succeeded · 0 skipped · 0 failed")).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/freed|released|释放/i);
    expect(document.querySelector(".display-capacity")).toHaveTextContent(/^500\.0 MiB$/);
    expect(screen.queryByText(/space freed|released space|4K/i)).not.toBeInTheDocument();
    expect(planExecute).toHaveBeenCalledWith(scanReport.plan, ["cargo.target:C:/work/app/target"], refreshed.digest);
    await user.click(screen.getByRole("button", { name: "Back to cleanup" }));
    expect(within(screen.getByRole("region", { name: "Project artifacts" })).getByText("Moved to trash; capacity becomes available after trash is emptied.")).toBeInTheDocument();
    expect(screen.getByText("1 succeeded · 0 skipped · 0 failed")).toBeInTheDocument();
  });

  it("reviews global commands independently while Cargo home remains inspect-only", async () => {
    const globalReport = decodeScanReport(scanReplays.global.report);
    const planExecute = vi.fn().mockResolvedValue({ ...npmDryRun.report, dry_run: false, audit_log: "C:/fixture-audit.jsonl" });
    const scanStart = vi.fn(async (scanId: string) => ({ type: "completed" as const, scan_id: scanId, report: globalReport }));
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart, planDryRun: vi.fn().mockResolvedValue(npmDryRun), planExecute })} presentationSettings={fakePresentationSettings()} />);
    await user.click(await screen.findByRole("button", { name: "Scan global caches" }));
    expect(scanStart.mock.calls[0]).toEqual(expect.arrayContaining([{ include_projects: false, include_global: true, roots: [] }]));
    await user.click(await screen.findByRole("button", { name: "Review global targets" }));
    expect(screen.getAllByText(/^Package caches/)[0]).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /C:\/Users\/dev\/\.cargo/ })).toBeDisabled();
    expect(screen.queryByText("Build artifacts")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Review dry run" })).toBeDisabled();
    await user.click(screen.getByRole("checkbox", { name: /Select npm.cache.clean/ }));
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(await screen.findByText("Dry-run preview")).toBeInTheDocument();
    expect(planExecute).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Confirm cleanup" }));
    expect(screen.getByRole("dialog")).toHaveTextContent(npmDryRun.digest);
    expect(screen.getByRole("dialog").querySelector(".dialog-warning")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Execute" }));
    expect(await screen.findByText(/Owner commands completed: 1/)).toBeInTheDocument();
    expect(screen.queryByText(/space freed|released space/i)).not.toBeInTheDocument();
    expect(planExecute).toHaveBeenCalledWith(globalReport.plan, ["npm.cache.clean:global"], npmDryRun.digest);
  });

  it("keeps partial project selection indeterminate and previews the entire original scoped plan", async () => {
    const second = { ...scanReport.plan.targets[0], id: "cargo.target:C:/work/second/target", path: "C:/work/second/target", scope: { type: "project" as const, root: "C:/work/second" } };
    const report = { ...scanReport, plan: { ...scanReport.plan, targets: [...scanReport.plan.targets, second] }, health: { ...scanReport.health, totals: { ...scanReport.health.totals, verified_bytes: 1048576000 } } };
    const preview = { ...dryRun, digest: "fixture-two-project-preview", report: { ...dryRun.report, selected: 2, attempted: 2, succeeded: 2,
      outcomes: [...dryRun.report.outcomes, { ...dryRun.report.outcomes[0], target_id: second.id }],
      estimated_recoverable: { verified_bytes: 1048576000, partial_lower_bound_bytes: 0, unknown_target_count: 0 }, confirmation_digest: "fixture-two-project-preview" } };
    const planDryRun = vi.fn().mockResolvedValue(preview);
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({
      cleanScanPreferencesGet: async () => ({ version: 1, roots: ["C:/work/app", "C:/work/second"], suggestions: [] }),
      scanStart: vi.fn(async (scanId: string) => ({ type: "completed" as const, scan_id: scanId, report })),
      planDryRun,
    })} presentationSettings={fakePresentationSettings()} />);
    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    await user.click(await screen.findByRole("button", { name: "Review targets" }));
    const selectAll = screen.getByRole<HTMLInputElement>("checkbox", { name: "Select all executable targets" });
    expect(selectAll).not.toBeChecked();
    await user.click(screen.getByRole("checkbox", { name: "Select C:/work/app/target" }));
    expect(selectAll.indeterminate).toBe(true);
    await user.click(selectAll);
    expect(selectAll).toBeChecked();
    expect(selectAll.indeterminate).toBe(false);
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(planDryRun).toHaveBeenCalledWith(report.plan, [scanReport.plan.targets[0].id, second.id]);
    expect(await screen.findByText("Selected 2")).toBeInTheDocument();
  });

  it("disables conflicting scan controls during dry run and execution", async () => {
    const oneTargetExecution: ExecutionReport = { ...dryRun.report, dry_run: false, audit_log: "C:/fixture-audit.jsonl" };
    let resolveDryRun: (value: typeof dryRun) => void = () => undefined;
    let resolveExecution: (value: ExecutionReport) => void = () => undefined;
    const planDryRun = vi.fn().mockReturnValue(new Promise<typeof dryRun>((resolve) => { resolveDryRun = resolve; }));
    const planExecute = vi.fn().mockReturnValue(new Promise<ExecutionReport>((resolve) => { resolveExecution = resolve; }));
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ planDryRun, planExecute })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    await screen.findByText("Scan complete. 1 targets.");
    await user.click(screen.getByRole("button", { name: "Review targets" }));
    await user.click(screen.getByRole("checkbox", { name: "Select C:/work/app/target" }));
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    await user.click(screen.getByRole("button", { name: "Back to cleanup" }));
    expect(screen.getByRole("button", { name: "Scan project artifacts" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Scan global caches" })).toBeDisabled();
    await act(async () => resolveDryRun(dryRun));

    await user.click(screen.getByRole("button", { name: "Confirm cleanup" }));
    await user.click(screen.getByRole("button", { name: "Execute" }));
    expect(screen.getByRole("button", { name: "Execute" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    await act(async () => resolveExecution(oneTargetExecution));
    expect((await screen.findAllByText("Moved to trash; capacity becomes available after trash is emptied.")).length).toBeGreaterThan(0);
  });

  it.each([
    [{ code: "scan_already_running" } as const, "already running"],
    [{ code: "stale_confirmation", expected_digest: "old", actual_digest: "new" } as const, "run the dry run again"],
    [{ code: "unknown_target", target_id: "gone" } as const, "no longer available"],
  ])("renders recovery copy for $0.code", async (error, message) => {
    const user = userEvent.setup();
    render(<App
      bridge={fakeBridge({ scanStart: vi.fn().mockRejectedValue(error) })}
      presentationSettings={fakePresentationSettings()}
    />);
    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent(message));
  });

  it("orders real bridge scan, dry-run, and execute starts after old cancel and join", async () => {
    const coordinator = new OperationCoordinator();
    const events: string[] = [];
    const bridge = fakeBridge({
      scanStart: vi.fn().mockImplementation(async (scanId: string) => {
        events.push("scan.start");
        return { type: "completed", scan_id: scanId, report: scanReport };
      }),
      planDryRun: vi.fn().mockImplementation(async () => {
        events.push("dry-run.start");
        return dryRun;
      }),
      planExecute: vi.fn().mockImplementation(async () => {
        events.push("execute.start");
        return execution;
      }),
    });
    const user = userEvent.setup();

    await armActiveOperation(coordinator, events, "scan-old");
    render(<App
      bridge={bridge}
      presentationSettings={fakePresentationSettings()}
      coordinator={coordinator}
    />);
    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    expect(await screen.findByText("Scan complete. 1 targets.")).toBeInTheDocument();
    expect(events).toEqual(["scan-old.cancel", "scan-old.join", "scan.start"]);
    await waitFor(() => expect(coordinator.activeIdentity()).toBeNull());

    events.length = 0;
    await armActiveOperation(coordinator, events, "dry-run-old");
    await user.click(screen.getByRole("button", { name: "Review targets" }));
    await user.click(screen.getByRole("checkbox", { name: "Select C:/work/app/target" }));
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(await screen.findByText("Dry-run preview")).toBeInTheDocument();
    expect(events).toEqual(["dry-run-old.cancel", "dry-run-old.join", "dry-run.start"]);
    await waitFor(() => expect(coordinator.activeIdentity()).toBeNull());

    await user.click(screen.getByRole("button", { name: "Confirm cleanup" }));
    events.length = 0;
    await armActiveOperation(coordinator, events, "execute-old");
    await user.click(screen.getByRole("button", { name: "Execute" }));
    await waitFor(() => expect(events).toEqual([
      "execute-old.cancel",
      "execute-old.join",
      "execute.start",
    ]));
    await waitFor(() => expect(coordinator.activeIdentity()).toBeNull());
  });

  it("does not invoke a real bridge service when the coordinator is closed or cancellation fails", async () => {
    const closedCoordinator = new OperationCoordinator();
    await closedCoordinator.close();
    const closedBridge = fakeBridge();
    const user = userEvent.setup();
    const closedView = render(<App
      bridge={closedBridge}
      presentationSettings={fakePresentationSettings("zh-CN")}
      userLocales={["zh-CN"]}
      coordinator={closedCoordinator}
    />);
    await user.click(await screen.findByRole("button", { name: "扫描项目产物" }));
    expect(closedBridge.scanStart).not.toHaveBeenCalled();
    expect(await screen.findByRole("alert")).toHaveTextContent("Desktop operation failed: The desktop operation coordinator is closed.");
    expect(closedCoordinator.activeIdentity()).toBeNull();
    closedView.unmount();

    const refusedCoordinator = new OperationCoordinator();
    const refusedEvents: string[] = [];
    await armActiveOperation(refusedCoordinator, refusedEvents, "refused-old", true);
    const refusedBridge = fakeBridge();
    render(<App
      bridge={refusedBridge}
      presentationSettings={fakePresentationSettings()}
      coordinator={refusedCoordinator}
    />);
    await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
    await waitFor(() => expect(refusedCoordinator.activeIdentity()).toBeNull());
    expect(refusedEvents).toEqual(["refused-old.cancel", "refused-old.join"]);
    expect(refusedBridge.scanStart).not.toHaveBeenCalled();
  });

  it.each(["sync", "async"] as const)(
    "releases coordinator ownership after a %s bridge start failure",
    async (failureKind) => {
      const coordinator = new OperationCoordinator();
      const scanStart = failureKind === "sync"
        ? vi.fn().mockImplementation(() => { throw new Error("sync start failure"); })
        : vi.fn().mockRejectedValue(new Error("async start failure"));
      const user = userEvent.setup();
      render(<App
        bridge={fakeBridge({ scanStart })}
        presentationSettings={fakePresentationSettings()}
        coordinator={coordinator}
      />);

      await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
      expect(await screen.findByRole("alert")).toHaveTextContent(`${failureKind} start failure`);
      expect(scanStart).toHaveBeenCalledOnce();
      await waitFor(() => expect(coordinator.activeIdentity()).toBeNull());
    },
  );

  it.each([false, true])(
    "closes an active App operation on unmount without survivors or unhandled rejection (cancel rejects: %s)",
    async (cancelRejects) => {
      const coordinator = new OperationCoordinator();
      const scanResult = deferred<DesktopScanResult>();
      const unhandled = vi.fn();
      window.addEventListener("unhandledrejection", unhandled);
      const bridge = fakeBridge({
        scanStart: vi.fn(() => scanResult.promise),
        scanCancel: vi.fn(async (scanId: string) => {
          scanResult.resolve({ type: "canceled", scan_id: scanId });
          if (cancelRejects) throw new Error("injected unmount cancellation failure");
        }),
      });
      const user = userEvent.setup();
      const view = render(<App
        bridge={bridge}
        presentationSettings={fakePresentationSettings()}
        coordinator={coordinator}
      />);
      await user.click(await screen.findByRole("button", { name: "Scan project artifacts" }));
      await waitFor(() => expect(coordinator.activeIdentity()?.kind).toBe("clean.scan"));

      view.unmount();
      await waitFor(() => expect(coordinator.activeIdentity()).toBeNull());
      await act(async () => { await Promise.resolve(); });
      expect(bridge.scanCancel).toHaveBeenCalledOnce();
      expect(unhandled).not.toHaveBeenCalled();
      window.removeEventListener("unhandledrejection", unhandled);
    },
  );

  it.each([false, true])(
    "awaits the real App close listener drain before Tauri owns last-window destroy (cancel rejects: %s)",
    async (cancelRejects) => {
      const coordinator = new OperationCoordinator();
      const closeSpy = vi.spyOn(coordinator, "close");
      const events: string[] = [];
      await armActiveOperation(coordinator, events, "native-close", cancelRejects);
      let closeHandler: (() => Promise<void>) | undefined;
      const unlisten = vi.fn();
      const lifecycle = fakeLifecycle({
        onCloseRequested: vi.fn(async (handler) => {
          closeHandler = handler;
          return unlisten;
        }),
      });
      const unhandled = vi.fn();
      window.addEventListener("unhandledrejection", unhandled);
      const view = render(<App
        bridge={fakeBridge()}
        presentationSettings={fakePresentationSettings()}
        coordinator={coordinator}
        lifecycle={lifecycle}
      />);
      await waitFor(() => expect(closeHandler).toBeDefined());

      const firstClose = closeHandler!();
      const duplicateClose = closeHandler!();
      await Promise.all([firstClose, duplicateClose]);
      events.push("tauri.destroy");
      expect(events).toEqual(["native-close.cancel", "native-close.join", "tauri.destroy"]);
      expect(coordinator.activeIdentity()).toBeNull();
      view.unmount();
      await act(async () => { await Promise.resolve(); });
      expect(closeSpy).toHaveBeenCalledOnce();
      expect(unlisten).toHaveBeenCalledOnce();
      expect(unhandled).not.toHaveBeenCalled();
      window.removeEventListener("unhandledrejection", unhandled);
    },
  );

  it("unlistens a late native close registration and reuses the unmount drain", async () => {
    const coordinator = new OperationCoordinator();
    const closeSpy = vi.spyOn(coordinator, "close");
    const registration = deferred<() => void>();
    const unlisten = vi.fn();
    const lifecycle = fakeLifecycle({
      onCloseRequested: vi.fn(() => registration.promise),
    });
    const view = render(<App
      bridge={fakeBridge()}
      presentationSettings={fakePresentationSettings()}
      coordinator={coordinator}
      lifecycle={lifecycle}
    />);

    view.unmount();
    registration.resolve(unlisten);

    await waitFor(() => expect(unlisten).toHaveBeenCalledOnce());
    expect(closeSpy).toHaveBeenCalledOnce();
  });

  it("projects the exact debug one-shot route fault through the real App shell", async () => {
    const lifecycle = fakeLifecycle({
      nativeFaultMode: vi.fn().mockResolvedValue("route_cancel_once"),
    });
    const user = userEvent.setup();
    render(<App
      bridge={fakeBridge()}
      presentationSettings={fakePresentationSettings()}
      lifecycle={lifecycle}
    />);
    const settings = await screen.findByRole("tab", { name: "Settings" });
    await waitFor(() => expect(lifecycle.nativeFaultMode).toHaveBeenCalledOnce());
    await act(async () => { await Promise.resolve(); });

    await openLanguageSettings(user);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not change destination. The current page remains active.",
    );
    expect(window.location.hash).toBe("#/clean");
    expect(settings).toHaveFocus();

    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByText("Could not change destination. The current page remains active.")).not.toBeInTheDocument();
    await openLanguageSettings(user);
    expect(await screen.findByRole("combobox", { name: "Language" })).toHaveTextContent("English");
    expect(window.location.hash).toBe("#/settings");
  });
});
