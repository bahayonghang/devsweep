import { useEffect, useMemo, useState } from "react";
import { message, type PresentationLanguageTag } from "../i18n";
import type { DesktopBridge } from "../api/bridge";
import type { RuleProjectionV1 } from "./types";
import "./styles.css";

interface RulesPageProps {
  readonly bridge: DesktopBridge;
  readonly locale: PresentationLanguageTag;
}

export function RulesPage({ bridge, locale }: RulesPageProps) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<RuleProjectionV1 | null>(null);
  const [rules, setRules] = useState<RuleProjectionV1[]>([]);

  useEffect(() => {
    void bridge.rulesList().then(setRules).catch(() => setRules([]));
  }, [bridge]);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return needle
      ? rules.filter((rule) => `${rule.id} ${rule.rationale}`.toLowerCase().includes(needle))
      : rules;
  }, [rules, query]);

  return <section className="support-page" aria-labelledby="mode-heading">
    <div className="card">
      <p>{message(locale, "rules.v1.inspect_only")}</p>
      <p>{message(locale, "rules.v1.source.shipped")}</p>
      <label>
        {message(locale, "rules.v1.list.summary", { count: String(visible.length) })}
        <input value={query} onChange={(event) => setQuery(event.target.value)} type="search" />
      </label>
      {visible.length === 0 ? <p>{message(locale, "rules.v1.empty")}</p> : (
        <table>
          <thead><tr><th>{message(locale, "rules.v1.column.id")}</th><th>{message(locale, "rules.v1.column.safety")}</th><th>{message(locale, "rules.v1.column.risk")}</th></tr></thead>
          <tbody>
            {visible.map((rule) => (
              <tr key={rule.id}>
                <td><button type="button" className="secondary-button" onClick={() => setSelected(rule)}>{rule.id}</button></td>
                <td>{rule.safety_class}</td>
                <td>{rule.risk}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
    {selected && <article className="card" aria-labelledby="rule-detail-title">
      <h2 id="rule-detail-title">{message(locale, "rules.v1.show.header", {
        id: selected.id,
        safety: selected.safety_class,
        scope: selected.scope,
        platform: selected.platform_applicability,
      })}</h2>
      <p>{selected.rationale}</p>
      <p>{message(locale, "rules.v1.inspect_only")}</p>
    </article>}
  </section>;
}
