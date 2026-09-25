import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StrictMode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import scanJson from "./api/fixtures/scan-report.json";
import dryRunJson from "./api/fixtures/dry-run-outcome.json";
import twoTargetDryRunJson from "./api/fixtures/dry-run-outcome-two-targets.json";
import executionJson from "./api/fixtures/execution-report.json";
import progressJson from "./api/fixtures/scan-progress.json";
import { decodeDesktopScanProgress, decodeDryRunOutcome, decodeExecutionReport, decodeScanReport } from "./api/contract";
import type { DesktopBridge } from "./api/bridge";
import { fixtureBridge } from "./api/fixture-bridge";
import type { DesktopScanProgress, DesktopScanResult, ExecutionReport } from "./api/types.gen";
import { App } from "./App";
import { formatBytes } from "./components/format";
import type { PresentationSettings, PresentationSettingsBridge } from "./i18n";
import type { DesktopLifecycleBridge } from "./lifecycle";
import { OperationCoordinator } from "./state/operation-coordinator";

const scanReport = decodeScanReport(scanJson);
const dryRun = decodeDryRunOutcome(dryRunJson);
const twoTargetDryRun = decodeDryRunOutcome(twoTargetDryRunJson);
const execution = decodeExecutionReport(executionJson);
const progress = decodeDesktopScanProgress(progressJson);

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
  brand = "DevSweep menu",
  language = "Language",
) {
  await user.click(await screen.findByRole("button", { name: brand }));
  await user.click(screen.getByRole("menuitem", { name: language }));
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
    expect(await screen.findByRole("button", { name: "扫描" })).toBeInTheDocument();
    expect(screen.getByText("项目")).toBeInTheDocument();
    expect(screen.getByText("全局缓存")).toBeInTheDocument();
    expect(screen.getByText("就绪")).toBeInTheDocument();
    expect(screen.queryByText("Projects")).not.toBeInTheDocument();
    expect(screen.queryByText("Scan")).not.toBeInTheDocument();
    expect(screen.queryByText("Search")).not.toBeInTheDocument();
    expect(screen.queryByText("Ready")).not.toBeInTheDocument();
    expect(screen.queryByText("没有扫描结果")).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "先逐项审核，再移动任何文件。" })).toBeInTheDocument();
    expect(screen.queryByText("Found so far")).not.toBeInTheDocument();
    expect(screen.queryByText("目前已发现")).not.toBeInTheDocument();
    expect(document.querySelector(".sweep-body-hero")).not.toBeInTheDocument();
    expect(document.querySelector(".clean-stage")).toBeInTheDocument();
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

    expect(await screen.findByRole("button", { name: "扫描" })).toBeInTheDocument();
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    await user.click(screen.getByRole("button", { name: "扫描" }));
    expect(scanStart).toHaveBeenCalledOnce();
    expect(screen.queryByText("就绪")).not.toBeInTheDocument();
  });

  it("starts Status live after StrictMode replays the lifecycle effect", async () => {
    const statusLiveStart = vi.fn().mockResolvedValue({ type: "canceled", operation_id: "strict-status-live" });
    const user = userEvent.setup();
    render(<StrictMode>
      <App bridge={fakeBridge({ statusLiveStart })} presentationSettings={fakePresentationSettings()} />
    </StrictMode>);

    expect(await screen.findByRole("button", { name: "Scan" })).toBeInTheDocument();
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    await user.click(screen.getByRole("tab", { name: "Status" }));
    const startLive = await screen.findByRole("button", { name: "Start live" });
    await waitFor(() => expect(startLive).toBeEnabled());
    await user.click(startLive);
    expect(statusLiveStart).toHaveBeenCalledOnce();
  });

  it("idle Clean home is a planet stage with visible scope and no cards", async () => {
    render(<App bridge={fakeBridge()} presentationSettings={fakePresentationSettings()} />);
    expect(await screen.findByRole("button", { name: "Scan" })).toBeInTheDocument();
    expect(screen.getByText("Projects")).toBeInTheDocument();
    expect(screen.getByText("Global caches")).toBeInTheDocument();
    expect(screen.getByText("Ready")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Review every target before anything moves." })).toBeInTheDocument();
    expect(screen.queryByText("No scan results")).not.toBeInTheDocument();
    expect(screen.queryByText("Found so far")).not.toBeInTheDocument();
    expect(document.querySelector(".sweep-body-hero")).not.toBeInTheDocument();
    expect(document.querySelector(".clean-stage")).toBeInTheDocument();
    expect(document.querySelector(".clean-stage .planet")).toBeInTheDocument();
    expect(document.querySelector(".clean-stage .card")).not.toBeInTheDocument();
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
    await user.selectOptions(screen.getByRole("combobox", { name: "Language" }), "zh-CN");
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
    await openLanguageSettings(user, "DevSweep 菜单", "语言");
    await user.selectOptions(screen.getByRole("combobox", { name: "语言" }), "en");
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
    await user.selectOptions(select, "zh-CN");
    expect(presentationSettings.save).toHaveBeenCalledWith("zh-CN");
    expect(select).toBeDisabled();
    expect(select).toHaveValue("en");
    expect(screen.getByRole("tab", { name: "Clean" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Saving language preference");

    act(() => save.resolve({ language: "zh-CN" }));
    expect(await screen.findByRole("tab", { name: "清理" })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "语言" })).toHaveValue("zh-CN");
  });

  it("preserves the prior locale and selection when save rejects", async () => {
    const presentationSettings: PresentationSettingsBridge = {
      load: vi.fn().mockResolvedValue({ language: "en" }),
      save: vi.fn().mockRejectedValue(new Error("store unavailable")),
    };
    const user = userEvent.setup();
    render(<App bridge={fakeBridge()} presentationSettings={presentationSettings} />);

    await openLanguageSettings(user);
    await user.selectOptions(screen.getByRole("combobox", { name: "Language" }), "zh-CN");

    expect(await screen.findByRole("alert")).toHaveTextContent("Language preference was not changed");
    expect(screen.getByRole("combobox", { name: "Language" })).toHaveValue("en");
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
    await user.selectOptions(screen.getByRole("combobox", { name: "Language" }), "zh-CN");

    expect(await screen.findByRole("alert")).toHaveTextContent("Language preference was not changed");
    expect(screen.getByRole("combobox", { name: "Language" })).toHaveValue("en");
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
    await user.selectOptions(screen.getByRole("combobox", { name: "Language" }), "zh-CN");
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

    await user.click(await screen.findByRole("button", { name: "Scan" }));
    await screen.findByText("Scan complete. 3 targets.");
    await openLanguageSettings(user);
    await user.selectOptions(screen.getByRole("combobox", { name: "Language" }), "zh-CN");
    await user.click(screen.getByRole("tab", { name: "清理" }));
    await user.click(screen.getByRole("button", { name: "扫描" }));
    await waitFor(() => expect(scanStart).toHaveBeenCalledTimes(2));
    expect(scanStart.mock.calls[0][1]).toEqual(scanStart.mock.calls[1][1]);
    expect(JSON.stringify(scanReport)).toBe(JSON.stringify(scanJson));
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

    await user.click(await screen.findByRole("button", { name: "Scan" }));
    expect(screen.getByText("Scan completed with partial evidence. Totals are not exact.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Found so far" })).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Scan" })).not.toHaveAttribute("aria-valuenow");
    expect(screen.queryByRole("checkbox", { name: /Select node.node_modules/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Review dry run" })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Cancel scan" }));
    expect(scanCancel).toHaveBeenCalledWith(expect.any(String));
    expect(screen.getByText("Cancel scan")).toBeInTheDocument();
    const firstScanId = scanStart.mock.calls[0][0] as string;
    act(() => finishFirst({ type: "canceled", scan_id: firstScanId }));
    expect(await screen.findByRole("heading", { name: "Scan canceled. Results stay non-selectable." })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Clean" }).querySelector(".planet")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Show details" }));
    expect(screen.getByRole("heading", { name: "Projects 1" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Scan" }));
    expect(await screen.findByText("Scan complete. 3 targets.")).toBeInTheDocument();
    expect(scanStart).toHaveBeenCalledTimes(2);
  });

  it("distinguishes an active empty scan from a completed empty report", async () => {
    let finish: (result: DesktopScanResult) => void = () => undefined;
    const scanStart = vi.fn().mockImplementation(() => new Promise<DesktopScanResult>((resolve) => { finish = resolve; }));
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan" }));
    expect(screen.getByRole("heading", { name: "Scanning. Targets appear as they are found." })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Found so far" })).toBeInTheDocument();
    expect(screen.queryByText("No scan results")).not.toBeInTheDocument();
    const scanId = scanStart.mock.calls[0][0] as string;
    act(() => finish({ type: "completed", scan_id: scanId, report: { ...scanReport, plan: { ...scanReport.plan, targets: [] } } }));
    expect(await screen.findByText("Scan complete; no cleanup targets found.")).toBeInTheDocument();
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

    await user.click(await screen.findByRole("button", { name: "Scan" }));
    await screen.findByText("Scan complete. 3 targets.");
    await user.click(screen.getByRole("button", { name: "Scan" }));
    const rescanId = scanStart.mock.calls[1][0] as string;
    act(() => finishRescan({ type: "canceled", scan_id: rescanId }));

    await user.click(await screen.findByRole("button", { name: "Show details" }));
    const returnButton = await screen.findByRole("button", { name: "Cancel" });
    expect(screen.queryByRole("button", { name: "Review dry run" })).not.toBeInTheDocument();
    await user.click(returnButton);
    expect(await screen.findByText("Scan complete. 3 targets.")).toBeInTheDocument();
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

    await user.click(await screen.findByRole("button", { name: "Scan" }));
    await waitFor(() => expect(scanCancel).toHaveBeenCalledOnce());
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    expect(screen.getByRole("alert")).toHaveTextContent("Malformed scan progress");
    const scanId = scanStart.mock.calls[0][0] as string;
    act(() => finish({ type: "canceled", scan_id: scanId }));
    expect(await screen.findByRole("heading", { name: "Scan stopped with an error. Found targets stay non-selectable." })).toBeInTheDocument();
  });

  it("groups cumulative preview rows by scope without announcing the table", async () => {
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
          targets: [globalTarget, projectTarget],
          totals: { target_count: 2, verified_bytes: 12288, partial_lower_bound_bytes: 0, unknown_target_count: 0 },
        },
      });
      onProgress({ ...progress, scan_id: scanId, sequence: 1, message: "Stale project message" });
      return new Promise<DesktopScanResult>((resolve) => { finish = resolve; });
    });
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ scanStart })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan" }));
    expect(screen.getByRole("heading", { name: "Found so far" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Projects 1" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Global caches 1" })).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("Scanning controlled global providers");
    expect(screen.queryByText("Stale project message")).not.toBeInTheDocument();
    expect(screen.getAllByRole("table")).toHaveLength(2);
    expect(screen.getAllByRole("table").every((table) => !table.hasAttribute("aria-live"))).toBe(true);
    const scanId = scanStart.mock.calls[0][0] as string;
    act(() => finish({ type: "canceled", scan_id: scanId }));
  });

  it("invalidates a dry run after selection changes, then confirms and reports execution", async () => {
    const planDryRun = vi.fn().mockResolvedValueOnce(dryRun).mockResolvedValueOnce(twoTargetDryRun);
    const planExecute = vi.fn().mockResolvedValue(execution);
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ planDryRun, planExecute })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan" }));
    expect(await screen.findByText("Scan complete. 3 targets.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent("Found in this scan 628.0 MiB");
    await user.click(screen.getByRole("button", { name: "Review targets" }));
    expect(screen.getByText("Build artifacts")).toBeInTheDocument();
    expect(screen.getByText("Package caches")).toBeInTheDocument();
    expect(screen.queryByText("build_artifacts")).not.toBeInTheDocument();
    expect(screen.queryByText("dangerous")).not.toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /C:\/Users\/dev\/\.cargo/ })).toBeDisabled();
    expect(screen.getByRole<HTMLInputElement>("checkbox", { name: "Select all executable targets" }).indeterminate).toBe(true);
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(await screen.findByText("Dry-run preview")).toBeInTheDocument();
    expect(screen.getByText("Selected 1")).toBeInTheDocument();
    expect(screen.getAllByText("500.0 MiB").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await user.click(screen.getByRole("checkbox", { name: /npm.cache.clean/ }));
    expect(screen.queryByText("Dry-run preview")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(await screen.findByText("Dry-run preview")).toBeInTheDocument();
    expect(planDryRun).toHaveBeenCalledTimes(2);
    expect(screen.getByText("Selected 2")).toBeInTheDocument();
    expect(screen.getAllByText(/500\.0 MiB \+ at least 128\.0 MiB/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(twoTargetDryRun.digest, { exact: false }).length).toBeGreaterThan(0);
    expect(twoTargetDryRun.digest).not.toBe(dryRun.digest);

    await user.click(screen.getByRole("button", { name: "Confirm cleanup" }));
    expect(screen.getByRole("dialog")).toHaveTextContent(twoTargetDryRun.digest);
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Execute" }));
    const result = await screen.findByRole("region", { name: "Clean" });
    expect(within(result).getByRole("heading", { level: 2 })).toHaveTextContent("Moved to Recycle Bin 500.0 MiB");
    expect(within(result).getByText("2 succeeded · 0 skipped · 0 failed")).toBeInTheDocument();
    expect(within(result).getByText(/capacity becomes available after trash is emptied/)).toBeInTheDocument();
    expect(await within(result).findByText("Total moved to Recycle Bin by DevSweep: at least 12.0 GiB")).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/freed|released|释放/i);
    await user.click(within(result).getByRole("button", { name: "Show details" }));
    expect((await screen.findAllByText("Execution completed.")).length).toBeGreaterThan(0);
    expect(document.querySelector(".display-capacity")).toHaveTextContent(/500\.0 MiB \+ at least 128\.0 MiB/);
    expect(screen.queryByText(/space freed|released space|4K/i)).not.toBeInTheDocument();
    expect(planExecute).toHaveBeenCalledWith(scanReport.plan, expect.arrayContaining(["cargo.target:C:/work/app/target", "npm.cache.clean:global"]), twoTargetDryRun.digest);
  });

  it("disables conflicting scan controls during dry run and execution", async () => {
    const oneTargetExecution: ExecutionReport = { ...dryRun.report, dry_run: false, audit_log: "C:/fixture-audit.jsonl" };
    let resolveDryRun: (value: typeof dryRun) => void = () => undefined;
    let resolveExecution: (value: ExecutionReport) => void = () => undefined;
    const planDryRun = vi.fn().mockReturnValue(new Promise<typeof dryRun>((resolve) => { resolveDryRun = resolve; }));
    const planExecute = vi.fn().mockReturnValue(new Promise<ExecutionReport>((resolve) => { resolveExecution = resolve; }));
    const user = userEvent.setup();
    render(<App bridge={fakeBridge({ planDryRun, planExecute })} presentationSettings={fakePresentationSettings()} />);

    await user.click(await screen.findByRole("button", { name: "Scan" }));
    await screen.findByText("Scan complete. 3 targets.");
    await user.click(screen.getByRole("button", { name: "Review targets" }));
    await user.click(screen.getByRole("button", { name: "Review dry run" }));
    expect(screen.getByRole("button", { name: "Scan" })).toBeDisabled();
    await act(async () => resolveDryRun(dryRun));

    await user.click(screen.getByRole("button", { name: "Confirm cleanup" }));
    await user.click(screen.getByRole("button", { name: "Execute" }));
    expect(screen.getByRole("button", { name: "Scan" })).toBeDisabled();
    await act(async () => resolveExecution(oneTargetExecution));
    expect(await screen.findByText("Moved to Recycle Bin")).toBeInTheDocument();
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
    await user.click(await screen.findByRole("button", { name: "Scan" }));
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
    await user.click(await screen.findByRole("button", { name: "Scan" }));
    expect(await screen.findByText("Scan complete. 3 targets.")).toBeInTheDocument();
    expect(events).toEqual(["scan-old.cancel", "scan-old.join", "scan.start"]);
    await waitFor(() => expect(coordinator.activeIdentity()).toBeNull());

    events.length = 0;
    await armActiveOperation(coordinator, events, "dry-run-old");
    await user.click(screen.getByRole("button", { name: "Review targets" }));
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
    await user.click(await screen.findByRole("button", { name: "扫描" }));
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
    await user.click(await screen.findByRole("button", { name: "Scan" }));
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

      await user.click(await screen.findByRole("button", { name: "Scan" }));
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
      await user.click(await screen.findByRole("button", { name: "Scan" }));
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
    const brand = await screen.findByRole("button", { name: "DevSweep menu" });
    await waitFor(() => expect(lifecycle.nativeFaultMode).toHaveBeenCalledOnce());
    await act(async () => { await Promise.resolve(); });

    await openLanguageSettings(user);

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not change destination. The current page remains active.",
    );
    expect(window.location.hash).toBe("#/clean");
    expect(brand).toHaveFocus();

    await user.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(screen.queryByText("Could not change destination. The current page remains active.")).not.toBeInTheDocument();
    await openLanguageSettings(user);
    expect(await screen.findByRole("combobox", { name: "Language" })).toHaveValue("en");
    expect(window.location.hash).toBe("#/settings");
  });
});
