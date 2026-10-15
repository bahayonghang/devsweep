import { act, renderHook } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { fixtureBridge } from "../../api/fixture-bridge";
import { decodeScanReport } from "../../api/contract";
import fixture from "../../api/fixtures/scan-report.json";
import type { CleanAdvisoriesV1 } from "../../api/types.gen";
import { useCleanAdvisory } from "./useCleanAdvisory";

it("ignores out-of-order threshold projections and hides old report assessments immediately", async () => {
  const report = decodeScanReport(fixture), newerReport = { ...report };
  const resolvers: ((value: CleanAdvisoriesV1) => void)[] = [];
  const cleanAdvisories = vi.fn(() => new Promise<CleanAdvisoriesV1>(resolve => resolvers.push(resolve)));
  const bridge = { ...fixtureBridge, cleanAdvisories };
  const { result, rerender } = renderHook(({ report, threshold }) => useCleanAdvisory(bridge, report, threshold), { initialProps: { report, threshold: 5 } });
  rerender({ report, threshold: 8 });
  const value: CleanAdvisoriesV1 = { version: 1, report_id: "sha256:" + "a".repeat(64), threshold_bytes: 8 * 1024 ** 3, included_kinds: [], projects: [] };
  await act(async () => resolvers[1](value));
  expect(result.current?.value).toBe(value);
  await act(async () => resolvers[0]({ ...value, threshold_bytes: 5 * 1024 ** 3 }));
  expect(result.current?.value).toBe(value);
  rerender({ report: newerReport, threshold: 8 });
  expect(result.current).toBeNull();
  expect(cleanAdvisories).toHaveBeenLastCalledWith(newerReport, 8);
});
