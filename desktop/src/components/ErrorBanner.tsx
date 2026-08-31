import type { CommandError } from "../api/types.gen";

function errorMessage(error: CommandError): string {
  switch (error.code) {
    case "scan_already_running": return "A scan is already running. Cancel it or wait for it to finish.";
    case "scan_failed": return `Scan failed: ${error.message}`;
    case "analyze_already_running": return "An analysis is already running. Cancel it or wait for it to finish.";
    case "analyze_failed": return `Analysis failed: ${error.message}`;
    case "software_already_running": return "A Software operation is already running. Cancel it or wait for it to finish.";
    case "software_failed": return `Software operation failed: ${error.message}`;
    case "software_stale_authority": return `Software authority is stale: ${error.message}`;
    case "software_audit_unavailable": return `Software audit is unavailable: ${error.message}`;
    case "invalid_plan": return `The scan plan is no longer valid: ${error.issues.join("; ")}`;
    case "stale_confirmation": return "The cleanup selection changed. Review the targets and run the dry run again.";
    case "unknown_target": return `Target ${error.target_id} is no longer available. Rescan before continuing.`;
    case "inspect_only_target": return `Target ${error.target_id} is inspect-only and cannot be executed.`;
    case "io": return `Desktop operation failed: ${error.message}`;
  }
}

export function ErrorBanner({ error, onDismiss }: { error: CommandError; onDismiss: () => void }) {
  return <div className="error-banner" role="alert"><span>{errorMessage(error)}</span><button className="icon-button" onClick={onDismiss} aria-label="Dismiss error" title="Dismiss error">×</button></div>;
}
