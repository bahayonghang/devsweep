# DevSweep Cleanup Planning

DevSweep models developer cleanup as an evidence-first planning workflow. These
terms distinguish observation and review from validated cleanup authority.

## Language

**Cleanup Target**:
An observed project artifact or global cache candidate with evidence, risk, and a
capacity estimate.
_Avoid_: File to delete, junk file

**Scan Progress**:
Correlated lifecycle information for an active scan, including its current phase
and backend-owned status; it is determinate only when a real bounded total exists.
_Avoid_: Percent complete, elapsed-time progress

**Scan Preview**:
A cumulative, read-only view of Cleanup Targets discovered during an active scan;
it is explicitly incomplete, carries observation facts rather than cleanup intent
or default selection, and never authorizes dry-run or execution.
_Avoid_: Partial plan, live plan, scan report

**Scan Report**:
The completed scan document containing health, diagnostics, capacity totals, and
an untrusted Cleanup Plan.
_Avoid_: Scan preview, executable plan

**Cleanup Plan**:
A declarative set of Cleanup Targets and typed cleanup intents that must be
validated before trusted actions can be reconstructed.
_Avoid_: Command list, deletion script

**Estimated Recoverable**:
Capacity associated with selected Cleanup Targets, separated into verified,
partial-lower-bound, and unknown observations.
_Avoid_: Space freed, released space

**Inspect Only**:
A Cleanup Target that may be reviewed but cannot be selected for cleanup.
_Avoid_: Disabled cleanup, safe to delete manually
