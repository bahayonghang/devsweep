import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import englishCatalogue from "../../../../resources/i18n/en.json";
import chineseCatalogue from "../../../../resources/i18n/zh-CN.json";
import progressJson from "../../api/fixtures/scan-progress.json";
import scanJson from "../../api/fixtures/scan-report.json";
import type { DesktopBridge } from "../../api/bridge";
import {
  decodeDesktopScanProgress,
  decodeScanReport,
} from "../../api/contract";
import { fixtureBridge } from "../../api/fixture-bridge";
import type {
  DesktopScanProgress,
  DesktopScanResult,
} from "../../api/types.gen";
import { message, type PresentationLanguageTag } from "../../i18n";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { CleanWorkbench } from "./CleanWorkbench";

const report = decodeScanReport(scanJson);
const progress = decodeDesktopScanProgress(progressJson);
const LOCALES: readonly PresentationLanguageTag[] = ["en", "zh-CN"];
const WIDTHS = [390, 800, 1024, 1440] as const;
const originalWidth = window.innerWidth;

afterEach(() => {
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    value: originalWidth,
  });
});

function setWidth(width: number) {
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    value: width,
  });
  window.dispatchEvent(new Event("resize"));
}

function bridge(overrides: Partial<DesktopBridge> = {}): DesktopBridge {
  return {
    ...fixtureBridge,
    scanStart: vi
      .fn()
      .mockImplementation(async (scanId: string) => ({
        type: "completed",
        scan_id: scanId,
        report,
      })),
    ...overrides,
  };
}

type StageCase = {
  readonly name: string;
  readonly bridge: () => DesktopBridge;
  readonly heading: (locale: PresentationLanguageTag) => RegExp | string;
};

const pending = () => new Promise<DesktopScanResult>(() => undefined);

const STAGES: readonly StageCase[] = [
  {
    name: "home",
    bridge: () => bridge(),
    heading: (locale) => message(locale, "clean.v1.stage.headline"),
  },
  {
    name: "scanning",
    bridge: () =>
      bridge({
        scanStart: vi
          .fn()
          .mockImplementation(
            (
              scanId: string,
              _options,
              onProgress: (value: DesktopScanProgress) => void,
            ) => {
              onProgress({ ...progress, scan_id: scanId });
              return pending();
            },
          ),
      }),
    heading: (locale) => message(locale, "clean.v1.stage.scanning_headline"),
  },
  {
    name: "found",
    bridge: () => bridge(),
    heading: (locale) =>
      new RegExp(message(locale, "clean.v1.stage.found_caption")),
  },
  {
    name: "empty",
    bridge: () =>
      bridge({
        scanStart: vi
          .fn()
          .mockImplementation(async (scanId: string) => ({
            type: "completed",
            scan_id: scanId,
            report: { ...report, plan: { ...report.plan, targets: [] } },
          })),
      }),
    heading: (locale) => message(locale, "clean.v1.scan.empty"),
  },
  {
    name: "error",
    bridge: () =>
      bridge({
        scanStart: vi
          .fn()
          .mockRejectedValue({
            code: "scan_failed",
            message: "controlled failure",
          }),
      }),
    heading: (locale) => message(locale, "clean.v1.stage.stopped_failed"),
  },
  {
    name: "canceled",
    bridge: () =>
      bridge({
        scanStart: vi
          .fn()
          .mockImplementation(
            async (
              scanId: string,
              _options,
              onProgress: (value: DesktopScanProgress) => void,
            ) => {
              onProgress({ ...progress, scan_id: scanId });
              return { type: "canceled", scan_id: scanId };
            },
          ),
      }),
    heading: (locale) => message(locale, "clean.v1.scan.canceled"),
  },
];

describe.each(LOCALES)("Clean stage states in %s", (locale) => {
  it.each(STAGES.map((stage) => [stage.name, stage] as const))(
    "renders %s at 390, 800, 1024, and 1440 CSS px",
    async (_name, stage) => {
      for (const width of WIDTHS) {
        setWidth(width);
        const user = userEvent.setup();
        const view = render(
          <CleanWorkbench
            bridge={stage.bridge()}
            coordinator={new OperationCoordinator()}
            locale={locale}
          />,
        );
        if (stage.name !== "home")
          await user.click(
            screen.getByRole("button", {
              name: message(locale, "clean.v1.action.scan"),
            }),
          );
        const heading = await screen.findByRole("heading", {
          level: 2,
          name: stage.heading(locale),
        });
        const region = heading.closest(".stage");
        expect(region).not.toBeNull();
        expect(
          region?.querySelector(".planet[aria-hidden='true']"),
        ).toBeInTheDocument();
        expect(region?.querySelector(".card")).not.toBeInTheDocument();
        if (stage.name === "error")
          expect(screen.getByRole("alert")).toHaveTextContent(
            "controlled failure",
          );
        expect(document.body.textContent).not.toMatch(/freed|released|释放/i);
        if (locale === "zh-CN")
          expect(region?.textContent).not.toMatch(/Scan|Review|Found|Moved/);
        view.unmount();
      }
    },
    20_000,
  );
});

