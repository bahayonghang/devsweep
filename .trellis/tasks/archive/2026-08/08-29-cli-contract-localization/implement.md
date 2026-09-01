# Implement - Breaking CLI and Localization Contract

Do not begin until the revised task-tree summary receives a later explicit
implementation approval and this child alone is started. This planning turn is
not that approval.

## 1. Freeze parser and output infrastructure

1. Replace `application/cli.rs` with the exact grammar/default/conflict table in
   `design.md`; add parser/help snapshots before adding domain handlers.
2. Add `application/output.rs` for atomic create-new output, stdout/stderr
   separation, versioned envelopes, exit classification, and broken-pipe join.
3. Convert `application/commands.rs` into the CLI-owned
   `application/commands/mod.rs`, create `application/presentation/mod.rs`,
   declare every mode/support module, and create the ten command plus eight
   presentation skeleton files from `design.md`. This compiler-enforced
   skeleton ownership was explicitly approved after the initial review; later
   tasks fill their assigned files without editing either root `mod.rs`.
4. Refactor `application/mod.rs` so bare invocation opens TUI only with TTYs and
   typed mode dispatch cannot add undocumented parser shapes.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli --test cli_contract
rtk cargo test -p devsweep-cli application::cli
```

Rollback point: restore the three application files together; do not leave a
new parser feeding old ambiguous handlers.

## 2. Add canonical bilingual catalogues

1. Add the two canonical JSON catalogues and the Rust facade/resolver.
2. Validate exact key/placeholder/plural/accelerator parity, escaped
   interpolation, the frozen English/Chinese plural selectors, binary unit
   boundaries, mnemonic collisions, the truncation contract, supported locale
   tags, CLI precedence, the exhaustive shell-owned
   `PresentationLanguageTag` <-> CLI `Locale` mapping contract, rejection of
   unknown tags, and locale-invariant machine fixtures.
3. Do not create or modify a persisted setting; hand the resolver/store interface
   to the shell task.

Focused validation:

```powershell
rtk cargo test -p devsweep-cli i18n
rtk cargo test -p devsweep-cli --test cli_contract locale
```

Rollback point: remove both catalogues and facade as one unit so no surface can
silently fall back to a partial mixed-language set.

## 3. Generate references and migration evidence

1. Generate English/Chinese help/reference pages from the same clap definition.
2. Test every migration row, removed root, plan non-conversion, rejection of the
   old `--audit-log` flag, byte-for-byte preservation/non-discovery of legacy
   audit files, option range, digest grammar, output collision, TTY, and exit
   class.
3. Review the diff to confirm no collector, executor, desktop page, preference
   store, or compatibility alias entered this child.

Validation:

```powershell
rtk cargo test -p devsweep-cli
rtk git diff --check
rtk just ci
```

## 4. Manual native acceptance and stop points

- In Windows Terminal and redirected PowerShell, exercise bare invocation,
  `status live` human/NDJSON TTY rules, broken pipe, create-new output refusal,
  English/Chinese help, long paths, and UTF-8 Chinese output.
- Record commands, exit codes, stdout/stderr captures, terminal code page, and
  Windows locale. Automated snapshots do not replace this evidence.
- Stop and return to planning if any downstream mode requires a grammar change,
  a persisted CLI/TUI setting owner, a compatibility alias, or an output
  overwrite flag. Do not improvise those behaviors during implementation.
