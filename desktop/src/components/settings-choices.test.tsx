import { createEvent, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsCombobox } from "./SettingsCombobox";
import { SettingsSelect } from "./SettingsSelect";

const sizes = [{ value: 100, label: "100%" }, { value: 110, label: "110%" }, { value: 125, label: "125%" }];
const fonts = [
  { value: "system", label: "System UI" },
  { value: "yahei", label: "Microsoft YaHei UI", aliases: ["微软雅黑"] },
  { value: "long", label: "A long font family name with English and 中文字体名称", aliases: ["长字体"] },
];
const fontProps = { id: "font", label: "Font family", value: "system", options: fonts, placeholder: "Search fonts", emptyLabel: "No matching fonts." };

beforeEach(() => {
  // jsdom does not read FocusOptions. Base UI probes preventScroll before
  // restoring focus on outside dismissal. Simulate the WebView option contract.
  const focus = HTMLElement.prototype.focus;
  vi.spyOn(HTMLElement.prototype, "focus").mockImplementation(function (this: HTMLElement, options) {
    void options?.preventScroll;
    focus.call(this, options);
  });
});
afterEach(() => vi.restoreAllMocks());

describe("Settings Select", () => {
  it("labels its committed value and description and commits one pointer choice", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    const view = render(<SettingsSelect id="size" label="Text size" description="Scale the interface" value={100} options={sizes} onValueChange={choose} />);
    const trigger = screen.getByRole("combobox", { name: "Text size" });
    expect(trigger).toHaveTextContent("100%");
    expect(trigger).toHaveAccessibleDescription("Scale the interface");
    await user.click(trigger);
    expect(view.container).not.toContainElement(screen.getByRole("listbox"));
    expect(screen.getByRole("option", { name: "100%" })).toHaveAttribute("aria-selected", "true");
    await user.click(screen.getByRole("option", { name: "125%" }));
    expect(choose).toHaveBeenCalledExactlyOnceWith(125);
    expect(trigger).toHaveTextContent("100%");
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it("keeps keyboard highlight separate from selection and restores focus on Escape", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    render(<SettingsSelect id="size" label="Text size" value={100} options={sizes} onValueChange={choose} />);
    const trigger = screen.getByRole("combobox");
    trigger.focus();
    await user.keyboard("{ArrowDown}");
    await waitFor(() => expect(screen.getByRole("option", { name: "100%" })).toHaveFocus());
    await user.keyboard("{End}");
    expect(choose).not.toHaveBeenCalled();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
    expect(trigger).toHaveFocus();
    expect(trigger).toHaveTextContent("100%");
    await user.keyboard("{ArrowDown}");
    await waitFor(() => expect(screen.getByRole("option", { name: "100%" })).toHaveFocus());
    await user.keyboard("{End}");
    await waitFor(() => expect(screen.getByRole("option", { name: "125%" })).toHaveFocus());
    await user.keyboard("{Enter}");
    expect(choose).toHaveBeenCalledExactlyOnceWith(125);
  });

  it("supports typeahead without saving and ignores the current option", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    render(<SettingsSelect id="language" label="Language" value="en" options={[{ value: "en", label: "English" }, { value: "zh", label: "简体中文" }]} onValueChange={choose} />);
    await user.click(screen.getByRole("combobox"));
    // Base UI synchronizes open state to its parts after the pointer handler.
    // Wait for the portal option instead of assuming user.click flushed it.
    await user.click(await screen.findByRole("option", { name: "English" }));
    expect(choose).not.toHaveBeenCalled();
    const trigger = screen.getByRole("combobox");
    await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
    await user.click(trigger);
    await waitFor(() => expect(screen.getByRole("option", { name: "English" })).toHaveFocus());
    await user.keyboard("简");
    await waitFor(() => expect(screen.getByRole("option", { name: "简体中文" })).toHaveFocus());
    expect(choose).not.toHaveBeenCalled();
    await user.keyboard("{Enter}");
    expect(choose).toHaveBeenCalledExactlyOnceWith("zh");
  });

  it("dismisses outside without a write and keeps disabled controls closed", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    const view = render(<SettingsSelect id="size" label="Text size" value={100} options={sizes} onValueChange={choose} />);
    const trigger = screen.getByRole("combobox");
    await user.click(trigger);
    await user.click(document.body);
    await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
    expect(choose).not.toHaveBeenCalled();
    expect(trigger).toHaveFocus();
    view.rerender(<SettingsSelect id="size" label="Text size" value={100} options={sizes} onValueChange={choose} disabled busy />);
    expect(trigger).toBeDisabled();
    expect(trigger).toHaveAttribute("aria-busy", "true");
    await user.click(trigger);
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
  });
});

