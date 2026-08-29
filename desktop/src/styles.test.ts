import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const styles = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");

describe("responsive scan workbench styles", () => {
  it("contains table overflow and an explicit narrow layout", () => {
    expect(styles).toContain(".table-frame { overflow: auto;");
    expect(styles).toContain("@media (max-width: 760px)");
    expect(styles).toContain("grid-template-columns: minmax(0, 1fr) 118px");
  });

  it("honors reduced motion without removing semantic progress", () => {
    expect(styles).toContain("@media (prefers-reduced-motion: reduce)");
    expect(styles).toContain("animation-duration: .01ms !important");
    expect(styles).not.toContain("@keyframes");
  });
});
