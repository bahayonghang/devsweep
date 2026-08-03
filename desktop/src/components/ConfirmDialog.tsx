import { useEffect, useRef } from "react";

export function ConfirmDialog(props: { open: boolean; digest: string; irreversible: boolean; busy: boolean; onCancel: () => void; onConfirm: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (props.open && !dialog.open) dialog.showModal();
    if (!props.open && dialog.open) dialog.close();
  }, [props.open]);
  return <dialog ref={ref} aria-labelledby="confirm-cleanup-title" aria-describedby="confirm-cleanup-description" onCancel={(event) => { event.preventDefault(); props.onCancel(); }}>
    <form method="dialog" onSubmit={(event) => event.preventDefault()}>
      <h2 id="confirm-cleanup-title">Confirm cleanup</h2>
      <p id="confirm-cleanup-description">The reviewed actions will now run. Files moved to trash continue to occupy capacity until the trash is emptied.</p>
      {props.irreversible && <p className="dialog-warning"><strong>Irreversible command selected.</strong> Its changes cannot be restored from trash.</p>}
      <dl className="digest"><dt>Confirmation digest</dt><dd>{props.digest}</dd></dl>
      <div className="dialog-actions"><button type="button" className="secondary-button" onClick={props.onCancel} disabled={props.busy}>Cancel</button><button type="button" className="danger-button" onClick={props.onConfirm} disabled={props.busy}>{props.busy ? "Executing…" : "Execute cleanup"}</button></div>
    </form>
  </dialog>;
}
