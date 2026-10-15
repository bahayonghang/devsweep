import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { App } from "../../App";
import { fixtureBridge } from "../../api/fixture-bridge";
import reportJson from "../../api/fixtures/scan-report.json";
import { decodeScanReport } from "../../api/contract";
import { createFixturePreferencesBridge } from "../../preferences/fixture";
import { createFixtureWindowBridge } from "../../lifecycle";

it("retains a completed Clean report through Settings and back", async () => {
  window.history.replaceState(null, "", "#/clean");
  const user = userEvent.setup();
  const scanStart = vi.fn(async (scanId: string) => ({ type: "completed" as const, scan_id: scanId, report: decodeScanReport(reportJson) }));
  render(<App bridge={{ ...fixtureBridge, scanStart }}
    desktopPreferences={createFixturePreferencesBridge()} windowControls={createFixtureWindowBridge()}
    lifecycle={createFixtureWindowBridge()}
    presentationSettings={{ load: async () => ({ language: "en" }), save: async (language) => ({ language }) }} userLocales={["en"]} />);
  await user.click(await screen.findByRole("button", { name: /^(Scan|Scan project artifacts)$/ }));
  expect(await screen.findByRole("button", { name: "Review targets" })).toBeInTheDocument();
  await user.click(screen.getByRole("tab", { name: "Settings" }));
  await user.click(screen.getByRole("tab", { name: "Clean" }));
  expect(await screen.findByRole("button", { name: "Review targets" })).toBeInTheDocument();
  expect(scanStart).toHaveBeenCalledTimes(1);
});
