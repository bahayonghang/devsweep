# CLI Reference

This reference follows the canonical Clap definition in
`crates/devsweep-cli/src/application/cli.rs`. Contract tests fail when a frozen
command path disappears from either the parser or this reference.

Run the binary from this repository with:

```powershell
cargo run --locked -p devsweep-cli --bin devsweep -- <COMMAND>
```

Bare `devsweep` opens the TUI only when both stdin and stdout are interactive
terminals. With redirected input or output, choose an explicit command.

## Global behavior

- `--language <en|zh-CN>` selects human output. An explicit flag wins over the
  supported Windows user locale; unknown locales fall back to English.
- `--language` is rejected for JSON, NDJSON, and plan-producing commands.
- Human output never changes to JSON merely because stdout is redirected.
- Optional `--output <FILE>` uses exclusive create-new behavior. Existing files
  are not truncated, and `-` is not a stdout sentinel.
- Input JSON paths are regular files; stdin and `-` are rejected. Relative paths
  resolve against the process working directory. Input and output must differ.
- `--help` prints help and `--version` prints the DevSweep version.

## Clean

| Command | Options and defaults |
| --- | --- |
| `clean scan` | Repeat `--root <PATH>` (default `.`); `--scope <projects|global|all>` (default `all`); optional `--rescan-target <TARGET_ID>`; `--format <human|json>`; optional `--output`. A rescan requires one root and a project-containing scope. |
| `clean plan` | Required `--observation <FILE>`, one or more `--select <TARGET_ID>`, and create-new `--output <FILE>`. Emits only a versioned plan. |
| `clean preview` | Required `--plan <FILE>`; `--format <human|json>`; optional `--output`. |
| `clean execute` | Required `--plan <FILE>`, `--preview-digest sha256:<64-lowercase-hex>`, and `--confirm`; `--format <human|json>`; optional `--output`. |
| `clean protect list` | `--format <human|json>`; optional `--output`. |
| `clean protect add` / `clean protect remove` | Required `--path <PATH>` and `--confirm`; `--format <human|json>`; optional `--output`. |
| `clean rules list` | `--format <human|json>`; optional `--output`. |
| `clean rules show` | Required `--id <RULE_ID>`; `--format <human|json>`; optional `--output`. |

## Software

| Command | Options and defaults |
| --- | --- |
| `software inventory` | `--source <all|arp|msi|msix>` (default `all`); `--format <human|json>`; optional `--output`. |
| `software plan` | Required `--inventory <FILE>`, one or more `--select <SOFTWARE_ID>`, and create-new `--output <FILE>`. |
| `software preview` | Required `--plan <FILE>`; `--format <human|json>`; optional `--output`. |
| `software uninstall` | Required `--plan <FILE>`, preview digest, and `--confirm`; `--format <human|json>`; optional `--output`. Only the current-user MSIX executor may eventually dispatch; MSI remains inventory/manual-disposition only. |

## Optimize

| Command | Options and defaults |
| --- | --- |
| `optimize list` | `--format <human|json>`; optional `--output`. |
| `optimize plan` | Required `--operation <OPERATION_ID>` and create-new `--output <FILE>`. |
| `optimize preview` | Required `--plan <FILE>`; `--format <human|json>`; optional `--output`. |
| `optimize run` | Required `--plan <FILE>`, preview digest, and `--confirm`; `--format <human|json>`; optional `--output`. |

## Analyze, Status, and History

| Command | Options and defaults |
| --- | --- |
| `analyze scan` | Required `--root <PATH>`; `--format <human|json>`; optional `--output`. This command cannot create an executable plan. |
| `status snapshot` | `--process-limit <1..100>` (default `15`); `--format <human|json>`; optional `--output`. |
| `status live` | `--interval <1..60>` (default `2`), `--process-limit <1..100>` (default `15`), `--format <human|ndjson>`; optional `--output` only with NDJSON. Human streaming requires interactive stdout. |
| `history list` | Optional `--domain <clean|software|optimize>`; `--limit <1..1000>` (default `100`); `--format <human|json>`; optional `--output`. |
| `history show` | Required `--operation-id <ID>`; `--format <human|json>`; optional `--output`. |

## Machine output and exit classes

JSON uses `{schema_version, command, outcome, data, warnings, error}`. NDJSON
contains one complete versioned event per line with an operation id and
monotonic sequence. Machine keys, enum values, codes, plans, digests, and audit
records are locale-neutral. Results use stdout or the selected output file;
progress and diagnostics use stderr.

| Exit | Meaning |
| ---: | --- |
| 0 | Success, including a gracefully joined broken pipe. |
| 2 | Usage, TTY, or option conflict. |
| 3 | Stale or invalid authority. |
| 4 | Unavailable, unsupported, or permission denied. |
| 5 | Truthful partial result. |
| 6 | Failure or unknown after dispatch, including non-broken-pipe I/O. |
| 130 | Canceled before dispatch. |

See [CLI migration](/guide/cli-migration) for the breaking transition from the
0.2 command surface.

## Generated parser manifest

This block is generated from the canonical Clap tree and exact-tested in both
language references. Do not edit it independently.

