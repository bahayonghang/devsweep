import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import fixture from "../api/fixtures/scan-progress.json";
import { decodeDesktopScanProgress } from "../api/contract";
import { ScanPreviewPage } from "./ScanPreviewPage";

describe("large read-only scan preview", () => {
  it("searches the full 10k observation and exposes the last full path without selection", async () => {
    const user = userEvent.setup();
    const progress = decodeDesktopScanProgress({ ...fixture, preview: {
      targets: Array.from({ length: 10_000 }, (_, index) => ({ ...fixture.preview.targets[0],
        id: `target-${index}`, path: `C:/fixture/目录 with spaces ${index}/node_modules`,
      })), totals: { ...fixture.preview.totals, target_count: 10_000, verified_bytes: 4096 * 10_000 },
    } });
    render(<ScanPreviewPage status="active" progress={progress} preview={progress.preview} canReturnToReport={false} onReturnToReport={() => undefined} />);
    expect(document.querySelectorAll("tbody tr")).toHaveLength(100);
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Last page" }));
    const path = "C:/fixture/目录 with spaces 9999/node_modules";
    expect(screen.getByText(path)).toHaveAttribute("aria-label", path);
    expect(screen.getByText(path)).toHaveAttribute("title", path);
    await user.type(screen.getByRole("searchbox", { name: "Search" }), "目录 with spaces 9999");
    expect(document.querySelectorAll("tbody tr")).toHaveLength(1);
    expect(screen.getByText(path)).toBeInTheDocument();
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();
    expect(progress.preview?.targets).toHaveLength(10_000);
    await user.clear(screen.getByRole("searchbox", { name: "Search" }));
    expect(screen.getByText("Page 1 of 100; targets 1–100 of 10000.")).toBeInTheDocument();
  });
});
