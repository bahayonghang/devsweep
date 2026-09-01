---
status: accepted
---

# Separate scan previews from cleanup authority

Active desktop scans will emit correlated, cumulative, read-only Scan Previews
alongside phase-aware indeterminate Scan Progress. The preview is projected at the
core/Tauri boundary as a purpose-built observation DTO without trusted program,
argv, cwd, direct action authority, cleanup intent, or default selection;
React stores it separately from the completed Scan Report and cannot select,
dry-run, or execute preview rows. This preserves the existing trust boundary while
showing discovered results continuously.

## Considered Options

Completed-phase checkpoints were smaller but could leave a long project scan
visually silent. A true percentage required a larger two-pass or bounded-work-unit
scanner and could imply elapsed-time accuracy that the current traversal cannot
support.

## Consequences

Progress messages need scan correlation and bounded emission. A command-scoped
ordered Tauri channel avoids application-global stream leakage; terminal results
distinguish completed, canceled, and failed work instead of inferring cancellation
from partial health. The core owns the preview projection, the desktop reducer owns
separate preview and completed-report state, the TUI keeps active staged targets
preview-only, and the final Scan Report remains the only source for default
selection and later dry-run/confirmation workflow.
