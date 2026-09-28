import { useMemo } from "react";
import type { DesktopFont } from "../api/types.gen";
import { SettingsCombobox } from "../components/SettingsCombobox";
import { message, type PresentationLanguageTag } from "../i18n";
import { useFontCatalogue } from "./fonts";

const SYSTEM_VALUE = "system:";
const installedValue = (family: string) => `installed:${family}`;

export function FontPicker({ locale, font, disabled, onChange }: {
  locale: PresentationLanguageTag; font: DesktopFont; disabled: boolean; onChange: (font: DesktopFont) => void;
}) {
  const catalogue = useFontCatalogue();
  const systemLabel = message(locale, "preferences.v1.font.system");
  const options = useMemo(() => [
    { value: SYSTEM_VALUE, label: systemLabel },
    ...(catalogue.families ?? []).map(({ family, names }) => {
      const local = names.find((name) => name.locale.toLowerCase() === locale.toLowerCase())?.name;
      return { value: installedValue(family), label: local && local !== family ? `${local} (${family})` : family, aliases: [family, ...names.map((name) => name.name)] };
    }),
  ], [catalogue.families, locale, systemLabel]);
  const missing = font.kind === "installed" && catalogue.families !== null && !catalogue.families.some(({ family }) => family.toLowerCase() === font.family.toLowerCase());
  const status = catalogue.loading ? { kind: "loading" as const, message: message(locale, "preferences.v2.font.loading") }
    : catalogue.failure ? { kind: "unavailable" as const, message: message(locale, catalogue.failure === "unsupported_platform" ? "preferences.v2.font.unsupported" : "preferences.v2.font.failed") } : undefined;
  return <div className="settings-font-picker">
    <SettingsCombobox id="settings-font" label={message(locale, "preferences.v1.font")} disabled={disabled}
      value={font.kind === "system" ? SYSTEM_VALUE : installedValue(font.family)} valueLabel={font.kind === "system" ? systemLabel : font.family}
      options={options} status={status} placeholder={message(locale, "preferences.v1.font.search")} emptyLabel={message(locale, "preferences.v1.font.empty")}
      onValueChange={(value) => {
        if (value === SYSTEM_VALUE) { onChange({ kind: "system" }); return; }
        const selected = catalogue.families?.find(({ family }) => installedValue(family) === value);
        if (selected) onChange({ kind: "installed", family: selected.family });
      }} />
    <div className="settings-font-actions">
      <button type="button" className="secondary-button" disabled={catalogue.loading} onClick={() => { void catalogue.refresh(); }}>{message(locale, "preferences.v2.font.refresh")}</button>
      {missing && <small role="status">{message(locale, "preferences.v2.font.missing", { family: font.family })}</small>}
      {catalogue.families?.length === 0 && <small role="status">{message(locale, "preferences.v2.font.no_fonts")}</small>}
    </div>
  </div>;
}
