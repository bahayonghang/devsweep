# Design - Protection, Rules, and History

Keep three typed supporting services. Protection persistence uses an
OS-visible sidecar lock that covers the entire load -> version/byte validation
-> canonical exact-path compare -> mutation -> same-directory temp write/flush
-> atomic replace transaction. Each writer reloads after acquiring the lock, so
two CLI/TUI/Desktop processes cannot lose an update. Invalid UTF-8/JSON and
unknown/newer versions preserve the original bytes and return a stable
`protection_store_unavailable` refusal; no code treats them as an empty list,
auto-clears them, or deletes them. Display paths remain user-facing data.

Protection add/remove emits `requested`, `committed`, or `failed` transitions
through the Clean task's accepted fixed V1 writer. Clean owns the closed journal
envelope/schema; this task owns mutation emission. Its payload contains action,
operation id, stable result code, and SHA-256 of canonical identity, never the
raw path. Store replacement is not reported committed until the audit terminal
append is durably flushed; pre-mutation audit failure blocks the mutation, and a
post-replace audit failure returns fail-closed/unknown without rolling the store
back or repeating the mutation.

Rules project the existing authoritative registry into read-only DTOs. History
uses a tagged union per audit domain and exposes summaries/details without plan
payloads or executable text.

CLI/TUI/Desktop adapters share filters and stable codes, but presentation state
is local. These destinations live under Clean/help/navigation overflow rather
than the five-mode segment. Any store or record version not understood is shown
as unavailable/unsupported, never silently dropped or executed.

History receives exactly the three fixed V1 Clean/Software/Optimize store paths
from the shared application-data resolver. It performs no directory walk or
legacy-path probing and never opens removed `--audit-log` targets or the legacy
default audit file; those files remain untouched and outside the reader union.

Rollback removes the new adapters while preserving versioned protection and
audit files. Migration never overwrites an unknown newer document.

## Phase and ownership boundary

This is phase 7 support work. It must not begin until the Clean saved-plan audit,
Software uninstall audit, and Optimize execution audit schemas are implemented
and accepted, and the desktop shell registry is available. Those domain tasks
own record creation; this task owns only versioned readers, redaction, protection
persistence, the read-only rules projection, and supporting routes.

The frozen CLI spellings are `clean protect list|add|remove`, `clean rules list|show`,
and `history list|show`. Their exact arguments, output modes, conflicts, and exit
codes come only from `08-29-cli-contract-localization/design.md`.

## File-level change list

- `crates/devsweep-core/src/execution/safety/protections.rs`: add exact canonical
  path identities and immediate scan enforcement without expanding cleanup scope.
- `crates/devsweep-core/src/history/mod.rs`: add the versioned, non-replayable,
  redacted Clean/Software/Optimize reader union.
- `crates/devsweep-cli/src/application/commands/{protect,rules,history}.rs`: implement only
  the frozen support commands and machine-output contracts.
- `crates/devsweep-cli/src/application/presentation/{protect,rules,history}.rs`:
  implement the three bilingual human renderers over typed support outcomes.
- `crates/devsweep-cli/src/tui/support/`: add support destinations without a
  sixth primary mode.
- `desktop/src-tauri/src/support.rs` and `desktop/src-tauri/src/lib.rs`: expose
  typed protection/rules/history commands without executable audit payloads.
- `desktop/src/support/`: add searchable supporting views, corrupt/unknown states,
  and accessible confirmations.
- `crates/devsweep-core/tests/fixtures/{protection,history}/`: add concurrent,
  corrupt/unknown-byte preservation, lost-update, mutation-audit, mixed-version,
  redaction, reparse, and unknown-schema fixtures.
- `docs/reference/{protection,rules,history}.md`: document capability, storage,
  non-replay, redaction, and recovery contracts in both languages.

The CLI contract task first owns the conversion to
`application/commands/mod.rs`; this task fills only the three frozen support
submodules. The CLI task also creates `application/presentation/mod.rs`; this
task fills only `presentation/{protect,rules,history}.rs`. It creates neither
module root nor a parallel `src/commands/` tree.
