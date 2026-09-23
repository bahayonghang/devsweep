import type { ReactNode } from "react";
import type { ModeId } from "../app-shell/AppShell";
import { Planet } from "./Planet";
import "./styles.css";

export interface StageProps {
  readonly mode: ModeId;
  /** Accessible name of the stage region. */
  readonly label: string;
  readonly className?: string;
  /** Defaults to the mode planet. */
  readonly hero?: ReactNode;
  readonly title?: ReactNode;
  readonly meta?: ReactNode;
  /** Controls the primary action needs, such as scope or path. */
  readonly controls?: ReactNode;
  readonly primary?: ReactNode;
  readonly secondary?: ReactNode;
  readonly busy?: boolean;
}

/** First-screen stack: planet, title or number, one secondary line, one
 * primary action, and an optional secondary action. */
export function Stage({
  mode,
  label,
  className,
  hero,
  title,
  meta,
  controls,
  primary,
  secondary,
  busy,
}: StageProps) {
  return (
    <section
      className={className ? `stage ${className}` : "stage"}
      aria-label={label}
      aria-busy={busy || undefined}
      data-stage-mode={mode}
    >
      <div className="stage-hero">{hero ?? <Planet mode={mode} />}</div>
      {title !== undefined ? <h2 className="stage-title">{title}</h2> : null}
      {meta !== undefined ? <div className="stage-meta">{meta}</div> : null}
      {controls !== undefined ? (
        <div className="stage-controls">{controls}</div>
      ) : null}
      {primary !== undefined ? (
        <div className="stage-primary">{primary}</div>
      ) : null}
      {secondary !== undefined ? (
        <div className="stage-secondary">{secondary}</div>
      ) : null}
    </section>
  );
}
