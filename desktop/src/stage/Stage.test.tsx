import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { fixtureBridge } from "../api/fixture-bridge";
import { message, type PresentationLanguageTag } from "../i18n";
import { AnalyzePage } from "../modes/analyze";
import { CleanWorkbench } from "../modes/clean";
import { OptimizeWorkbench } from "../modes/optimize";
import { SoftwareWorkbench } from "../modes/software";
import { StatusWorkbench } from "../modes/status";
import { OperationCoordinator } from "../state/operation-coordinator";
import { DetailView } from "./DetailView";
import { Stage } from "./Stage";
import { StageResult } from "./StageResult";

const LOCALES: readonly PresentationLanguageTag[] = ["en", "zh-CN"];

function expectStageStack(label: string, title: string, action: string) {
  const stage = screen.getByRole("region", { name: label });
  expect(stage).toHaveClass("stage");
  expect(
    stage.querySelector(".planet[aria-hidden='true'] canvas"),
  ).toBeInTheDocument();
  expect(
    within(stage).getByRole("heading", { level: 2, name: title }),
  ).toBeInTheDocument();
  expect(
    within(stage).getByRole("button", { name: action }),
  ).toBeInTheDocument();
  expect(stage.querySelector(".card")).not.toBeInTheDocument();
}

describe("Stage", () => {
  it("renders the hero, title, meta, controls, primary, and secondary slots in order", () => {
    render(
      <Stage
        mode="clean"
        label="Stage"
        title="Title"
        meta={<p>Meta</p>}
        controls={<label>Scope</label>}
        primary={<button type="button">Go</button>}
        secondary={<button type="button">More facts</button>}
      />,
    );
    const stage = screen.getByRole("region", { name: "Stage" });
    const slots = [...stage.children].map((child) => child.className);
    expect(slots).toEqual([
      "stage-hero",
      "stage-title",
      "stage-meta",
      "stage-controls",
      "stage-primary",
      "stage-secondary",
    ]);
  });

  it("renders one number with unit, one line of facts, and one action", () => {
    render(
      <StageResult
        mode="status"
        label="Status"
        caption="CPU"
        value="43.21"
        unit="%"
        meta={<p>Memory line</p>}
        action={<button type="button">Start live</button>}
      />,
    );
    const heading = screen.getByRole("heading", { level: 2 });
    expect(heading).toHaveTextContent("CPU 43.21 %");
    expect(heading.querySelector(".stage-result-value")).toHaveTextContent(
      "43.21",
    );
    expect(heading.querySelector(".stage-result-unit")).toHaveTextContent("%");
    expect(screen.getByText("Memory line")).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/freed|released|释放/i);
  });

  it.each(LOCALES)(
    "labels the detail back control in %s and calls back",
    async (locale) => {
      const onBack = vi.fn();
      render(
        <DetailView locale={locale} onBack={onBack}>
          <p>Rows</p>
        </DetailView>,
      );
      await userEvent
        .setup()
        .click(
          screen.getByRole("button", {
            name: message(locale, "stage.v1.action.back"),
          }),
        );
      expect(onBack).toHaveBeenCalledOnce();
      expect(message("en", "stage.v1.action.back")).toBe("Back to overview");
      expect(message("zh-CN", "stage.v1.action.back")).toBe("返回概览");
    },
  );
});

describe.each(LOCALES)("mode first screens in %s", (locale) => {
  it("Clean shows the stage stack with scope and Scan", () => {
    render(
      <CleanWorkbench
        bridge={fixtureBridge}
        coordinator={new OperationCoordinator()}
        locale={locale}
      />,
    );
    expectStageStack(
      message(locale, "clean.v1.action.scan"),
      message(locale, "clean.v1.stage.headline"),
      message(locale, "clean.v1.action.scan"),
    );
    expect(
      screen.getByText(message(locale, "clean.v1.scope.projects")),
    ).toBeInTheDocument();
  });

  it("Software shows the stage stack with the inventory action", () => {
    render(
      <SoftwareWorkbench
        bridge={fixtureBridge}
        coordinator={new OperationCoordinator()}
        locale={locale}
      />,
    );
    expectStageStack(
      message(locale, "command.software"),
      message(locale, "software.v1.state.empty.title"),
      message(locale, "software.v1.action.inventory"),
    );
  });

  it("Optimize shows the stage stack with the catalogue action", () => {
    render(
      <OptimizeWorkbench
        bridge={fixtureBridge}
        coordinator={new OperationCoordinator()}
        locale={locale}
      />,
    );
    expectStageStack(
      message(locale, "command.optimize"),
      message(locale, "optimize.v1.state.empty.title"),
      message(locale, "optimize.v1.action.refresh"),
    );
  });

  it("Analyze shows the stage stack with the path control", () => {
    render(
      <AnalyzePage
        bridge={fixtureBridge}
        coordinator={new OperationCoordinator()}
        locale={locale}
      />,
    );
    expectStageStack(
      message(locale, "command.analyze"),
      message(locale, "analyze.v1.state.empty.title"),
      message(locale, "analyze.v1.action.start"),
    );
    expect(
      screen.getByRole("textbox", {
        name: message(locale, "analyze.v1.path.label"),
      }),
    ).toBeInTheDocument();
  });

  it("Status shows the CPU number, the memory line, and the live action", async () => {
    render(
      <StatusWorkbench
        bridge={fixtureBridge}
        coordinator={new OperationCoordinator()}
        locale={locale}
      />,
    );
    const heading = await screen.findByRole("heading", {
      level: 2,
      name: /43\.21/,
    });
    expect(heading).toHaveTextContent(message(locale, "status.v1.chart.cpu"));
    const stage = screen.getByRole("region", {
      name: message(locale, "command.status"),
    });
    expect(stage.querySelector(".planet canvas")).toBeInTheDocument();
    expect(
      within(stage).getByRole("button", {
        name: message(locale, "status.v1.action.live.start"),
      }),
    ).toBeInTheDocument();
    expect(
      within(stage).getByRole("button", {
        name: message(locale, "stage.v1.action.details"),
      }),
    ).toBeInTheDocument();
  });
});

describe("stage and detail switching", () => {
  it("opens detail after inventory, returns to the stage result, and reopens detail", async () => {
    const user = userEvent.setup();
    render(
      <SoftwareWorkbench
        bridge={fixtureBridge}
        coordinator={new OperationCoordinator()}
        locale="en"
      />,
    );
    await user.click(screen.getByRole("button", { name: "Refresh inventory" }));
    expect(
      (await screen.findAllByText("Contoso Tools")).length,
    ).toBeGreaterThan(0);
    expect(document.querySelector(".stage")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Back to overview" }));
    const stage = screen.getByRole("region", { name: "Software" });
    expect(
      within(stage).getByText("Software entries in the inventory"),
    ).toBeInTheDocument();
    expect(screen.queryByText("Contoso Tools")).not.toBeInTheDocument();
    await user.click(
      within(stage).getByRole("button", { name: "Show details" }),
    );
    expect(
      (await screen.findAllByText("Contoso Tools")).length,
    ).toBeGreaterThan(0);
  });
});
