import { useEffect, useRef } from "react";
import type { CommandError } from "../../api/types.gen";
import { AccessibleUserData } from "../../app-shell";
import { ErrorBanner } from "../../components/ErrorBanner";
import { message, type PresentationLanguageTag } from "../../i18n";

/** Confirms one exact path before it enters the protection list. */
export function ProtectDialog(props: {
  locale: PresentationLanguageTag;
  path: string | null;
  busy: boolean;
  error: CommandError | null;
  onCancel: () => void;
  onConfirm: () => void;
  onDismissError: () => void;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const open = props.path !== null;
  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);
  return (
    <dialog
      ref={ref}
      className="card protect-dialog"
      aria-labelledby="protect-target-title"
      aria-describedby="protect-target-description"
      onCancel={(event) => {
        event.preventDefault();
        if (!props.busy) props.onCancel();
      }}
    >
      {props.path !== null ? (
        <form method="dialog" onSubmit={(event) => event.preventDefault()}>
          <h2 id="protect-target-title">
            {message(props.locale, "clean.v1.protect.title")}
          </h2>
          <p id="protect-target-description">
            {message(props.locale, "protect.v1.confirm.add")}
          </p>
          <p className="protect-path">
            <AccessibleUserData value={props.path} />
          </p>
          {props.error && (
            <ErrorBanner error={props.error} onDismiss={props.onDismissError} />
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={props.onCancel}
              disabled={props.busy}
            >
              {message(props.locale, "shell.v1.settings.cancel")}
            </button>
            <button
              type="button"
              className="danger-button"
              onClick={props.onConfirm}
              disabled={props.busy}
            >
              {message(props.locale, "clean.v1.action.protect")}
            </button>
          </div>
        </form>
      ) : null}
    </dialog>
  );
}
