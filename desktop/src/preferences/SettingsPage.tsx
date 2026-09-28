import type { DesktopMotion, DesktopPreferencesPatch } from "../api/types.gen";
import { FontPicker } from "./FontPicker";
import { SettingsSelect, type SettingsOption } from "../components/SettingsSelect";
import { message, type MessageKey, type PresentationLanguageTag } from "../i18n";
import { THEME_CATALOGUE, useMediaPreference } from "./appearance";
import { usePreferences } from "./context";

export function PreferencesNotice({ locale }: { locale: PresentationLanguageTag }) {
  const store = usePreferences();
  if (!store.unavailable) return null;
  return <div className="preferences-warning" role="alert">
    <span>{message(locale, "preferences.v1.unavailable")}</span>
    <button type="button" onClick={store.reload}>{message(locale, "preferences.v1.reload")}</button>
  </div>;
}

export function SettingsPage({ locale, onLocaleChange, localeSaving }: {
  locale: PresentationLanguageTag;
  onLocaleChange: (locale: PresentationLanguageTag) => void | Promise<void>;
  localeSaving: boolean;
}) {
  const store = usePreferences();
  const p = store.preferences;
  const osReduced = useMediaPreference("(prefers-reduced-motion: reduce)");
  const reduced = osReduced || p.motion === "reduced";
  const disabled = store.loading || store.unavailable || store.saving;
  const t = (key: MessageKey) => message(locale, key);
  const save = (patch: DesktopPreferencesPatch) => { void store.update(patch); };
  const numberOptions = (values: readonly number[], unit: "percent" | "seconds" | "fps" | "rows"): SettingsOption<number>[] => values.map((value) =>
    ({ value, label: message(locale, (`preferences.v1.unit.${unit}`) as MessageKey, { value: String(value) }) }));
  const themes = Object.values(THEME_CATALOGUE);

  return <div className="settings-page">
    <p className="settings-save-state" role="status" aria-live="polite">{t(store.saving ? "preferences.v1.saving" : "preferences.v1.commit.note")}</p>
    {store.saveError && <p className="preferences-warning" role="alert">{t("preferences.v1.save.error")}</p>}
    <section className="settings-group" aria-labelledby="settings-appearance">
      <div className="settings-group-heading"><h2 id="settings-appearance">{t("preferences.v1.appearance")}</h2>
        <button type="button" className="secondary-button" disabled={disabled} onClick={() => save({ field: "reset_appearance" })}>{t("preferences.v1.reset.appearance")}</button></div>
      <p>{t("preferences.v1.appearance.note")}</p>
      <fieldset className="settings-themes" disabled={disabled}>
        <legend>{t("preferences.v1.theme")}</legend>
        <div className="settings-theme-grid">{themes.map((theme) => <label key={theme.id} className="settings-theme">
          <input type="radio" name="settings-theme" value={theme.id} checked={p.theme === theme.id}
            onChange={() => { if (p.theme !== theme.id) save({ field: "theme", value: theme.id }); }} />
          <span className="settings-theme-preview" aria-hidden>{theme.preview.map((palette) =>
            <span key={palette} className="settings-theme-sample" data-palette-preview={palette}><span className="settings-theme-capsule" /><span className="settings-theme-lines"><i /><i /></span></span>)}</span>
          <span className="settings-theme-caption"><span>{t(theme.label)}</span><span className="settings-theme-check" aria-hidden>✓</span></span>
        </label>)}</div>
      </fieldset>
      <FontPicker locale={locale} font={p.font} disabled={disabled} onChange={(value) => save({ field: "font", value })} />
      <SettingsSelect id="settings-text-scale" label={t("preferences.v1.scale")} value={p.text_scale_percent} disabled={disabled}
        options={numberOptions([100, 110, 125], "percent")} onValueChange={(value) => save({ field: "text_scale_percent", value })} />
      <div className="settings-font-preview" aria-label={t("preferences.v1.preview")}><span lang="en">DevSweep · Clear, readable text</span><span lang="zh-CN">开发环境 · 清晰可读</span><span className="tabular">0123456789 · 128 GiB</span></div>
    </section>
    <section className="settings-group" aria-labelledby="settings-language-heading">
      <h2 id="settings-language-heading">{t("preferences.v1.language")}</h2>
      <SettingsSelect<PresentationLanguageTag> id="settings-language" label={t("preferences.v1.language")} value={locale} disabled={localeSaving} busy={localeSaving}
        options={[{ value: "en", label: t("shell.v1.settings.option.en") }, { value: "zh-CN", label: t("shell.v1.settings.option.zh_cn") }]} onValueChange={(value) => { void onLocaleChange(value); }} />
    </section>
    <section className="settings-group" aria-labelledby="settings-performance">
      <div className="settings-group-heading"><h2 id="settings-performance">{t("preferences.v1.performance")}</h2>
        <button type="button" className="secondary-button" disabled={disabled} onClick={() => save({ field: "reset_performance" })}>{t("preferences.v1.reset.performance")}</button></div>
      <SettingsSelect<DesktopMotion> id="settings-motion" label={t("preferences.v1.motion")} description={t("preferences.v1.motion.note")} value={p.motion} disabled={disabled}
        options={[{ value: "system", label: t("preferences.v1.motion.system") }, { value: "reduced", label: t("preferences.v1.motion.reduced") }]} onValueChange={(value) => save({ field: "motion", value })} />
      <SettingsSelect id="settings-planet-fps" label={t("preferences.v1.fps")} description={t(reduced ? "preferences.v1.fps.disabled" : "preferences.v1.fps.note")} value={p.planet_fps} disabled={disabled || reduced}
        options={numberOptions([15, 30], "fps")} onValueChange={(value) => save({ field: "planet_fps", value })} />
      <SettingsSelect id="settings-status-interval" label={t("preferences.v1.status.interval")} description={t("preferences.v1.status.interval.note")} value={p.status_interval_seconds} disabled={disabled}
        options={numberOptions([1, 2, 5, 10, 30, 60], "seconds")} onValueChange={(value) => save({ field: "status_interval_seconds", value })} />
      <SettingsSelect id="settings-status-rows" label={t("preferences.v1.status.rows")} description={t("preferences.v1.status.rows.note")} value={p.status_process_limit} disabled={disabled}
        options={numberOptions([5, 15, 30, 50, 100], "rows")} onValueChange={(value) => save({ field: "status_process_limit", value })} />
      <SettingsSelect id="settings-hud-interval" label={t("preferences.v1.hud.interval")} description={t("preferences.v1.hud.interval.note")} value={p.hud_interval_seconds} disabled={disabled}
        options={numberOptions([2, 5, 10], "seconds")} onValueChange={(value) => save({ field: "hud_interval_seconds", value })} />
    </section>
  </div>;
}
