# Build Protection, Rules, and History support surfaces

## Goal

Expose protection management, rule inspection, and versioned audit history as capability-backed supporting surfaces without creating additional product modes.

## Requirements

- R1: Protection is a persistent exact-path allowlist/deny-to-clean service with
  canonical identity, duplicate handling, reparse-safe display, add/remove
  confirmation, a locked read-validate-modify-atomic-replace transaction, and
  mutation audit through the Clean V1 writer. Corrupt/unknown store bytes are
  preserved and fail closed; they are never auto-cleared or deleted. It cannot
  expand cleanup scope.
- R2: Rules is a read-only catalogue of shipped rule identity, source, safety
  class, platform applicability, and inspect-only rationale. It cannot edit or
  import rule code in this task.
- R3: History reads versioned Clean/Software/Optimize audit records with domain,
  operation id, timestamps, requested/validated/outcome states, stable codes,
  partial/unknown evidence, and redaction. It never retries or replays actions.
  It reads only the three fixed V1 domain stores and never scans, imports, or
  guesses the removed `--audit-log` paths or legacy default audit file. The
  Clean union includes redacted protection-mutation records and never contains
  raw protection paths or executable text.
- R4: Expose these as `clean protect`, `clean rules`, and `history` CLI roots plus
  supporting TUI/Desktop destinations. Use shared bilingual contracts, search,
  filters, keyboard access, and truthful empty/corrupt/partial states.

## Acceptance Criteria

- [ ] AC1 (R1, R4): Protection tests cover normalization, duplicates, missing targets,
      reparse paths, full-transaction concurrent writers, corrupt/unknown byte
      preservation with fail-closed refusal, atomic update, stale removal,
      mutation audit transitions, and immediate scan enforcement.
- [ ] AC2 (R2, R4): Rule views enumerate exactly the authoritative registry, preserve
      inspect-only/safety metadata, and contain no edit/execute affordance.
- [ ] AC3 (R3, R4): History readers tolerate mixed supported versions, reject unknown
      schema safely, redact command/path data per domain, and cannot reconstruct
      executable argv or trigger replay. Fixtures prove legacy explicit/default
      audit files stay unchanged and are not discovered or imported, including
      Clean protection-mutation records with no raw path.
- [ ] AC4 (R1, R2, R3, R4): CLI/TUI/Desktop bilingual, accessibility, responsive, native, and
      `just ci` gates pass with no additional primary navigation mode.

## Out of Scope

- User-authored rules, cloud sync, audit replay, arbitrary history export, or
  protection by fuzzy display name.
