# Design - Breaking CLI and Localization Contract

## Exact command grammar

`--language <en|zh-CN>` is a global option and is valid only when the effective
format is human. `--output <FILE>` is optional unless stated otherwise; omission
means stdout. A named output uses atomic create-new semantics and fails with
`output_exists`/exit 6 if the file exists. `-` is not a file sentinel; use stdout
by omitting `--output`.

```text
devsweep [--language <en|zh-CN>]

devsweep [--language <en|zh-CN>] clean scan
  [--root <PATH>]... [--scope <projects|global|all>]
  [--rescan-target <TARGET_ID>] [--format <human|json>] [--output <FILE>]
devsweep clean plan --observation <FILE> --select <TARGET_ID>...
  --output <FILE>
devsweep [--language <en|zh-CN>] clean preview --plan <FILE>
  [--format <human|json>] [--output <FILE>]
devsweep [--language <en|zh-CN>] clean execute --plan <FILE>
  --preview-digest <DIGEST> --confirm
  [--format <human|json>] [--output <FILE>]
devsweep [--language <en|zh-CN>] clean protect list
  [--format <human|json>] [--output <FILE>]
devsweep [--language <en|zh-CN>] clean protect add|remove --path <PATH>
  --confirm [--format <human|json>] [--output <FILE>]
devsweep [--language <en|zh-CN>] clean rules list
  [--format <human|json>] [--output <FILE>]
devsweep [--language <en|zh-CN>] clean rules show --id <RULE_ID>
  [--format <human|json>] [--output <FILE>]

devsweep [--language <en|zh-CN>] software inventory
  [--source <all|arp|msi|msix>] [--format <human|json>] [--output <FILE>]
devsweep software plan --inventory <FILE> --select <SOFTWARE_ID>...
  --output <FILE>
devsweep [--language <en|zh-CN>] software preview --plan <FILE>
  [--format <human|json>] [--output <FILE>]
devsweep [--language <en|zh-CN>] software uninstall --plan <FILE>
  --preview-digest <DIGEST> --confirm
  [--format <human|json>] [--output <FILE>]

devsweep [--language <en|zh-CN>] optimize list
  [--format <human|json>] [--output <FILE>]
devsweep optimize plan --operation <OPERATION_ID> --output <FILE>
devsweep [--language <en|zh-CN>] optimize preview --plan <FILE>
  [--format <human|json>] [--output <FILE>]
devsweep [--language <en|zh-CN>] optimize run --plan <FILE>
  --preview-digest <DIGEST> --confirm
  [--format <human|json>] [--output <FILE>]

devsweep [--language <en|zh-CN>] analyze scan --root <PATH>
  [--format <human|json>] [--output <FILE>]

devsweep [--language <en|zh-CN>] status snapshot
  [--process-limit <1..100>] [--format <human|json>] [--output <FILE>]
devsweep [--language <en|zh-CN>] status live
  [--interval <1..60>] [--process-limit <1..100>]
  [--format <human|ndjson>] [--output <FILE>]

devsweep [--language <en|zh-CN>] history list
  [--domain <clean|software|optimize>] [--limit <1..1000>]
  [--format <human|json>] [--output <FILE>]
devsweep [--language <en|zh-CN>] history show --operation-id <ID>
  [--format <human|json>] [--output <FILE>]
```

Defaults are: Clean root `.`; scope `all`; format `human`; Software source
`all`; Status process limit 15; live interval 2 seconds; history limit 100.
Repeated roots/selections preserve command-line order before canonical identity
deduplication. `--rescan-target` requires exactly one root and a scope containing
`projects`. Plan-producing commands accept no `--format` or `--language` and
write only their domain's versioned JSON plan. Observation, inventory, and plan
inputs are regular JSON files; stdin and `-` are rejected. Relative paths resolve
against the process working directory.

