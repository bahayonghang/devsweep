# 命令行参考

本参考以 `crates/devsweep-cli/src/application/cli.rs` 中的 Clap 定义为唯一解析
契约；契约测试会检查冻结命令路径同时存在于解析器和本参考中。

在仓库中运行：

```powershell
cargo run --locked -p devsweep-cli --bin devsweep -- <COMMAND>
```

只有标准输入和标准输出均为交互式终端时，直接运行 `devsweep` 才会打开 TUI；
发生重定向时必须选择明确命令。

## 全局行为

- `--language <en|zh-CN>` 只控制人类可读输出。明确选项优先于受支持的
  Windows 用户区域设置；未知区域设置回退为英语。
- JSON、NDJSON 和生成计划的命令拒绝 `--language`。
- 输出重定向不会把默认的人类可读格式自动改成 JSON。
- `--output <FILE>` 使用排他的新建语义，不截断既有文件；`-` 不是标准输出哨兵。
- 输入 JSON 必须是普通文件路径，拒绝标准输入和 `-`；相对路径按进程工作目录
  解析，输入与输出路径必须不同。
- `--help` 输出帮助，`--version` 输出 DevSweep 版本。

## Clean

| 命令 | 选项和默认值 |
| --- | --- |
| `clean scan` | 可重复 `--root <PATH>`（默认 `.`）；`--scope <projects|global|all>`（默认 `all`）；可选 `--rescan-target <TARGET_ID>`；`--format <human|json>`；可选 `--output`。重新测量要求一个 root 且 scope 包含 projects。 |
| `clean plan` | 必需 `--observation <FILE>`、一个或多个 `--select <TARGET_ID>` 和新建的 `--output <FILE>`；只生成版本化计划。 |
| `clean preview` | 必需 `--plan <FILE>`；`--format <human|json>`；可选 `--output`。 |
| `clean execute` | 必需 `--plan <FILE>`、`--preview-digest sha256:<64 位小写十六进制>` 和 `--confirm`；`--format <human|json>`；可选 `--output`。 |
| `clean protect list` | `--format <human|json>`；可选 `--output`。 |
| `clean protect add` / `clean protect remove` | 必需 `--path <PATH>` 和 `--confirm`；`--format <human|json>`；可选 `--output`。 |
| `clean rules list` | `--format <human|json>`；可选 `--output`。 |
| `clean rules show` | 必需 `--id <RULE_ID>`；`--format <human|json>`；可选 `--output`。 |

## Software

| 命令 | 选项和默认值 |
| --- | --- |
| `software inventory` | `--source <all|arp|msi|msix>`（默认 `all`）；`--format <human|json>`；可选 `--output`。 |
| `software plan` | 必需 `--inventory <FILE>`、一个或多个 `--select <SOFTWARE_ID>` 和新建的 `--output <FILE>`。 |
| `software preview` | 必需 `--plan <FILE>`；`--format <human|json>`；可选 `--output`。 |
| `software uninstall` | 必需 `--plan <FILE>`、预览摘要和 `--confirm`；`--format <human|json>`；可选 `--output`。后续仅允许当前用户 MSIX 执行器实际派发，MSI 只盘点并人工处置。 |

## Optimize

| 命令 | 选项和默认值 |
| --- | --- |
| `optimize list` | `--format <human|json>`；可选 `--output`。 |
| `optimize plan` | 必需 `--operation <OPERATION_ID>` 和新建的 `--output <FILE>`。 |
| `optimize preview` | 必需 `--plan <FILE>`；`--format <human|json>`；可选 `--output`。 |
| `optimize run` | 必需 `--plan <FILE>`、预览摘要和 `--confirm`；`--format <human|json>`；可选 `--output`。 |

## Analyze、Status 和 History

| 命令 | 选项和默认值 |
| --- | --- |
| `analyze scan` | 必需 `--root <PATH>`；`--format <human|json>`；可选 `--output`。不得创建可执行计划。 |
| `status snapshot` | `--process-limit <1..100>`（默认 `15`）；`--format <human|json>`；可选 `--output`。 |
| `status live` | `--interval <1..60>`（默认 `2`）、`--process-limit <1..100>`（默认 `15`）、`--format <human|ndjson>`；只有 NDJSON 可以使用可选 `--output`，human 流要求交互式标准输出。 |
| `history list` | 可选 `--domain <clean|software|optimize>`；`--limit <1..1000>`（默认 `100`）；`--format <human|json>`；可选 `--output`。 |
| `history show` | 必需 `--operation-id <ID>`；`--format <human|json>`；可选 `--output`。 |

## 机器输出和退出类别

JSON 统一使用 `{schema_version, command, outcome, data, warnings, error}`；
NDJSON 每行是一个完整的版本化事件，含 operation id 和单调递增 sequence。机器
字段、枚举、稳定代码、计划、摘要和审计记录均不受语言影响。结果写入标准输出或
指定文件，进度和诊断写入标准错误。

| 退出码 | 含义 |
| ---: | --- |
| 0 | 成功，包括破管后正确取消并 join。 |
| 2 | 用法、TTY 或选项冲突。 |
| 3 | 过期或无效授权。 |
| 4 | 不可用、不支持或权限不足。 |
| 5 | 如实的部分结果。 |
| 6 | 失败或派发后未知，包括非破管 I/O 失败。 |
| 130 | 派发前取消。 |

从旧命令面迁移的方法见 [CLI 迁移](/guide/cli-migration)。

## 生成的解析器清单

此区块从 canonical Clap 树生成，并在英中参考文档中执行完全相等检查；不要单独编辑。

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
