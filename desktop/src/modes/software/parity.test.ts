import { describe, expect, it } from "vitest";
import inventoryJson from "../../api/fixtures/software/inventory.json";
import allMsiJson from "../../api/fixtures/software/all-msi-manual.json";
import executionJson from "../../api/fixtures/software/execution-five-terminal.json";
import { decodeDesktopSoftwareUninstallResult, decodeSoftwareInventory } from "../../api/contract";

describe("software cross-surface fixtures", () => {
  it("covers every ordered refusal and strict last-used unknown", () => {
    const inventory = decodeSoftwareInventory(inventoryJson);
    expect(new Set(inventory.entries.map((entry) => entry.eligibility.reason))).toEqual(new Set([
      "protected_product", "source_incomplete", "conflicting_identity", "no_remove", "hidden_entry",
      "system_or_update", "dependency_package", "stub_package", "unhealthy_package",
      "msi_execution_not_supported_v1", "registry_only_manual", "unsupported_source", "eligible_current_user_msix",
    ]));
    expect(inventory.entries.every((entry) => entry.last_used.state === "unknown"
      && entry.last_used.reason_code === "no_supported_exact_source")).toBe(true);
    expect(JSON.stringify(inventory)).not.toMatch(/UninstallString|QuietUninstallString|"installed_path"\s*:|"argv"\s*:/i);
  });

  it("keeps every MSI context manual and execution closed to five post-dispatch terminals", () => {
    const msi = decodeSoftwareInventory(allMsiJson);
    expect(msi.entries.map((entry) => entry.eligibility.reason)).toEqual([
      "msi_execution_not_supported_v1", "msi_execution_not_supported_v1", "msi_execution_not_supported_v1",
    ]);
    const report = decodeDesktopSoftwareUninstallResult(executionJson).report;
    expect(new Set(report.outcomes.map((item) => item.outcome))).toEqual(new Set([
      "removed", "reboot_required", "still_present", "failed", "unknown_after_dispatch",
    ]));
    expect(JSON.stringify(report)).not.toContain("partial");
  });
});