Mutating commands have no user-selectable audit path. They append under an
exclusive lock to `%LOCALAPPDATA%\DevSweep\audit\v1\<domain>.jsonl`; failure to
resolve, create, lock, flush, or append the journal fails closed. `history`
reads these same three stores and never searches arbitrary files. The removed
`--audit-log <PATH>` files and the legacy default
`%APPDATA%\devsweep\audit.jsonl` remain byte-for-byte untouched; there is no
automatic discovery, import, conversion, or replay from either legacy source.

## Conflicts and TTY behavior

- `--language` conflicts with `json` and `ndjson`; machine formats never consult
  Windows locale or the persisted presentation setting.
- Bare `devsweep` requires stdin and stdout TTYs. Otherwise it exits 2 and names
  the required explicit command; it never guesses a mode.
- `status live --format human` requires an interactive stdout TTY and conflicts
  with `--output`.
  `--format ndjson` is the explicit non-interactive stream contract.
- Mutating commands never prompt on stdin. Missing `--confirm`, plan, or digest
  is usage exit 2; a well-formed but stale/mismatched authority is exit 3.
- `DIGEST` is exactly `sha256:` plus 64 lowercase hexadecimal characters.
- Plan input and output paths must differ. No output file is truncated or
  replaced.

## Output contract

JSON uses one versioned envelope
`{schema_version, command, outcome, data, warnings, error}`. NDJSON emits one
complete versioned event per line with operation id and monotonic sequence.
Stable codes are ASCII identifiers. Human renderers consume typed outcomes and
catalogue keys; domain code never emits translations. Progress/diagnostics use
stderr; the selected result format uses stdout or the create-new output file.

### Status V1 common wire schema

Status uses no floating-point wire values. `u32`/`u64` below are nonnegative JSON
integers; `string` is UTF-8; all enum/reason values are closed ASCII identifiers.
`*_unix_ms` is UTC milliseconds since Unix epoch for display/correlation.
Elapsed windows, scheduler deadlines, delta rates, and age use a monotonic clock
and are serialized as integer `*_ms`. Bytes are integer bytes. Basis points are
1/100 of one percentage point.

Every metric group is exactly one `AvailabilityV1<T>` variant:

```text
{state:"available", sampled_at_unix_ms:u64, age_ms:u64, value:T}
{state:"partial", sampled_at_unix_ms:u64, age_ms:u64, value:T,
 reason_codes:string[1..]}
{state:"unavailable", sampled_at_unix_ms:u64|null, reason_code:string}
{state:"permission_denied", sampled_at_unix_ms:u64|null, reason_code:string}
{state:"unsupported", sampled_at_unix_ms:null, reason_code:string}
```

The `data` of `command:"status.snapshot"` is exactly `StatusSnapshotV1`:

```text
{
 snapshot_id:string, sampled_at_unix_ms:u64, sample_window_ms:u32,
 logical_processor_count:u32,
 cpu:AvailabilityV1<{system_utilization_basis_points:u32}>,
 memory:AvailabilityV1<{total_bytes:u64, available_bytes:u64, used_bytes:u64}>,
 volumes:AvailabilityV1<{items:[{volume_id:string,mount_points:string[],
   total_bytes:u64,available_bytes:u64}],complete:bool}>,
 network:AvailabilityV1<{interval_ms:u32,interfaces:[{interface_luid:string,
   name:string,rx_bytes_per_second:u64,tx_bytes_per_second:u64}]}>,
 power:AvailabilityV1<{battery_present:bool,
   ac_state:"online"|"offline"|"unknown",
   charge_basis_points:u32|null,remaining_seconds:u64|null}>,
 processes:AvailabilityV1<{items:[{pid:u32,name:string,
   cpu_basis_points_of_one_logical_core:u32,private_bytes:u64,
   read_bytes_per_second:u64,write_bytes_per_second:u64}],
   enumerated_count:u32,returned_count:u32,requested_limit:u32,
   enumeration_ceiling:u32,detail_budget_ms:u32,
   truncated_by_limit:bool,budget_exhausted:bool}>,
 unsupported_capabilities:[{code:"gpu_utilization"|"vram"|"thermal"|
   "fan"|"smart"|"physical_disk_activity",state:"unsupported",
   reason_code:"not_supported_v1"}]
}
```

