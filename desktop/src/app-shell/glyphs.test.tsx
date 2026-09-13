import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DestinationGlyph, GLYPH_NAMES, GLYPHS } from "./glyphs";

describe("DevSweep glyphs", () => {
  it("renders each export as an aria-hidden 16x16 svg", () => {
    expect(Object.keys(GLYPHS)).toEqual([...GLYPH_NAMES]);
    for (const name of GLYPH_NAMES) {
      const view = render(<DestinationGlyph name={name} />);
      const svg = view.container.querySelector("svg");
      expect(svg).not.toBeNull();
      expect(svg).toHaveAttribute("aria-hidden", "true");
      expect(svg).toHaveAttribute("viewBox", "0 0 16 16");
      expect(svg).toHaveAttribute("fill", "none");
      expect(svg).toHaveAttribute("stroke", "currentColor");
      view.unmount();
    }
  });
});
