import { describe, expect, it } from "vitest";
import fixture from "../../api/fixtures/scan-report.json";
import dryRunFixture from "../../api/fixtures/dry-run-outcome.json";
import { decodeDryRunOutcome, decodeScanReport } from "../../api/contract";
import { allExecutableSelected } from "../../state/selectors";
import { cleanReducer, initialCleanState, type CleanState } from "./reducer";

const report = decodeScanReport(fixture);
const dryRun = decodeDryRunOutcome(dryRunFixture);
const cargo = "cargo.target:C:/work/app/target";
const npm = "npm.cache.clean:global";
const inspect = "cargo.home.inspect:C:/Users/dev/.cargo";

function reviewed(): CleanState {
  const scanning = cleanReducer(initialCleanState, {
    type: "scan_requested",
    scanId: "s1",
  });
  return cleanReducer(scanning, {
    type: "scan_completed",
    scanId: "s1",
    report,
  });
}

function withDryRun(): CleanState {
  const requested = cleanReducer(reviewed(), { type: "dry_run_requested" });
  return cleanReducer(requested, {
    type: "dry_run_succeeded",
    outcome: dryRun,
  });
}

describe("row skip", () => {
  it("removes the id from the selection, invalidates the dry run, and restore re-adds it", () => {
    let state = withDryRun();
    expect(state.dryRun?.digest).toBe(dryRun.digest);
    const plan = state.scan?.plan;

    state = cleanReducer(state, { type: "target_skipped", targetId: cargo });
    expect(state.selectedIds.has(cargo)).toBe(false);
    expect(state.skippedIds.has(cargo)).toBe(true);
    expect(state.dryRun).toBeNull();
    expect(state.phase).toBe("reviewed");
    expect(state.scan?.plan).toBe(plan);

    expect(
      cleanReducer(state, {
        type: "selection_changed",
        targetId: cargo,
        selected: true,
      }).selectedIds.has(cargo),
    ).toBe(false);
    expect(
      cleanReducer(state, {
        type: "select_all_changed",
        selected: true,
      }).selectedIds.has(cargo),
    ).toBe(false);

    state = cleanReducer(state, { type: "target_restored", targetId: cargo });
    expect(state.skippedIds.has(cargo)).toBe(false);
    expect(state.selectedIds.has(cargo)).toBe(true);
    expect(state.dryRun).toBeNull();
  });

  it("ignores inspect-only rows and never adds a target to the selection", () => {
    const state = reviewed();
    expect(
      cleanReducer(state, { type: "target_skipped", targetId: inspect }),
    ).toBe(state);
    const skippedUnselected = cleanReducer(state, {
      type: "target_skipped",
      targetId: npm,
    });
    expect(skippedUnselected.selectedIds.has(npm)).toBe(false);
    expect(
      cleanReducer(state, { type: "target_restored", targetId: npm }),
    ).toBe(state);
  });

  it("is ignored while a scan preview is active", () => {
    const scanning = cleanReducer(reviewed(), {
      type: "scan_requested",
      scanId: "s2",
    });
    expect(
      cleanReducer(scanning, { type: "target_skipped", targetId: cargo }),
    ).toBe(scanning);
  });
});

describe("row protect", () => {
  it("marks the row inspect-only and invalidates selection and dry run", () => {
    let state = withDryRun();
    state = cleanReducer(state, { type: "target_protected", targetId: cargo });
    expect(state.protectedIds.has(cargo)).toBe(true);
    expect(state.selectedIds.has(cargo)).toBe(false);
    expect(state.dryRun).toBeNull();
    expect(state.phase).toBe("reviewed");
    expect(
      cleanReducer(state, {
        type: "selection_changed",
        targetId: cargo,
        selected: true,
      }).selectedIds.has(cargo),
    ).toBe(false);
    expect(
      cleanReducer(state, { type: "target_skipped", targetId: cargo }),
    ).toBe(state);
    const all = cleanReducer(state, {
      type: "select_all_changed",
      selected: true,
    });
    expect([...all.selectedIds]).toEqual([npm]);
    expect(allExecutableSelected(all)).toBe(true);
  });

  it("clears protection only when a new completed report replaces the plan", () => {
    let state = cleanReducer(reviewed(), {
      type: "target_protected",
      targetId: cargo,
    });
    state = cleanReducer(state, { type: "scan_requested", scanId: "s2" });
    state = cleanReducer(state, {
      type: "scan_failed",
      scanId: "s2",
      error: { code: "scan_failed", message: "controlled" },
    });
    state = cleanReducer(state, { type: "stopped_preview_dismissed" });
    expect(state.protectedIds.has(cargo)).toBe(true);
    expect(state.selectedIds.has(cargo)).toBe(false);

    state = cleanReducer(state, { type: "scan_requested", scanId: "s3" });
    state = cleanReducer(state, {
      type: "scan_completed",
      scanId: "s3",
      report,
    });
    expect(state.protectedIds.size).toBe(0);
    expect(state.selectedIds.has(cargo)).toBe(true);
  });

  it("does not change the selection while execution runs", () => {
    let state = cleanReducer(withDryRun(), { type: "confirmation_opened" });
    state = cleanReducer(state, { type: "execute_requested" });
    const next = cleanReducer(state, {
      type: "target_protected",
      targetId: cargo,
    });
    expect(next.phase).toBe("executing");
    expect(next.selectedIds).toBe(state.selectedIds);
    expect(next.dryRun).toBe(state.dryRun);
    expect(next.protectedIds.has(cargo)).toBe(true);
  });

  it("drops the id and the pending dry run so a late dry-run result is ignored", () => {
    let state = cleanReducer(reviewed(), { type: "dry_run_requested" });
    state = cleanReducer(state, { type: "target_protected", targetId: cargo });
    expect(state.pending).toBeNull();
    expect(state.selectedIds.has(cargo)).toBe(false);
    const late = cleanReducer(state, {
      type: "dry_run_succeeded",
      outcome: dryRun,
    });
    expect(late).toBe(state);
    expect(late.dryRun).toBeNull();
  });
});