`interface_luid` is a decimal string so JavaScript does not lose 64-bit
identity precision. `system_utilization_basis_points` is 0..10000 across the
whole system. Process CPU uses one-logical-core basis and may exceed 10000 for a
multithreaded process. `enumeration_ceiling` is 4096 and `detail_budget_ms` is
150 in V1. `truncated_by_limit` means more ranked rows existed than
`requested_limit`; `budget_exhausted` means detail collection stopped early.
Either condition makes the process group `partial` with a stable reason. Missing
battery hardware is represented by an available `battery_present:false` value,
not a zero charge. Static unsupported capabilities do not alone make the outer
envelope partial; an enabled supported group that is partial/unavailable/denied
does.

`status live --format ndjson` emits only these complete events, sequence starting
at 0 and increasing by one for every produced event:

```text
{schema_version:1,event:"status_started",operation_id:string,sequence:u64,
 emitted_at_unix_ms:u64,data:{interval_ms:u32,process_limit:u32}}
{schema_version:1,event:"status_snapshot",operation_id:string,sequence:u64,
 emitted_at_unix_ms:u64,data:StatusSnapshotV1}
{schema_version:1,event:"tick_skipped",operation_id:string,sequence:u64,
 emitted_at_unix_ms:u64,data:{reason:"sample_in_flight",skipped_total:u64}}
{schema_version:1,event:"status_terminal",operation_id:string,sequence:u64,
 emitted_at_unix_ms:u64,data:{reason:"completed"|"canceled"|
 "broken_pipe"|"producer_error",error_code:string|null}}
```

A writable stream emits exactly one terminal event. If a write itself returns
broken pipe, no further write is attempted because the sink is gone; the
producer still creates the internal `status_terminal/broken_pipe` lifecycle
event, cancels and joins the sampler, and exits 0. Test sinks must assert both
the serialized prefix and internal terminal/join evidence. Other sink errors
create/emit `producer_error` when possible and exit 6. The Tauri event payload is
the same event object; generated bindings and runtime decoders may not rename,
drop, widen, or localize fields.

## Locale and persistence boundary

Canonical catalogues are `resources/i18n/en.json` and
`resources/i18n/zh-CN.json`, embedded/validated at build time. This task owns the
key schema, placeholder signatures, `Locale`, and a pure resolver:

```text
CLI command: explicit CLI flag -> supported Windows user locale -> en
TUI/Desktop: session selection -> shell-owned persisted value
             -> supported Windows user locale -> en
```

The shell task owns the versioned store and TUI/Desktop adapters. To preserve the
existing dependency direction, core never imports the CLI-owned runtime
`Locale`. The store instead serializes a core-owned closed
`PresentationLanguageTag` whose only V1 values are `en` and `zh-CN`; the shell's
CLI/TUI boundary provides an exhaustive bidirectional mapping between that tag
and `Locale`. An unrecognized tag is not passed through as a string and is not a
fallback candidate. This task never writes the store. A CLI flag used while
opening the bare TUI is a session-only override; persistence occurs only through
the shell setting action.

## Catalogue rendering contract

Catalogue entries carry identical placeholder signatures plus explicit count
and accelerator metadata in both locales. English selects `one` only for an
integer count of exactly 1 and `other` otherwise; Simplified Chinese uses
`other` for every count. Missing variants or placeholder mismatches fail the
catalogue build gate.

Human byte values use one shared binary formatter: base 1024, `B` below 1024,
then `KiB`, `MiB`, `GiB`, or `TiB` with one ASCII-decimal fractional digit.
Machine fields remain integer bytes. Boundary snapshots cover 0, 1, 1023, 1024,
unit transitions, and the maximum supported value in both languages.

Each actionable message may define one locale-specific ASCII mnemonic. The
catalogue validator rejects duplicate mnemonics within a visible command/control
group; shell adapters bind and display the accepted metadata rather than
inventing their own keys. CLI human output never truncates. TUI/Desktop may
visually ellipsize only user data such as paths or application names when the
full value remains accessible and copyable; authority, refusal, warning, and
action text never truncates.