describe("Clean found stage", () => {
  it("shows the found total and opens review from the Review action", async () => {
    const user = userEvent.setup();
    render(
      <CleanWorkbench
        bridge={bridge()}
        coordinator={new OperationCoordinator()}
        locale="en"
      />,
    );
    await user.click(screen.getByRole("button", { name: "Scan" }));
    const stage = await screen.findByRole("region", { name: "Clean" });
    expect(within(stage).getByRole("heading", { level: 2 })).toHaveTextContent(
      "Found in this scan 628.0 MiB",
    );
    expect(
      within(stage).getByText("Scan complete. 3 targets."),
    ).toBeInTheDocument();
    expect(
      within(stage).getByText("At least 128.0 MiB (partial)"),
    ).toBeInTheDocument();
    await user.click(
      within(stage).getByRole("button", { name: "Review targets" }),
    );
    expect(
      screen.getByRole("button", { name: "Review dry run" }),
    ).toBeInTheDocument();
  });
});

async function openReview(overrides: Partial<DesktopBridge> = {}) {
  const user = userEvent.setup();
  const current = bridge(overrides);
  render(
    <CleanWorkbench
      bridge={current}
      coordinator={new OperationCoordinator()}
      locale="en"
    />,
  );
  await user.click(screen.getByRole("button", { name: "Scan" }));
  await user.click(
    await screen.findByRole("button", { name: "Review targets" }),
  );
  return { user, bridge: current };
}

describe("Clean review row controls", () => {
  it("skips a row into the collapsed Skipped group and restores it; inspect-only rows have no Skip", async () => {
    const { user } = await openReview();
    expect(screen.getByText("Selected 1")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Skip C:/Users/dev/.cargo" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Skip npm.cache.clean:global" }),
    ).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "Skip C:/work/app/target" }),
    );
    expect(screen.getByText("Selected 0")).toBeInTheDocument();
    const skipped = document.querySelector("details.skipped-group");
    expect(skipped).not.toBeNull();
    expect(skipped).not.toHaveAttribute("open");
    expect(document.querySelector(".preview-groups")?.lastElementChild).toBe(
      skipped,
    );
    await user.click(within(skipped as HTMLElement).getByText("Skipped 1"));
    expect(
      within(skipped as HTMLElement).getByRole("checkbox", {
        name: /C:\/work\/app\/target/,
      }),
    ).toBeDisabled();

    await user.click(
      within(skipped as HTMLElement).getByRole("button", {
        name: "Restore C:/work/app/target",
      }),
    );
    expect(document.querySelector("details.skipped-group")).toBeNull();
    expect(screen.getByText("Selected 1")).toBeInTheDocument();
  });

  it("requires confirmation, calls protectionAdd with the path and confirm=true, and marks the row protected", async () => {
    const protectionAdd = vi.fn().mockResolvedValue({
      operation_id: "op-protect",
      action: "add",
      outcome_code: "committed",
      error_code: null,
      identity_sha256: "a".repeat(64),
      display_path: "C:/work/app/target",
      changed: true,
    });
    const { user } = await openReview({ protectionAdd });
    expect(
      screen.queryByRole("button", { name: /^Protect npm\.cache\.clean/ }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Protect C:/Users/dev/.cargo" }),
    ).toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "Protect C:/work/app/target" }),
    );
    const dialog = screen.getByRole("dialog", { name: "Protect this path?" });
    expect(protectionAdd).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(protectionAdd).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await user.click(
      screen.getByRole("button", { name: "Protect C:/work/app/target" }),
    );
    await user.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Protect",
      }),
    );
    expect(protectionAdd).toHaveBeenCalledExactlyOnceWith(
      "C:/work/app/target",
      true,
    );
    expect(
      await screen.findByText("Protected; scan again to refresh"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", { name: /C:\/work\/app\/target/ }),
    ).toBeDisabled();
    expect(
      screen.queryByRole("button", { name: "Protect C:/work/app/target" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Skip C:/work/app/target" }),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Selected 0")).toBeInTheDocument();
    expect(
      screen.getByText(
        "The protection list changed. Scan again to refresh the report.",
      ),
    ).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Back to overview" }));
    expect(
      within(screen.getByRole("region", { name: "Clean" })).getByText(
        "The protection list changed. Scan again to refresh the report.",
      ),
    ).toBeInTheDocument();
  });

  it("keeps the row selectable and shows the error when protectionAdd fails", async () => {
    const protectionAdd = vi
      .fn()
      .mockRejectedValue({
        code: "protection_store_unavailable",
        message: "store locked",
      });
    const { user } = await openReview({ protectionAdd });
    await user.click(
      screen.getByRole("button", { name: "Protect C:/work/app/target" }),
    );
    await user.click(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Protect",
      }),
    );
    expect(
      await within(screen.getByRole("dialog")).findByRole("alert"),
    ).toBeInTheDocument();
    expect(
      screen.queryByText("Protected; scan again to refresh"),
    ).not.toBeInTheDocument();
    expect(screen.getByText("Selected 1")).toBeInTheDocument();
  });
});

describe("Clean result copy", () => {
  it("never says freed or released in Clean or stage catalogue entries", () => {
    for (const catalogue of [englishCatalogue, chineseCatalogue]) {
      for (const [key, entry] of Object.entries(catalogue.messages)) {
        if (!key.startsWith("clean.") && !key.startsWith("stage.")) continue;
        expect(`${key}: ${JSON.stringify(entry.forms)}`).not.toMatch(
          /freed|released|释放/i,
        );
      }
    }
    expect(message("en", "clean.v1.result.caption")).toBe(
      "Moved to Recycle Bin",
    );
    expect(message("zh-CN", "clean.v1.result.caption")).toBe("已移到回收站");
  });
});
