import { useEffect, useMemo, useState } from "react";
import { AccessibleUserData } from "../app-shell/AppShell";
import { ErrorBanner } from "../components/ErrorBanner";
import { decodeCommandError } from "../api/contract";
import type { CommandError } from "../api/types.gen";
import { message, type PresentationLanguageTag } from "../i18n";
import type { DesktopBridge } from "../api/bridge";
import "./styles.css";

interface ProtectionPageProps {
  readonly bridge: DesktopBridge;
  readonly locale: PresentationLanguageTag;
}

export function ProtectionPage({ bridge, locale }: ProtectionPageProps) {
  const [paths, setPaths] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<CommandError | null>(null);
  const [confirm, setConfirm] = useState<{ kind: "add" | "remove"; path: string } | null>(null);
  const [unavailable, setUnavailable] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void bridge.protectionList().then((next) => {
      if (cancelled) return;
      setPaths(next);
      setUnavailable(false);
      setError(null);
    }).catch((caught) => {
      if (cancelled) return;
      const decoded = commandError(caught);
      setUnavailable(decoded.code === "protection_store_unavailable");
      setError(decoded);
    });
    return () => {
      cancelled = true;
    };
  }, [bridge]);

  const visible = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return needle ? paths.filter((path) => path.toLowerCase().includes(needle)) : paths;
  }, [paths, query]);

  const runMutation = () => {
    if (!confirm) return;
    setBusy(true);
    const work = confirm.kind === "add"
      ? bridge.protectionAdd(confirm.path, true)
      : bridge.protectionRemove(confirm.path, true);
    void work.then(() => bridge.protectionList()).then((next) => {
      setPaths(next);
      setUnavailable(false);
      setError(null);
      setConfirm(null);
      setDraft("");
    }).catch((caught) => {
      setError(commandError(caught));
    }).finally(() => {
      setBusy(false);
    });
  };

  return <section className="support-page" aria-labelledby="protection-title">
    <h1 id="protection-title">{message(locale, "shell.v1.supporting.protection")}</h1>
    <p>{message(locale, "protect.v1.list.summary", { count: String(paths.length) })}</p>
    {unavailable && <p role="status">{message(locale, "protect.v1.store.unavailable")}</p>}
    {error && <ErrorBanner error={error} onDismiss={() => setError(null)} />}
    <label>
      {message(locale, "protect.v1.input.path")}
      <input value={query} onChange={(event) => setQuery(event.target.value)} type="search" />
    </label>
    {visible.length === 0 ? <p>{message(locale, "protect.v1.empty")}</p> : (
      <table>
        <thead><tr><th>{message(locale, "protect.v1.input.path")}</th><th>{message(locale, "protect.v1.action.remove")}</th></tr></thead>
        <tbody>
          {visible.map((path) => (
            <tr key={path}>
              <td><AccessibleUserData value={path} /></td>
              <td>
                <button type="button" className="secondary-button" onClick={() => setConfirm({ kind: "remove", path })}>
                  {message(locale, "protect.v1.action.remove")}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    )}
    <form onSubmit={(event) => {
      event.preventDefault();
      const submitted = new FormData(event.currentTarget).get("path");
      const path = (typeof submitted === "string" ? submitted : draft).trim();
      if (path) setConfirm({ kind: "add", path });
    }}>
      <label>
        {message(locale, "protect.v1.action.add")}
        <input name="path" value={draft} onChange={(event) => setDraft(event.target.value)} />
      </label>
      <button type="submit" className="primary-button">{message(locale, "protect.v1.action.add")}</button>
    </form>
    {confirm && <dialog open aria-modal="true" aria-labelledby="protect-confirm-title" onCancel={(event) => { event.preventDefault(); setConfirm(null); }}>
      <h2 id="protect-confirm-title">{message(locale, confirm.kind === "add" ? "protect.v1.confirm.add" : "protect.v1.confirm.remove")}</h2>
      <p><AccessibleUserData value={confirm.path} /></p>
      <button type="button" className="secondary-button" onClick={() => setConfirm(null)} disabled={busy}>{message(locale, "shell.v1.settings.cancel")}</button>
      <button type="button" className="danger-button" onClick={runMutation} disabled={busy}>{message(locale, confirm.kind === "add" ? "protect.v1.action.add" : "protect.v1.action.remove")}</button>
    </dialog>}
  </section>;
}

function commandError(error: unknown): CommandError {
  try { return decodeCommandError(error); }
  catch { return { code: "io", message: error instanceof Error ? error.message : "Unexpected protection error" }; }
}