## Migration matrix

| Old shape | New shape / disposition |
| --- | --- |
| `devsweep tui` | `devsweep`; old root is rejected |
| `devsweep scan ROOT... [--global|--projects] [--json]` | `clean scan --root ROOT... --scope ... --format json`; old documents are not executable authority |
| `devsweep scan --rescan-target ID ROOT` | `clean scan --root ROOT --scope projects --rescan-target ID` |
| `devsweep inventory ROOT [--json]` | `analyze scan --root ROOT --format json`; schema is replaced |
| `devsweep clean --plan P` | rescan/replan, then `clean preview --plan NEW_P`; no old-plan conversion |
| `devsweep clean --plan P --execute` | rescan -> plan -> preview -> `clean execute` with digest and confirm |
| `devsweep clean --audit-log P` | option is removed; `P` is neither moved nor imported, and `history` does not search it |
| `devsweep protect list|add|remove` | `clean protect ...` with explicit `--path` and confirm for mutations |
| `devsweep rules` | `clean rules list` |

## Exact change list

| Path | Ownership and planned change |
| --- | --- |
| `crates/devsweep-cli/src/application/cli.rs` | exclusive grammar/default/conflict owner |
| `crates/devsweep-cli/src/application/mod.rs` | bare-TUI/TTY routing and typed dispatch boundary |
| `crates/devsweep-cli/src/application/commands.rs` -> `crates/devsweep-cli/src/application/commands/mod.rs` | convert the existing single handler file into the sole command-module root, remove obsolete legacy dispatch, and expose only frozen downstream submodules |
| `crates/devsweep-cli/src/application/commands/{clean,analyze,optimize,status,protect,rules,history}.rs` | create compiler-registered empty handler skeletons; downstream owning tasks fill their domain implementations without editing the root module |
| `crates/devsweep-cli/src/application/commands/software/{mod,inventory,execution}.rs` | create the compiler-registered Software hierarchy; `software/mod.rs` declares the two leaves and downstream Software tasks fill their owned implementations |
| `crates/devsweep-cli/src/application/presentation/mod.rs` | create the sole bilingual human-renderer module root over CLI-owned catalogue/output contracts |
| `crates/devsweep-cli/src/application/presentation/{clean,analyze,software,optimize,status,protect,rules,history}.rs` | create compiler-registered empty renderer skeletons; downstream owning tasks fill content without editing the root module |
| `crates/devsweep-cli/src/application/output.rs` | new output sink, envelope, broken-pipe/create-new behavior |
| `crates/devsweep-cli/src/i18n/mod.rs` | new locale/catalogue facade and resolver |
| `resources/i18n/en.json` | canonical English catalogue |
| `resources/i18n/zh-CN.json` | canonical Simplified-Chinese catalogue |
| `crates/devsweep-cli/tests/cli_contract.rs` | parser/help/stream/exit contract tests |
| `crates/devsweep-cli/tests/fixtures/cli/` | golden human/JSON/NDJSON and migration fixtures |
| `docs/reference/cli.md` | generated English command reference |
| `docs/zh/reference/cli.md` | generated Chinese command reference |
| `docs/guide/cli-migration.md` | sole complete breaking migration table |

The explicitly approved frozen command convention is
`application/commands/{clean,analyze,optimize,status,protect,rules,history}.rs`
plus `application/commands/software/{mod,inventory,execution}.rs`. The frozen
human-renderer tree is
`application/presentation/{clean,analyze,software,optimize,status,protect,rules,history}.rs`;
this task creates both root modules and all eighteen external-module skeleton
files so the compiler enforces the route immediately. Skeleton declarations and
empty initial contents belong to this task; downstream mode tasks own the later
domain implementation inside their assigned handler/renderer files and their
domain fixtures, not either root `mod.rs`, `cli.rs`, the catalogue schema, or
global output behavior. Rollback restores the previous CLI release and never
reinterprets new plans as old ones.
