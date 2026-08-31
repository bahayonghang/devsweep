import { describe, expect, it } from "vitest";
import { fixtureBridge } from "../api/fixture-bridge";
import { OperationCoordinator } from "../state/operation-coordinator";
import { MODE_IDS, SUPPORTING_DESTINATION_IDS } from "./AppShell";
import { shippedModeRegistrations, shippedSupportingRegistrations } from "./registry";

describe("shipped five-mode registry", () => {
  const input = {
    bridge: fixtureBridge,
    coordinator: new OperationCoordinator(),
    locale: "en" as const,
  };

  it("registers every accepted primary mode in frozen order and no placeholder", () => {
    const modes = shippedModeRegistrations(input);
    expect(modes.map((mode) => mode.id)).toEqual([...MODE_IDS]);
    expect(MODE_IDS).toEqual(["clean", "software", "optimize", "analyze", "status"]);
    expect(new Set(modes.map((mode) => mode.id)).size).toBe(5);
    for (const mode of modes) {
      expect(typeof mode.render).toBe("function");
    }
  });

  it("registers every accepted supporting destination and no sixth primary mode", () => {
    const supporting = shippedSupportingRegistrations(input);
    expect(supporting.map((destination) => destination.id)).toEqual([
      ...SUPPORTING_DESTINATION_IDS,
    ]);
    expect(SUPPORTING_DESTINATION_IDS).toEqual(["protection", "rules", "history"]);
    expect(supporting.some((destination) => MODE_IDS.includes(destination.id as typeof MODE_IDS[number]))).toBe(false);
  });
});
