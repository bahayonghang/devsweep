import type { DesktopFontFamily, DesktopMotion, DesktopPreferencesPatch, DesktopTheme } from "../api/types.gen";
import { message, type MessageKey, type PresentationLanguageTag } from "../i18n";
import { useMediaPreference } from "./appearance";
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
  const option = (value: string, key: MessageKey) => <option value={value} key={value}>{t(key)}</option>;
  const numberOptions = (values: readonly number[], unit: "percent" | "seconds" | "fps" | "rows") => values.map((value) =>
    <option key={value} value={value}>{message(locale, ('preferences.v1.unit.' + unit) as MessageKey, { value: String(value) })}</option>);

  return <div className="settings-page">
    <p className="settings-save-state" role="status" aria-live="polite">{t(store.saving ? "preferences.v1.saving" : "preferences.v1.commit.note")}</p>
    {store.saveError && <p className="preferences-warning" role="alert">{t("preferences.v1.save.error")}</p>}
    <section className="card settings-group" aria-labelledby="settings-appearance">
      <div className="settings-group-heading"><h2 id="settings-appearance">{t("preferences.v1.appearance")}</h2>
        <button type="button" className="secondary-button" disabled={disabled} onClick={() => save({ field: "reset_appearance" })}>{t("preferences.v1.reset.appearance")}</button></div>
      <p>{t("preferences.v1.appearance.note")}</p>
      <div className="settings-fields">
        <label><span>{t("preferences.v1.theme")}</span><select id="settings-theme" value={p.theme} disabled={disabled} onChange={(e) => save({ field: "theme", value: e.target.value as DesktopTheme })}>
          {option("dark", "preferences.v1.theme.dark")}{option("light", "preferences.v1.theme.light")}{option("system", "preferences.v1.theme.system")}
        </select></label>
        <label><span>{t("preferences.v1.font")}</span><select id="settings-font" value={p.font_family} disabled={disabled} onChange={(e) => save({ field: "font_family", value: e.target.value as DesktopFontFamily })}>
          {option("system", "preferences.v1.font.system")}{option("segoe_ui", "preferences.v1.font.segoe")}{option("microsoft_yahei_ui", "preferences.v1.font.yahei")}
        </select></label>
        <label><span>{t("preferences.v1.scale")}</span><select id="settings-text-scale" value={p.text_scale_percent} disabled={disabled} onChange={(e) => save({ field: "text_scale_percent", value: Number(e.target.value) })}>{numberOptions([100, 110, 125], "percent")}</select></label>
      </div>
      <div className="settings-font-preview" aria-label={t("preferences.v1.preview")}><span lang="en">DevSweep · Clear, readable text</span><span lang="zh-CN">开发环境 · 清晰可读</span><span className="tabular">0123456789 · 128 GiB</span></div>
    </section>
    <section className="card settings-group" aria-labelledby="settings-language-heading">
      <h2 id="settings-language-heading">{t("preferences.v1.language")}</h2>
      <label><span>{t("preferences.v1.language")}</span><select id="settings-language" value={locale} disabled={localeSaving} aria-busy={localeSaving} onChange={(e) => void onLocaleChange(e.target.value as PresentationLanguageTag)}>
        {option("en", "shell.v1.settings.option.en")}{option("zh-CN", "shell.v1.settings.option.zh_cn")}
      </select></label>
    </section>
    <section className="card settings-group" aria-labelledby="settings-performance">
      <div className="settings-group-heading"><h2 id="settings-performance">{t("preferences.v1.performance")}</h2>
        <button type="button" className="secondary-button" disabled={disabled} onClick={() => save({ field: "reset_performance" })}>{t("preferences.v1.reset.performance")}</button></div>
      <div className="settings-fields">
        <label><span>{t("preferences.v1.motion")}</span><select id="settings-motion" value={p.motion} disabled={disabled} onChange={(e) => save({ field: "motion", value: e.target.value as DesktopMotion })}>
          {option("system", "preferences.v1.motion.system")}{option("reduced", "preferences.v1.motion.reduced")}
        </select><small>{t("preferences.v1.motion.note")}</small></label>
        <label><span>{t("preferences.v1.fps")}</span><select id="settings-planet-fps" value={p.planet_fps} disabled={disabled || reduced} aria-describedby="planet-fps-note" onChange={(e) => save({ field: "planet_fps", value: Number(e.target.value) })}>{numberOptions([15, 30], "fps")}</select>
          <small id="planet-fps-note">{t(reduced ? "preferences.v1.fps.disabled" : "preferences.v1.fps.note")}</small></label>
        <label><span>{t("preferences.v1.status.interval")}</span><select id="settings-status-interval" value={p.status_interval_seconds} disabled={disabled} onChange={(e) => save({ field: "status_interval_seconds", value: Number(e.target.value) })}>{numberOptions([1, 2, 5, 10, 30, 60], "seconds")}</select><small>{t("preferences.v1.status.interval.note")}</small></label>
        <label><span>{t("preferences.v1.status.rows")}</span><select id="settings-status-rows" value={p.status_process_limit} disabled={disabled} onChange={(e) => save({ field: "status_process_limit", value: Number(e.target.value) })}>{numberOptions([5, 15, 30, 50, 100], "rows")}</select><small>{t("preferences.v1.status.rows.note")}</small></label>
        <label><span>{t("preferences.v1.hud.interval")}</span><select id="settings-hud-interval" value={p.hud_interval_seconds} disabled={disabled} onChange={(e) => save({ field: "hud_interval_seconds", value: Number(e.target.value) })}>{numberOptions([2, 5, 10], "seconds")}</select><small>{t("preferences.v1.hud.interval.note")}</small></label>
      </div>
    </section>
  </div>;
}
