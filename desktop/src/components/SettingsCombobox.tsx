import { useRef, useState } from "react";
import { Combobox } from "@base-ui/react/combobox";
import type { SettingsChoiceProps, SettingsOption } from "./SettingsSelect";

export interface SettingsComboboxProps<Value extends string | number> extends SettingsChoiceProps<Value> {
  placeholder: string;
  emptyLabel: string;
  valueLabel?: string;
  status?: { kind: "loading" | "unavailable"; message: string };
}

export function SettingsCombobox<Value extends string | number>({
  id, label, description, value, options, disabled, busy, onValueChange, placeholder, emptyLabel, status, valueLabel,
}: SettingsComboboxProps<Value>) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const anchor = useRef<HTMLDivElement>(null);
  const committed = options.find((option) => option.value === value) ?? { value, label: valueLabel ?? String(value) };
  const describedBy = [description && `${id}-description`, status && `${id}-status`].filter(Boolean).join(" ") || undefined;

  return <div className="settings-row">
    <div className="settings-row-copy">
      <label id={`${id}-label`} htmlFor={id}>{label}</label>
      {description && <small id={`${id}-description`}>{description}</small>}
    </div>
    <div className="settings-choice-field">
      <Combobox.Root<SettingsOption<Value>> items={options} value={committed} open={open}
        disabled={disabled} inputValue={open ? query : committed.label}
        isItemEqualToValue={(a, b) => a.value === b.value}
        filter={(option, search) => [option.label, ...(option.aliases ?? [])].some((name) => name.toLocaleLowerCase().includes(search.toLocaleLowerCase()))}
        onOpenChange={(next, details) => {
          setOpen(next);
          if (!next || details.reason !== "input-change") setQuery("");
        }}
        onInputValueChange={(next, details) => {
          if (details.reason === "input-change") {
            setQuery(next);
            setOpen(true);
          }
        }}
        onValueChange={(next, details) => {
          if (details.reason === "item-press" && next !== null && next.value !== value) onValueChange(next.value);
        }}>
        <div ref={anchor} className="settings-choice settings-combobox" data-disabled={disabled || undefined}>
          <Combobox.Input id={id} aria-labelledby={`${id}-label`} aria-describedby={describedBy}
            aria-busy={busy || status?.kind === "loading"} placeholder={placeholder}
            onKeyDown={(event) => {
              if (event.nativeEvent.isComposing || event.nativeEvent.keyCode === 229) {
                // Preserve the IME default action and skip Base UI selection.
                event.preventBaseUIHandler();
              }
            }} />
          <Combobox.Trigger className="settings-combobox-trigger" aria-label={label}>
            <span className="settings-choice-icon" aria-hidden>⌄</span>
          </Combobox.Trigger>
        </div>
        <Combobox.Portal>
          <Combobox.Positioner className="settings-choice-positioner" anchor={anchor} align="start" sideOffset={6} collisionPadding={8}>
            <Combobox.Popup className="settings-choice-popup">
              <Combobox.Empty className="settings-choice-empty">{emptyLabel}</Combobox.Empty>
              <Combobox.List className="settings-choice-list" aria-labelledby={`${id}-label`}>
                {(option: SettingsOption<Value>) => <Combobox.Item key={option.value} value={option} className="settings-choice-item">
                  <Combobox.ItemIndicator className="settings-choice-check" aria-hidden>✓</Combobox.ItemIndicator>
                  <span>{option.label}</span>
                </Combobox.Item>}
              </Combobox.List>
            </Combobox.Popup>
          </Combobox.Positioner>
        </Combobox.Portal>
      </Combobox.Root>
      {status && <small id={`${id}-status`} role="status">{status.message}</small>}
    </div>
  </div>;
}
