import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { DesktopBridge } from "../../api/bridge";
import { fixtureBridge } from "../../api/fixture-bridge";
import { message, type PresentationLanguageTag } from "../../i18n";
import { OperationCoordinator } from "../../state/operation-coordinator";
import { SoftwareWorkbench } from "./SoftwareWorkbench";

const LOCALES: readonly PresentationLanguageTag[] = ["en", "zh-CN"];
const ELIGIBLE = "software:v1:msix:eligible-contoso";

function renderWorkbench(locale: PresentationLanguageTag, bridge: DesktopBridge = fixtureBridge) {
  render(<SoftwareWorkbench bridge={bridge} coordinator={new OperationCoordinator()} locale={locale} />);
}

function expectNoFreedCopy() {
  expect(document.body.textContent).not.toMatch(/freed|released|释放/i);
}

describe.each(LOCALES)("Software support views in %s", (locale) => {
  it("shows installed, update, and startup counts on the stage", async () => {
    renderWorkbench(locale);
    const stage = screen.getByRole("region", { name: message(locale, "command.software") });
    const notChecked = message(locale, "software.v1.stage.not_checked");
    expect(await within(stage).findByText(message(locale, "software.v1.stage.facts", { installed: notChecked, updates: notChecked, startup: "4" }))).toBeInTheDocument();
    expect(within(stage).getByRole("button", { name: message(locale, "software.v1.action.updates") })).toBeInTheDocument();
    expect(within(stage).getByRole("button", { name: message(locale, "software.v1.action.startup") })).toBeInTheDocument();
  });

  it("Updates view checks winget read-only and lists rows with truncation", async () => {
    const user = userEvent.setup();
    const bridge = { ...fixtureBridge, softwareUpdatesCheck: vi.fn(fixtureBridge.softwareUpdatesCheck) };
    renderWorkbench(locale, bridge);
    await user.click(screen.getByRole("button", { name: message(locale, "software.v1.action.updates") }));
    expect(screen.getByText(message(locale, "software.v1.updates.not_checked"))).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: message(locale, "software.v1.action.updates_check") }));
    expect(await screen.findByText(message(locale, "software.v1.updates.summary", { count: "3" }))).toBeInTheDocument();
    expect(screen.getByText("坚果云")).toBeInTheDocument();
    expect(screen.getByText("Microsoft Visual C++ 2015-2022 Redistr")).toBeInTheDocument();
    expect(screen.getAllByText(new RegExp(message(locale, "software.v1.updates.truncated"))).length).toBe(1);
    expect(bridge.softwareUpdatesCheck).toHaveBeenCalledWith(expect.stringMatching(/^software-updates-/), null);
    await user.click(screen.getByRole("button", { name: message(locale, "stage.v1.action.back") }));
    const notChecked = message(locale, "software.v1.stage.not_checked");
    expect(screen.getByText(message(locale, "software.v1.stage.facts", { installed: notChecked, updates: "3", startup: "4" }))).toBeInTheDocument();
    expectNoFreedCopy();
  });

  it("Updates view shows a stable unavailable reason", async () => {
    const user = userEvent.setup();
    renderWorkbench(locale, {
      ...fixtureBridge,
      softwareUpdatesCheck: async (operationId) => ({
        operation_id: operationId,
        updates: { state: "unavailable", version: 1, observed_at_unix_ms: 1, reason_code: "source_agreement_pending" },
      }),
    });
    await user.click(screen.getByRole("button", { name: message(locale, "software.v1.action.updates") }));
    await user.click(screen.getByRole("button", { name: message(locale, "software.v1.action.updates_check") }));
    expect(await screen.findByText(message(locale, "software.v1.updates.reason.source_agreement_pending"))).toBeInTheDocument();
    expect(screen.getByText(message(locale, "software.v1.updates.unavailable"))).toBeInTheDocument();
  });

  it("Startup view toggles only current-user rows and labels machine rows", async () => {
    const user = userEvent.setup();
    const bridge = { ...fixtureBridge, softwareStartupSet: vi.fn(fixtureBridge.softwareStartupSet) };
    renderWorkbench(locale, bridge);
    await user.click(screen.getByRole("button", { name: message(locale, "software.v1.action.startup") }));
    const userSwitch = await screen.findByRole("switch", { name: message(locale, "software.v1.startup.toggle", { name: "Contoso Sync" }) });
    const machineSwitch = screen.getByRole("switch", { name: message(locale, "software.v1.startup.toggle", { name: "SecurityHealth" }) });
    expect(userSwitch).toBeChecked();
    expect(userSwitch).toBeEnabled();
    expect(machineSwitch).toBeDisabled();
    expect(screen.getAllByText(message(locale, "software.v1.startup.requires_administrator"))).toHaveLength(2);
    expect(screen.getByText(message(locale, "software.v1.startup.sources_unavailable", { count: "1" }))).toBeInTheDocument();
    await user.click(userSwitch);
    await waitFor(() => expect(userSwitch).not.toBeChecked());
    expect(bridge.softwareStartupSet).toHaveBeenCalledWith(`startup:v1:${"1".repeat(64)}`, false, true);
    await user.click(machineSwitch);
    expect(bridge.softwareStartupSet).toHaveBeenCalledTimes(1);
  });

  it("Inventory view reviews leftovers after a succeeded uninstall and reports moved size", async () => {
    const user = userEvent.setup();
    const bridge: DesktopBridge = {
      ...fixtureBridge,
      softwareUninstall: async (operationId) => ({
        operation_id: operationId,
        report: {
          version: 1,
          irreversible: true,
          outcomes: [{ operation_id: "core-removed", software_id: ELIGIBLE, outcome: "removed", installed_state: "absent", reboot_evidence: "none", irreversible: true }],
        },
      }),
      softwareLeftoversExecute: vi.fn(fixtureBridge.softwareLeftoversExecute),
    };
    renderWorkbench(locale, bridge);
    await user.click(screen.getByRole("button", { name: message(locale, "software.v1.action.inventory") }));
    const eligible = await screen.findByRole("checkbox", { name: `Contoso Tools: ${message(locale, "software.v1.eligibility.eligible_current_user_msix")}` });
    await user.click(eligible);
    await user.click(screen.getByRole("button", { name: message(locale, "software.v1.action.preview") }));
    const uncertain = message(locale, "software.v1.leftovers.certainty.uncertain");
    const roaming = await screen.findByRole("checkbox", { name: `C:\\Users\\fixture\\AppData\\Roaming\\Contoso\\Contoso Tools: ${uncertain}` });
    expect(roaming).not.toBeChecked();
    expect(screen.queryByRole("button", { name: message(locale, "software.v1.action.leftovers_review") })).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: message(locale, "software.v1.action.confirm") }));
    await user.click(screen.getByRole("button", { name: message(locale, "software.v1.action.uninstall") }));
    const review = await screen.findByRole("button", { name: message(locale, "software.v1.action.leftovers_review") });
    expect(review).toBeDisabled();
    await user.click(roaming);
    await user.click(review);
    await user.click(await screen.findByRole("button", { name: message(locale, "software.v1.action.leftovers_confirm") }));
    const dialog = await screen.findByRole("dialog", { name: message(locale, "software.v1.leftovers.confirm.title") });
    await user.click(within(dialog).getByRole("button", { name: message(locale, "software.v1.action.leftovers_move") }));
    expect(await screen.findAllByText(message(locale, "software.v1.leftovers.result", {
      app: message(locale, "software.v1.size.measured", { bytes: "256.0 MiB" }),
      leftovers: "50.0 MiB",
    }))).not.toHaveLength(0);
    expect(bridge.softwareLeftoversExecute).toHaveBeenCalledWith(expect.any(String), expect.objectContaining({
      software_id: ELIGIBLE,
      uninstall_operation_id: "core-removed",
      selected_candidate_ids: [`leftover:v1:${"a".repeat(64)}`],
    }), `sha256:${"8".repeat(64)}`, true);
    expectNoFreedCopy();
  });
});
