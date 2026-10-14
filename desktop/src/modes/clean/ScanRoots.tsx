import { useState } from "react";
import { message, type PresentationLanguageTag } from "../../i18n";

export function ScanRoots({ roots, suggestions, disabled, locale, onChange }: {
  roots: readonly string[]; suggestions: readonly string[]; disabled: boolean;
  locale: PresentationLanguageTag; onChange: (roots: string[]) => Promise<boolean>;
}) {
  const [path, setPath] = useState("");
  const add = async (value: string) => {
    if (await onChange([...roots, value])) setPath("");
  };
  return <fieldset className="scan-roots" disabled={disabled}>
    <legend>{message(locale, "clean.v1.scope.roots")}</legend>
    <ul>{roots.map((root) => <li key={root}><span>{root}</span>
      <button type="button" className="stage-link" aria-label={`${message(locale, "clean.v1.scope.remove")} ${root}`} onClick={() => void onChange(roots.filter((item) => item !== root))}>{message(locale, "clean.v1.scope.remove")}</button>
    </li>)}</ul>
    {roots.length > 1 && <button type="button" className="stage-link" onClick={() => void onChange([])}>{message(locale, "clean.v1.scope.clear")}</button>}
    <div className="scan-root-entry"><label>{message(locale, "clean.v1.scope.path")}<input value={path} onChange={(event) => setPath(event.target.value)} /></label>
      <button type="button" className="secondary-button" disabled={!path.trim()} onClick={() => void add(path)}>{message(locale, "clean.v1.scope.add")}</button>
    </div>
    {roots.length === 0 && <p role="status">{message(locale, "clean.v1.scope.required")}</p>}
    {suggestions.filter((value) => !roots.includes(value)).length > 0 && <details><summary>{message(locale, "clean.v1.scope.suggestions")}</summary>
      <ul>{suggestions.filter((value) => !roots.includes(value)).map((value) => <li key={value}><span>{value}</span><button type="button" className="stage-link" onClick={() => void add(value)}>{message(locale, "clean.v1.scope.add")}</button></li>)}</ul>
    </details>}
    <p className="secondary">{message(locale, "clean.v1.scope.note")}</p>
  </fieldset>;
}
