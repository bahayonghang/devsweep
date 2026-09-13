import { useEffect, useMemo, useState } from "react";
import { ErrorBanner } from "../components/ErrorBanner";
import { decodeCommandError } from "../api/contract";
import type { CommandError } from "../api/types.gen";
import { message, type PresentationLanguageTag } from "../i18n";
import type { DesktopBridge } from "../api/bridge";
import type { HistoryDetailV1, HistoryListV1 } from "./types";
import "./styles.css";

interface HistoryPageProps {
  readonly bridge: DesktopBridge;
  readonly locale: PresentationLanguageTag;
}

export function HistoryPage({ bridge, locale }: HistoryPageProps) {
  const [listed, setListed] = useState<HistoryListV1>({ operations: [], stores: [] });
  const [query, setQuery] = useState("");
  const [detail, setDetail] = useState<HistoryDetailV1 | null>(null);
  const [error, setError] = useState<CommandError | null>(null);

  useEffect(() => {
    let cancelled = false;
    void bridge.historyList().then((next) => {
      if (!cancelled) {
        setListed(next);
        setError(null);
      }
    }).catch((caught) => {
      if (!cancelled) setError(commandError(caught));
    });
    return () => {
      cancelled = true;
    };
  }, [bridge]);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return needle
      ? listed.operations.filter((operation) => `${operation.operation_id} ${operation.outcome_code}`.toLowerCase().includes(needle))
      : listed.operations;
  }, [listed, query]);

  return <section className="support-page" aria-labelledby="mode-heading">
    <div className="card">
      <p>{message(locale, "history.v1.no_replay")}</p>
      {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}
      {listed.stores.filter((store) => store.state === "unavailable" || store.state === "unsupported").map((store) => (
        <p key={store.domain} role="status">
          {message(locale, store.state === "unsupported" ? "history.v1.unsupported" : "history.v1.store.unavailable", { domain: store.domain })}
        </p>
      ))}
      <label>
        {message(locale, "history.v1.list.summary", { count: String(visible.length) })}
        <input value={query} onChange={(event) => setQuery(event.target.value)} type="search" />
      </label>
      {visible.length === 0 ? <p>{message(locale, "history.v1.empty")}</p> : (
        <table>
          <thead><tr><th>{message(locale, "history.v1.column.domain")}</th><th>{message(locale, "history.v1.column.operation")}</th><th>{message(locale, "history.v1.column.outcome")}</th></tr></thead>
          <tbody>
            {visible.map((operation) => (
              <tr key={`${operation.domain}:${operation.operation_id}`}>
                <td>{operation.domain}</td>
                <td>
                  <button type="button" className="secondary-button" onClick={() => {
                    void bridge.historyShow(operation.operation_id).then(setDetail).catch((caught) => setError(commandError(caught)));
                  }}>{operation.operation_id}</button>
                </td>
                <td>{operation.outcome_code}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
    {detail && <article className="card" aria-labelledby="history-detail-title">
      <h2 id="history-detail-title">{message(locale, "history.v1.show.header", {
        id: detail.summary.operation_id,
        domain: detail.summary.domain,
        outcome: detail.summary.outcome_code,
      })}</h2>
      <p>{message(locale, "history.v1.no_replay")}</p>
      <p>{JSON.stringify(detail.records.map((record) => record.history_kind ?? record.record_kind ?? "record"))}</p>
    </article>}
  </section>;
}

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected history error" }; }
}
