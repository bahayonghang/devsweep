import { describe, expect, it } from "vitest";
import updatesAvailableJson from "./fixtures/software/updates-available.json";
import updatesUnavailableJson from "./fixtures/software/updates-unavailable.json";
import startupListJson from "./fixtures/software/startup-list.json";
import startupToggleJson from "./fixtures/software/startup-toggle.json";
import leftoversDiscoveredJson from "./fixtures/software/leftovers-discovered.json";
import leftoversPlannedJson from "./fixtures/software/leftovers-planned.json";
import leftoversReportJson from "./fixtures/software/leftovers-report.json";
import {
  decodeDesktopSoftwareLeftoversPreviewResult,
  decodeDesktopSoftwareLeftoversResult,
  decodeDesktopSoftwareUpdatesResult,
  decodeSoftwareStartupList,
  decodeSoftwareStartupToggleReport,
} from "./contract";

describe("Software support decoders", () => {
  it("decode the update, startup, and leftover fixtures", () => {
    const available = decodeDesktopSoftwareUpdatesResult(updatesAvailableJson).updates;
    expect(available.state === "available" && available.rows.map((row) => row.name_truncated)).toEqual([false, false, true]);
    expect(decodeDesktopSoftwareUpdatesResult(updatesUnavailableJson).updates).toMatchObject({ state: "unavailable", reason_code: "source_agreement_pending" });
    expect(decodeSoftwareStartupList(startupListJson).entries.map((entry) => entry.toggle)).toEqual(["allowed", "allowed", "requires_administrator", "requires_administrator"]);
    expect(decodeSoftwareStartupToggleReport(startupToggleJson).entry.state).toBe("disabled");
    expect(decodeDesktopSoftwareLeftoversPreviewResult(leftoversDiscoveredJson).type).toBe("discovered");
    expect(decodeDesktopSoftwareLeftoversPreviewResult(leftoversPlannedJson).type).toBe("planned");
    expect(decodeDesktopSoftwareLeftoversResult(leftoversReportJson).report.moved_known_bytes).toBe(52428800);
  });

  it("reject unknown fields, open reason codes, and empty success", () => {
    expect(() => decodeDesktopSoftwareUpdatesResult({ ...updatesAvailableJson, argv: ["upgrade"] })).toThrow("unknown field");
    expect(() => decodeDesktopSoftwareUpdatesResult({ ...updatesUnavailableJson, updates: { ...updatesUnavailableJson.updates, reason_code: "freeform" } })).toThrow("reason_code");
    expect(() => decodeDesktopSoftwareUpdatesResult({ ...updatesUnavailableJson, updates: { ...updatesUnavailableJson.updates, rows: [] } })).toThrow("unknown field");
    const row = updatesAvailableJson.updates.rows[0];
    expect(() => decodeDesktopSoftwareUpdatesResult({ ...updatesAvailableJson, updates: { ...updatesAvailableJson.updates, rows: [{ ...row, command: "winget upgrade" }] } })).toThrow("unknown field");
  });

  it("reject machine startup rows that claim a current-user toggle", () => {
    const entries = structuredClone(startupListJson.entries);
    entries[2].toggle = "allowed";
    expect(() => decodeSoftwareStartupList({ ...startupListJson, entries })).toThrow("scope");
    expect(() => decodeSoftwareStartupToggleReport({ ...startupToggleJson, entry: startupListJson.entries[2] })).toThrow("toggle scope");
    expect(() => decodeSoftwareStartupToggleReport({ ...startupToggleJson, outcome: "failed" })).toThrow("toggle outcome");
  });

  it("reject leftover candidates that break certainty, plan, or moved-total rules", () => {
    const discovered = structuredClone(leftoversDiscoveredJson);
    discovered.preview.apps[0].candidates[0].selected_by_default = true;
    expect(() => decodeDesktopSoftwareLeftoversPreviewResult(discovered)).toThrow("certainty");
    const planned = structuredClone(leftoversPlannedJson);
    planned.plan.selected_candidate_ids = [`leftover:v1:${"b".repeat(64)}`];
    expect(() => decodeDesktopSoftwareLeftoversPreviewResult(planned)).toThrow("does not match its plan");
    const report = structuredClone(leftoversReportJson);
    report.report.moved_known_bytes = 1;
    expect(() => decodeDesktopSoftwareLeftoversResult(report)).toThrow("moved total");
  });
});
