import { Select } from "@base-ui/react/select";

export interface SettingsOption<Value extends string | number> {
  value: Value;
  label: string;
  aliases?: readonly string[];
}

export interface SettingsChoiceProps<Value extends string | number> {
  id: string;
  label: string;
  description?: string;
  value: Value;
  options: readonly SettingsOption<Value>[];
  disabled?: boolean;
  busy?: boolean;
  onValueChange: (value: Value) => void;
}

export function SettingsSelect<Value extends string | number>({
  id, label, description, value, options, disabled, busy, onValueChange,
}: SettingsChoiceProps<Value>) {
  return <div className="settings-row">
    <div className="settings-row-copy">
      <label id={`${id}-label`} htmlFor={id}>{label}</label>
      {description && <small id={`${id}-description`}>{description}</small>}
    </div>
    <Select.Root<Value> value={value} items={options} disabled={disabled} modal={false}
      onValueChange={(next, details) => {
        if (details.reason === "item-press" && next !== null && next !== value) onValueChange(next);
      }}>
      <Select.Trigger id={id} className="settings-choice" aria-labelledby={`${id}-label`}
        aria-describedby={description ? `${id}-description` : undefined} aria-busy={busy}>
        <Select.Value className="settings-choice-value" />
        <Select.Icon className="settings-choice-icon" aria-hidden>⌄</Select.Icon>
      </Select.Trigger>
      <Select.Portal>
        <Select.Positioner className="settings-choice-positioner" align="start" sideOffset={6}
          collisionPadding={8} alignItemWithTrigger={false}>
          <Select.Popup className="settings-choice-popup">
            <Select.List className="settings-choice-list" aria-labelledby={`${id}-label`}>
              {options.map((option) => <Select.Item key={option.value} value={option.value} className="settings-choice-item">
                <Select.ItemIndicator className="settings-choice-check" aria-hidden>✓</Select.ItemIndicator>
                <Select.ItemText>{option.label}</Select.ItemText>
              </Select.Item>)}
            </Select.List>
          </Select.Popup>
        </Select.Positioner>
      </Select.Portal>
    </Select.Root>
  </div>;
}