<!-- cli-contract-manifest:start -->
```text
devsweep|subcommands=analyze,clean,history,optimize,software,status|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--version[action=version;names=-;required=false;default=-;values=-;range=-]|constraints=bare_stdin_stdout_tty,language_session_only
devsweep clean|subcommands=execute,plan,preview,protect,rules,scan|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-]|constraints=language_human,create_new
devsweep clean scan|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--rescan-target[action=set;names=TARGET_ID;required=false;default=-;values=-;range=-];--root[action=append;names=PATH;required=false;default=.;values=-;range=-];--scope[action=set;names=SCOPE;required=false;default=all;values=projects,global,all;range=-]|constraints=language_human,create_new,rescan_single_project_root
devsweep clean plan|subcommands=-|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--observation[action=set;names=FILE;required=true;default=-;values=-;range=-];--output[action=set;names=FILE;required=true;default=-;values=-;range=-];--select[action=append;names=TARGET_ID;required=true;default=-;values=-;range=-]|constraints=plan_only,no_language,no_format,create_new,input_output_distinct
devsweep clean preview|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--plan[action=set;names=FILE;required=true;default=-;values=-;range=-]|constraints=language_human,create_new,input_output_distinct
devsweep clean execute|subcommands=-|options=--confirm[action=set_true;names=-;required=true;default=-;values=-;range=-];--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--plan[action=set;names=FILE;required=true;default=-;values=-;range=-];--preview-digest[action=set;names=DIGEST;required=true;default=-;values=-;range=-]|constraints=language_human,create_new,input_output_distinct,digest_confirm,no_prompt
devsweep clean protect|subcommands=add,list,remove|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-]|constraints=language_human,create_new
devsweep clean protect list|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-]|constraints=language_human,create_new
devsweep clean protect add|subcommands=-|options=--confirm[action=set_true;names=-;required=true;default=-;values=-;range=-];--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--path[action=set;names=PATH;required=true;default=-;values=-;range=-]|constraints=language_human,create_new,confirm
devsweep clean protect remove|subcommands=-|options=--confirm[action=set_true;names=-;required=true;default=-;values=-;range=-];--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--path[action=set;names=PATH;required=true;default=-;values=-;range=-]|constraints=language_human,create_new,confirm
devsweep clean rules|subcommands=list,show|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-]|constraints=language_human,create_new
devsweep clean rules list|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-]|constraints=language_human,create_new
devsweep clean rules show|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--id[action=set;names=RULE_ID;required=true;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-]|constraints=language_human,create_new
devsweep software|subcommands=inventory,plan,preview,uninstall|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-]|constraints=language_human,create_new
devsweep software inventory|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--source[action=set;names=SOURCE;required=false;default=all;values=all,arp,msi,msix;range=-]|constraints=language_human,create_new
devsweep software plan|subcommands=-|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--inventory[action=set;names=FILE;required=true;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=true;default=-;values=-;range=-];--select[action=append;names=SOFTWARE_ID;required=true;default=-;values=-;range=-]|constraints=plan_only,no_language,no_format,create_new,input_output_distinct
devsweep software preview|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--plan[action=set;names=FILE;required=true;default=-;values=-;range=-]|constraints=language_human,create_new,input_output_distinct
devsweep software uninstall|subcommands=-|options=--confirm[action=set_true;names=-;required=true;default=-;values=-;range=-];--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--plan[action=set;names=FILE;required=true;default=-;values=-;range=-];--preview-digest[action=set;names=DIGEST;required=true;default=-;values=-;range=-]|constraints=language_human,create_new,input_output_distinct,digest_confirm,no_prompt
devsweep optimize|subcommands=list,plan,preview,run|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-]|constraints=language_human,create_new
devsweep optimize list|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-]|constraints=language_human,create_new
devsweep optimize plan|subcommands=-|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--operation[action=set;names=OPERATION_ID;required=true;default=-;values=-;range=-];--output[action=set;names=FILE;required=true;default=-;values=-;range=-]|constraints=plan_only,no_language,no_format,create_new,input_output_distinct
devsweep optimize preview|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--plan[action=set;names=FILE;required=true;default=-;values=-;range=-]|constraints=language_human,create_new,input_output_distinct
devsweep optimize run|subcommands=-|options=--confirm[action=set_true;names=-;required=true;default=-;values=-;range=-];--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--plan[action=set;names=FILE;required=true;default=-;values=-;range=-];--preview-digest[action=set;names=DIGEST;required=true;default=-;values=-;range=-]|constraints=language_human,create_new,input_output_distinct,digest_confirm,no_prompt
devsweep analyze|subcommands=scan|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-]|constraints=language_human,create_new
devsweep analyze scan|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--root[action=set;names=PATH;required=true;default=-;values=-;range=-]|constraints=language_human,create_new
devsweep status|subcommands=live,snapshot|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-]|constraints=language_human,create_new
devsweep status snapshot|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--process-limit[action=set;names=PROCESS_LIMIT;required=false;default=15;values=-;range=1..100]|constraints=language_human,create_new
devsweep status live|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,ndjson;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--interval[action=set;names=INTERVAL;required=false;default=2;values=-;range=1..60];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-];--process-limit[action=set;names=PROCESS_LIMIT;required=false;default=15;values=-;range=1..100]|constraints=language_human,create_new,human_tty_output_conflict
devsweep history|subcommands=list,show|options=--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-]|constraints=language_human,create_new
devsweep history list|subcommands=-|options=--domain[action=set;names=DOMAIN;required=false;default=-;values=clean,software,optimize;range=-];--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--limit[action=set;names=LIMIT;required=false;default=100;values=-;range=1..1000];--output[action=set;names=FILE;required=false;default=-;values=-;range=-]|constraints=language_human,create_new
devsweep history show|subcommands=-|options=--format[action=set;names=FORMAT;required=false;default=human;values=human,json;range=-];--help[action=help;names=-;required=false;default=-;values=-;range=-];--language[action=set;names=LANGUAGE;required=false;default=-;values=en,zh-CN;range=-];--operation-id[action=set;names=ID;required=true;default=-;values=-;range=-];--output[action=set;names=FILE;required=false;default=-;values=-;range=-]|constraints=language_human,create_new
```
<!-- cli-contract-manifest:end -->
