import type { ReactNode } from "react";
import type { ModeId } from "../app-shell/AppShell";
import { Stage } from "./Stage";

export interface StageResultProps {
  readonly mode: ModeId;
  readonly label: string;
  readonly className?: string;
  /** Short name of the number, shown above it. */
  readonly caption: string;
  /** Preformatted value from the shared byte/count formatters. */
  readonly value: string;
  readonly unit?: string;
  readonly meta?: ReactNode;
  readonly action: ReactNode;
  readonly secondary?: ReactNode;
}

/** One large number with unit, one line of facts, and one action. */
export function StageResult({
  mode,
  label,
  className,
  caption,
  value,
  unit,
  meta,
  action,
  secondary,
}: StageResultProps) {
  return (
    <Stage
      mode={mode}
      label={label}
      className={className ? `stage-result ${className}` : "stage-result"}
      title={
        <>
          <span className="stage-result-caption">{caption}</span>
          {" "}
          <span className="stage-result-figure">
            <span className="stage-result-value">{value}</span>
            {unit ? <span className="stage-result-unit"> {unit}</span> : null}
          </span>
        </>
      }
      meta={meta}
      primary={action}
      secondary={secondary}
    />
  );
}