describe("Settings Combobox", () => {
  it.each(["", "{Backspace}"])("keeps keyboard search after replacing committed text with %s", async (clear) => {
    const user = userEvent.setup();
    const choose = vi.fn();
    render(<SettingsCombobox {...fontProps} onValueChange={choose} />);
    const input = screen.getByRole("combobox");
    await user.tab();
    expect(input).toHaveFocus();
    await user.keyboard(`{Control>}a{/Control}${clear}yahei`);
    expect(input).toHaveValue("yahei");
    expect(screen.getByRole("option", { name: "Microsoft YaHei UI" })).toBeInTheDocument();
    expect(choose).not.toHaveBeenCalled();
  });

  it("filters aliases and full long labels without committing query or highlight", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    const view = render(<SettingsCombobox {...fontProps} onValueChange={choose} />);
    const input = screen.getByRole("combobox", { name: "Font family" });
    expect(input).toHaveValue("System UI");
    await user.click(input);
    await user.type(input, "长字体");
    expect(screen.getByRole("option", { name: fonts[2].label })).toBeInTheDocument();
    expect(view.container).not.toContainElement(screen.getByRole("listbox"));
    await user.keyboard("{ArrowDown}");
    await user.hover(screen.getByRole("option", { name: fonts[2].label }));
    expect(choose).not.toHaveBeenCalled();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
    expect(input).toHaveFocus();
    expect(input).toHaveValue("System UI");
  });

  it("commits one explicit choice and displays only the committed value", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    const view = render(<SettingsCombobox {...fontProps} onValueChange={choose} />);
    const input = screen.getByRole("combobox");
    await user.click(input);
    await user.type(input, "yahei");
    await user.keyboard("{ArrowDown}{Enter}");
    expect(choose).toHaveBeenCalledExactlyOnceWith("yahei");
    expect(input).toHaveValue("System UI");
    view.rerender(<SettingsCombobox {...fontProps} value="yahei" onValueChange={choose} />);
    expect(input).toHaveValue("Microsoft YaHei UI");
    await user.click(input);
    await user.click(screen.getByRole("option", { name: "Microsoft YaHei UI" }));
    expect(choose).toHaveBeenCalledTimes(1);
  });

  it("preserves native composition navigation, Enter, and dismissal without saving", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    render(<SettingsCombobox {...fontProps} value="long" onValueChange={choose} />);
    const input = screen.getByRole("combobox");
    await user.click(input);
    fireEvent.compositionStart(input);
    fireEvent.compositionUpdate(input, { data: "微软雅黑" });
    fireEvent.input(input, { target: { value: "微软雅黑" }, data: "微软雅黑", inputType: "insertCompositionText", isComposing: true });
    await screen.findByRole("listbox");
    const arrow = createEvent.keyDown(input, { key: "ArrowDown", code: "ArrowDown", isComposing: true, keyCode: 40, which: 40 });
    const enter = createEvent.keyDown(input, { key: "Enter", code: "Enter", isComposing: true, keyCode: 13, which: 13 });
    fireEvent(input, arrow);
    fireEvent(input, enter);
    expect(choose).not.toHaveBeenCalled();
    expect(arrow.defaultPrevented).toBe(false);
    expect(enter.defaultPrevented).toBe(false);
    expect(input).toHaveValue("微软雅黑");
    expect(input).toHaveFocus();
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    fireEvent.compositionEnd(input, { data: "微软雅黑" });
    await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(1));
    expect(screen.getByRole("option", { name: "Microsoft YaHei UI" })).toBeInTheDocument();
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
    expect(input).toHaveValue(fonts[2].label);
    expect(input).toHaveFocus();
    expect(choose).not.toHaveBeenCalled();
  });

  it.each([
    { label: "native isComposing", isComposing: true, keyCode: 13 },
    { label: "legacy keyCode 229", isComposing: false, keyCode: 229 },
  ])("does not select an already highlighted font for $label", async ({ isComposing, keyCode }) => {
    const user = userEvent.setup();
    const choose = vi.fn();
    render(<SettingsCombobox {...fontProps} onValueChange={choose} />);
    const input = screen.getByRole("combobox");
    await user.click(input);
    await user.type(input, "yahei");
    const option = await screen.findByRole("option", { name: "Microsoft YaHei UI" });
    await user.keyboard("{ArrowDown}");
    await waitFor(() => expect(input).toHaveAttribute("aria-activedescendant", option.id));
    const enter = createEvent.keyDown(input, { key: "Enter", code: "Enter", isComposing, keyCode, which: keyCode });
    fireEvent(input, enter);
    expect(choose).not.toHaveBeenCalled();
    expect(enter.defaultPrevented).toBe(false);
    expect(input).toHaveFocus();
    expect(screen.getByRole("listbox")).toBeInTheDocument();
    await user.keyboard("{Enter}");
    expect(choose).toHaveBeenCalledExactlyOnceWith("yahei");
  });

  it("keeps explicit pointer selection available during composition", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    render(<SettingsCombobox {...fontProps} onValueChange={choose} />);
    const input = screen.getByRole("combobox");
    await user.click(input);
    fireEvent.compositionStart(input);
    fireEvent.input(input, { target: { value: "微软雅黑" }, inputType: "insertCompositionText", isComposing: true });
    await user.click(await screen.findByRole("option", { name: "Microsoft YaHei UI" }));
    expect(choose).toHaveBeenCalledExactlyOnceWith("yahei");
  });

  it("announces empty and catalogue states without disabling available choices", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    const view = render(<SettingsCombobox {...fontProps} onValueChange={choose} status={{ kind: "loading", message: "Loading fonts" }} />);
    const input = screen.getByRole("combobox");
    expect(input).toHaveAttribute("aria-busy", "true");
    expect(input).toHaveAccessibleDescription("Loading fonts");
    expect(input).not.toBeDisabled();
    await user.click(input);
    await user.type(input, "unmatched");
    expect(screen.getByText("No matching fonts.")).toBeVisible();
    await user.click(document.body);
    await waitFor(() => expect(input).toHaveValue("System UI"));
    expect(choose).not.toHaveBeenCalled();
    view.rerender(<SettingsCombobox {...fontProps} onValueChange={choose} status={{ kind: "unavailable", message: "Font list unavailable" }} />);
    expect(screen.getByRole("status")).toHaveTextContent("Font list unavailable");
    expect(input).toHaveAttribute("aria-busy", "false");
  });

  it("preserves a missing committed name and disables only its own input", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    render(<SettingsCombobox {...fontProps} value="Unavailable saved family 中文" disabled onValueChange={choose} />);
    const input = screen.getByRole("combobox");
    expect(input).toHaveValue("Unavailable saved family 中文");
    expect(input).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Font family" }));
    expect(screen.queryByRole("listbox")).not.toBeInTheDocument();
    expect(choose).not.toHaveBeenCalled();
  });

  it("lets Tab reach the next control without committing a highlighted font", async () => {
    const user = userEvent.setup();
    const choose = vi.fn();
    render(<><SettingsCombobox {...fontProps} onValueChange={choose} /><button>Next setting</button></>);
    await user.click(screen.getByRole("combobox"));
    await user.keyboard("{ArrowDown}{Tab}");
    await waitFor(() => expect(screen.queryByRole("listbox")).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Next setting" })).toHaveFocus();
    expect(screen.getByRole("combobox")).toHaveValue("System UI");
    expect(choose).not.toHaveBeenCalled();
  });
});
