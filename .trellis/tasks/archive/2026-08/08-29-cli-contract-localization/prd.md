# Define breaking CLI and bilingual output contract

## Goal

Freeze the complete five-mode CLI grammar, locale-neutral machine schemas,
bilingual human output contract, bare-TUI behavior, and old-to-new migration
mapping before any downstream mode implements a handler.

## Requirements

- R1: Replace the public roots in one breaking release. Bare `devsweep` opens
  the TUI; no subcommand opens it. Roots are `clean`, `software`, `optimize`,
  `analyze`, `status`, and read-only `history`. Old `tui`, `scan`, `inventory`,
  `protect`, and `rules` roots are unknown commands, not aliases.
- R2: Implement exactly the grammar in `design.md`, including every option,
  required value, default, conflict, range, output destination, digest syntax,
  TTY rule, and overwrite rule. Downstream tasks may implement their owned
  handlers but may not add aliases or change parser shapes. This task owns the
  single `application/commands` and `application/presentation` module roots plus
  the compiler-registered empty mode/support skeleton files. Downstream tasks
  fill only the frozen submodule files; they do not edit either root `mod.rs`.
  This ownership clarification was explicitly approved after implementation
  exposed that Rust cannot register a missing external module file.
- R3: Mutating roots use observation -> saved plan -> live preview -> explicit
  execution. Clean/Software execution requires `--plan`, a matching
  `--preview-digest sha256:<64-lowercase-hex>`, and `--confirm`; Optimize uses the
  same authority chain for one exact operation. Inventory/JSON documents never
  become execution authority.
- R4: `--format human|json` exists only on the documented snapshot/list/preview/
  execution commands; `status live` supports only `human|ndjson`. Human is the
  default and never auto-switches because of redirection. Machine output is one
  versioned document/event per contract and contains no localized prose. The
  exact Status V1 availability union, snapshot fields/types/units/time bases,
  process truncation metadata, and live terminal lifecycle are frozen in
  `design.md` before any collector, IPC adapter, or renderer implements them.
- R5: Publish stable exit classes: 0 success, 2 usage/TTY/conflict, 3 stale or
  invalid authority, 4 unavailable/unsupported/permission, 5 truthful partial,
  6 failed or unknown after dispatch, and 130 canceled before dispatch. Broken
  pipe cancels and joins a producer and exits 0; other I/O failures exit 6.
- R6: Own the CLI-layer runtime `Locale`, message-key schema, canonical
  English/Simplified-Chinese catalogues, English/Chinese plural selection, one binary byte/unit formatter,
  locale-specific accelerator metadata, the non-destructive truncation contract,
  CLI locale resolution, human snapshots, and machine-output invariance.
  Non-interactive CLI precedence is explicit `--language`, Windows user locale,
  English. The shell child exclusively owns the versioned persisted TUI/Desktop
  preference and applies accelerator/truncation behavior; its core store uses a
  separate closed `PresentationLanguageTag` wire value (`en|zh-CN`) so core never
  imports this crate. TUI/Desktop adapters must exhaustively map that tag to this
  task's `Locale`; this task does not write a preference store or TUI adapter.
- R7: Generate English and Chinese command references and a complete migration
  table. Feature registration may keep incomplete modes absent during staged
  development, but the final integration exposes all frozen roots together.

## Acceptance Criteria

- [ ] AC1 (R1, R2): Parser and help snapshots cover every documented root,
      subcommand, option, value/range/default, conflict, removed root, bare-TUI
      behavior, and TTY/non-TTY case in both languages.
- [ ] AC2 (R4, R5): Golden JSON/NDJSON fixtures prove version fields, stable
      codes, locale invariance, one complete event per line, stdout/stderr
      separation, output create-new behavior, and graceful broken-pipe join.
      Status fixtures additionally prove every V1 primitive/unit/time field,
      availability variant, process truncation flag/count, sequence, and
      terminal reason exactly matches `design.md` across CLI and Tauri fixtures.
- [ ] AC3 (R3): Clean/Software/Optimize cannot dispatch without the correct
      saved plan, live preview digest, and `--confirm`; Analyze/Status cannot
      construct executable plans.
- [ ] AC4 (R6): Catalogue key/placeholder/plural/accelerator parity, fallback,
      injection-safe interpolation, English/Chinese count forms, binary unit
      boundary snapshots, accelerator collision rejection, long Chinese labels,
      non-destructive truncation fixtures, and locale-invariant machine fixtures
      pass; an exhaustive `PresentationLanguageTag` <-> `Locale` mapping fixture
      rejects unknown values, and no persisted setting file or TUI adapter is
      changed by this task.
- [ ] AC5 (R7): English/Chinese references and the migration table cover every
      old invocation shape, including removal of `clean --audit-log` and its
      deliberate no-conversion/no-import file disposition.
- [ ] AC6 (R1, R2, R7): Focused CLI tests, generated-reference checks,
      canonical command/presentation module-tree checks, `git diff --check`, and
      `just ci` pass with no domain collector/executor or desktop page
      implementation.

## Out of Scope

- Domain collectors, executors, desktop pages, native evidence, shell language
  persistence, or mode-specific copy beyond catalogue keys required by grammar.
- Compatibility aliases, automatic old-plan conversion, shell completion
  redesign, locale-dependent machine fields, or overwriting output files.
