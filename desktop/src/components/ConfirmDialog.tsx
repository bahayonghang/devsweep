import { useEffect, useRef } from "react";
import { message, type PresentationLanguageTag } from "../i18n";

export function ConfirmDialog(props: {
  locale?: PresentationLanguageTag;
  open: boolean; digest: string; irreversible: boolean; busy: boolean; onCancel: () => void; onConfirm: () => void;
}) {
  const locale = props.locale ?? "en";
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (props.open && !dialog.open) dialog.showModal();
    if (!props.open && dialog.open) dialog.close();
  }, [props.open]);
  return <dialog ref={ref} className="card" aria-labelledby="confirm-cleanup-title" aria-describedby="confirm-cleanup-description" onCancel={(event) => { event.preventDefault(); props.onCancel(); }}>
    <form method="dialog" onSubmit={(event) => event.preventDefault()}>
      <h2 id="confirm-cleanup-title">{message(locale, "clean.v1.confirm.title")}</h2>
      <p id="confirm-cleanup-description">{message(locale, "clean.v1.trash.moved")}</p>
      {props.irreversible && <p className="dialog-warning"><strong>{message(locale, "clean.v1.action.execute")}</strong></p>}
      <dl className="digest"><dt>{message(locale, "clean.v1.preview.digest", { digest: "" }).replace(/:?\s*$/, "")}</dt><dd>{props.digest}</dd></dl>
      <div className="dialog-actions"><button type="button" className="secondary-button" onClick={props.onCancel} disabled={props.busy}>{message(locale, "clean.v1.action.cancel")}</button><button type="button" className="danger-button" onClick={props.onConfirm} disabled={props.busy}>{props.busy ? message(locale, "clean.v1.action.execute") : message(locale, "clean.v1.action.execute")}</button></div>
    </form>
  </dialog>;
}
