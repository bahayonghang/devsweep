import type { ReactNode } from "react";
import { message, type PresentationLanguageTag } from "../i18n";
import "./styles.css";

export interface DetailViewProps {
  readonly locale: PresentationLanguageTag;
  /** Returns to the stage. Omit while the stage is visible above. */
  readonly onBack?: () => void;
  /** Status chip or other header content. */
  readonly status?: ReactNode;
  readonly className?: string;
  readonly children: ReactNode;
}

/** Full-width detail region that replaces the stage. */
export function DetailView({
  locale,
  onBack,
  status,
  className,
  children,
}: DetailViewProps) {
  return (
    <div className={className ? `detail-view ${className}` : "detail-view"}>
      {onBack || status ? (
        <div className="detail-view-bar">
          {onBack ? (
            <button type="button" className="detail-back" onClick={onBack}>
              <span aria-hidden="true">‹</span>
              <span>{message(locale, "stage.v1.action.back")}</span>
            </button>
          ) : null}
          {status ? <div className="detail-view-status">{status}</div> : null}
        </div>
      ) : null}
      {children}
    </div>
  );
}
