# Bilingual Runtime Contract Research

## Confirmed user decision

DevSweep's CLI human output, TUI, and desktop runtime UI will support English and
Simplified Chinese. This is product behavior, not only bilingual documentation.

## Contract boundary

- One canonical message-key namespace is shared by the three presentation
  surfaces. Surface-specific keys are allowed, but duplicate concepts must not
  drift into unrelated translations.
- Explicit CLI `--language en|zh-CN` takes precedence for human output. TUI and
  desktop persist the user's explicit choice. Otherwise the supported Windows
  user locale is consulted and unknown locales fall back to English.
- JSON/NDJSON keys, enum representations, error codes, plan identities, preview
  digests, and audit records remain locale-neutral and byte-stable for the same
  semantic input.
- Localized text is presentation only. It cannot grant authority, affect a
  digest, become a command or path, or change selection/confirmation behavior.
- User-controlled strings are interpolated as plain data. Application names,
  paths, registry values, rule labels from external sources, and captured output
  are never message keys or markup.

## Required planning ownership

The CLI-contract child owns locale precedence, human-output snapshots, and
machine-output invariance. The desktop-shell child owns persisted selection,
font/fallback behavior, accelerators, and responsive layout. Each mode child owns
its message keys and both-language state/error copy. Final integration owns a
catalogue completeness gate and native Windows evidence at 100%, 125%, 150%, and
200% scaling.

## Dependency posture

Planning should first evaluate an internal compile-time catalogue with existing
serialization facilities. Adding an i18n dependency is not authorized merely by
this decision; any proposed crate must be named, justified, and approved with the
final implementation plan.
