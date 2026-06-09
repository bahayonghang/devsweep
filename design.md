# Rust + ratatui 全局/项目级清理 TUI 工具实施方案

你的目标不应该是简单复刻 Mole，而是做一个 **“清理计划器 + 风险分级执行器”**。Mole/null-e/Kondo/clearrr 的共同问题是：它们容易把“全局包管理器缓存”和“项目构建产物”混在一起。这个工具最好从第一版就把二者拆开：

```text
全局清理 Global Cleanup
  npm / pip / pnpm / yarn / Cargo home / Docker builder cache / 未来扩展

项目级清理 Project Cleanup
  node_modules / Rust target / .venv / __pycache__ / .pytest_cache / build / dist / .next / .turbo ...
```

第一原则：**能调用官方命令就调用官方命令；只有项目产物目录才直接 move-to-trash；永久删除默认禁用。**

---

## 1. 产品定位

建议项目定位：

```text
一个跨平台、Rust 编写、ratatui 驱动的开发者磁盘清理 TUI。
它扫描全局开发缓存和项目级构建产物，生成可审计的 cleanup plan，
默认 dry-run，用户选择后执行可回收/可重建/可验证的清理动作。
```

它和现有工具的区别应该是：

| 维度                 | 设计取向                                              |
| ------------------ | ------------------------------------------------- |
| Mole-like 一站式体验    | 有，但不要牺牲安全性                                        |
| Kondo-like 项目扫描    | 必须有，尤其是 `target`、`node_modules`、`.venv`           |
| null-e-like 全局缓存覆盖 | 有，但每个 provider 明确风险                               |
| 官方命令优先             | npm/pip/pnpm/yarn/Docker/Cargo 都优先 command-backed |
| 可审计                | 每次清理生成 JSON plan + JSONL audit log                |
| 跨平台                | Windows 优先，Linux/macOS 同步设计                       |
| 可自动化               | TUI 之外提供 CLI/JSON 模式                              |

---

## 2. 技术栈选择

### TUI 层

建议使用：

```toml
ratatui = { version = "0.30", features = ["crossterm"] }
crossterm = "0.29"
```

当前 ratatui 文档显示 `ratatui` 为 0.30.1；官方文档也建议普通应用直接依赖 `ratatui` 主 crate，而不是拆到底层 workspace crate。Ratatui 默认启用 crossterm backend，官方说明 crossterm 是 Linux/macOS/Windows 都支持的合理默认选择。([Docs.rs][1])

ratatui 的应用结构建议采用 **TEA/Elm Architecture + 局部 Component** 的混合模型。TEA 把 TUI 拆成 `Model / Update / View`，ratatui 官方文档也明确推荐用这个结构组织应用状态、输入事件和渲染；复杂面板可以再用 Component trait 封装局部状态、事件处理和渲染逻辑。([Ratatui][2])

### CLI / 配置 / 文件系统

建议依赖：

```toml
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
serde_json = "1"
directories = "6"
ignore = "0.4"
trash = "5"
rayon = "1"
crossbeam-channel = "0.5"
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
```

理由：

* `clap` derive 适合把 TUI、scan、clean、json 输出等模式做成统一 CLI。其 derive API 需要启用 `derive` feature。([Docs.rs][3])
* `directories` 可按平台找到配置、缓存、数据目录，适合存放 `config.toml`、audit log 和 plan 文件。([Docs.rs][4])
* `serde`/`toml` 适合配置和 cleanup plan 序列化；Serde 是 Rust 生态中通用的序列化/反序列化框架。([serde.rs][5])
* `ignore` 可做高速递归扫描，但注意：它默认尊重 `.gitignore`/`.ignore` 规则；项目清理恰好要发现经常被 `.gitignore` 忽略的 `node_modules`、`target`、`.venv`，所以扫描清理产物时要显式关闭 gitignore 过滤，或只使用它的遍历框架与自定义过滤。([Docs.rs][6])
* `trash` crate 可把文件/目录移动到操作系统回收站/Trash，适合项目级目录的默认删除策略。([Docs.rs][7])

---

## 3. 功能范围

### MVP 必须支持

全局缓存：

| Provider             | 发现方式                                                        | 清理方式                                                   | 风险 |
| -------------------- | ----------------------------------------------------------- | ------------------------------------------------------ | -- |
| npm                  | `npm config get cache` 或默认路径                                | `npm cache verify` / `npm cache clean --force`         | 中  |
| pip                  | `py -m pip cache dir/info` 或 `python -m pip cache dir/info` | `py -m pip cache purge`                                | 中  |
| Cargo home           | `CARGO_HOME` / `$HOME/.cargo` / `%USERPROFILE%\.cargo`      | 默认仅展示；高级模式清 selected cache components                  | 中高 |
| pnpm                 | `pnpm store path`                                           | `pnpm store prune`                                     | 中低 |
| Yarn                 | `yarn cache clean` / `yarn cache clean --all`               | command-backed                                         | 中  |
| Docker builder cache | `docker system df` / `docker builder prune`                 | `docker builder prune --filter ... --keep-storage ...` | 中高 |

项目级清理：

| Ecosystem     | 检测 marker                                                  | 可清理目标                                                                                  | 默认风险       |
| ------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------- | ---------- |
| Rust          | `Cargo.toml`                                               | `target/`，优先 `cargo clean`                                                             | 低/中        |
| Node          | `package.json`、lockfile                                    | `node_modules/`、`.next/cache`、`.turbo`、`.parcel-cache`、`dist/`、`build/`                | 中          |
| Python        | `pyproject.toml`、`requirements.txt`、`setup.py`、`setup.cfg` | `.venv/`、`venv/`、`__pycache__/`、`.pytest_cache/`、`.mypy_cache/`、`.ruff_cache/`、`.tox/` | 低到中        |
| Generic build | `CMakeLists.txt`、`Makefile`、语言 marker                      | `build/`、`out/`、`dist/`                                                                | 中高，默认不自动选中 |

### MVP 不建议做

不要第一版就支持：

```text
系统垃圾清理
浏览器缓存
注册表清理
Docker volumes 清理
全盘无 marker 的 build/target/dist 删除
直接删除 %USERPROFILE%\.cargo 整个目录
直接删除用户指定根目录外的任意绝对路径
```

特别是 Cargo home：Cargo 官方文档明确说明 Cargo home 是下载和源码缓存，内部结构“不稳定，可能随时变化”；它包括 `bin`、`credentials.toml`、registry、git 等不同语义的内容。你的工具不应把整个 `%USERPROFILE%\.cargo` 当垃圾目录处理。([Rust Documentation][8])

---

## 4. 架构总览

建议分层：

```text
┌──────────────────────────────────────────────────────────┐
│ CLI / TUI Entrypoint                                     │
│  rclean tui | rclean scan | rclean clean | rclean rules   │
└──────────────────────────────────────────────────────────┘
                          │
┌──────────────────────────────────────────────────────────┐
│ App Model / State Machine                                │
│  tabs, selected targets, active jobs, logs, modal states  │
└──────────────────────────────────────────────────────────┘
                          │
┌──────────────────────────────────────────────────────────┐
│ Scan Engine                                              │
│  project scanner, global provider discovery, size walker  │
└──────────────────────────────────────────────────────────┘
                          │
┌──────────────────────────────────────────────────────────┐
│ Rule Engine                                              │
│  built-in rules + user config + allow/deny policy         │
└──────────────────────────────────────────────────────────┘
                          │
┌──────────────────────────────────────────────────────────┐
│ Execution Engine                                         │
│  dry-run, command-backed clean, trash move, permanent rm  │
└──────────────────────────────────────────────────────────┘
                          │
┌──────────────────────────────────────────────────────────┐
│ Audit / Plan                                             │
│  cleanup-plan.json, audit.jsonl, failure report           │
└──────────────────────────────────────────────────────────┘
```

---

## 5. 核心数据模型

建议从第一版就把“候选项”和“执行动作”分开。不要让 scanner 直接删除任何东西。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanTarget {
    pub id: TargetId,
    pub scope: Scope,
    pub ecosystem: Ecosystem,
    pub kind: TargetKind,
    pub path: Option<PathBuf>,
    pub estimated_bytes: u64,
    pub last_modified: Option<SystemTime>,
    pub risk: RiskLevel,
    pub reversible: bool,
    pub selected_by_default: bool,
    pub evidence: Vec<Evidence>,
    pub action: CleanAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Scope {
    Global,
    Project { root: PathBuf },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Ecosystem {
    Rust,
    Node,
    Python,
    Docker,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetKind {
    PackageCache,
    BuildArtifacts,
    DependencyDirectory,
    VirtualEnv,
    TestCache,
    ToolCache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Dangerous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CleanAction {
    Command {
        program: String,
        args: Vec<String>,
        cwd: Option<PathBuf>,
        irreversible: bool,
    },
    MoveToTrash {
        path: PathBuf,
    },
    DeletePermanently {
        path: PathBuf,
        requires_explicit_flag: bool,
    },
    NoopInspectOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Evidence {
    MarkerFile { path: PathBuf },
    KnownCacheDir { source: String, path: PathBuf },
    OfficialCommand { command: String },
    RuleMatched { rule_id: String },
    UserConfigured,
}
```

这个模型有几个好处：

1. TUI 可以展示“为什么判定它可清理”。
2. JSON plan 可以独立保存和复现。
3. 可以把 command-backed 清理和 path-backed 清理统一展示。
4. 风险分级不再是 UI 文案，而是执行策略的一部分。

---

## 6. Cleaner trait 设计

建议把每个生态做成 provider：

```rust
pub trait CleanerProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn supported_platforms(&self) -> PlatformSet;

    fn discover(&self, ctx: &DiscoverContext) -> anyhow::Result<Vec<CleanTarget>>;

    fn dry_run(&self, target: &CleanTarget) -> anyhow::Result<DryRunReport>;

    fn execute(
        &self,
        target: &CleanTarget,
        mode: ExecuteMode,
        sink: &dyn EventSink,
    ) -> anyhow::Result<CleanReport>;
}
```

`ExecuteMode`：

```rust
pub enum ExecuteMode {
    DryRun,
    Trash,
    Permanent,
    CommandBacked,
}
```

对 npm/pip/pnpm/yarn/Docker，`execute()` 基本就是启动官方命令。对项目级 `node_modules`、`.venv`、`target` fallback，`execute()` 走 `trash::delete()` 或永久删除。

---

## 7. 全局清理 provider 设计

### 7.1 npm provider

官方 npm 文档说明：

* `npm cache clean` 删除 cache folder 数据。
* npm cache 是 opaque `_cacache` 内容寻址缓存。
* npm cache 自愈，通常不需要清理；为了释放磁盘空间才需要清理。
* `npm cache clean` 需要 `--force`。
* Windows 默认 cache 路径是 `%LocalAppData%\npm-cache`。([npm Docs][9])

实现策略：

```text
Discover:
  1. 检测 npm 是否在 PATH。
  2. 运行 npm config get cache。
  3. 计算 cache dir size。
  4. 生成两个 target：
     - npm.verify: npm cache verify
     - npm.clean: npm cache clean --force

Dry-run:
  - 展示 cache path、size、将执行的命令。
  - 不解析 npm 内部 _cacache。

Execute:
  - 默认建议 npm cache verify。
  - 用户显式选择 aggressive clean 时执行 npm cache clean --force。
```

不要默认递归删除 `_cacache`。npm 官方明确说没有暴露直接 inspect/manage cache 内容的方法，直接硬删内部结构不是稳定接口。([npm Docs][9])

### 7.2 pip provider

pip 官方文档给出了 Windows 命令：

```powershell
py -m pip cache dir
py -m pip cache info
py -m pip cache list
py -m pip cache remove <pattern>
py -m pip cache purge
```

`purge` 用于移除所有 cache items；pip 文档还说明 Windows 默认路径是 `%LocalAppData%\pip\Cache`，并提醒 cache 内部文件系统结构是实现细节，可能在 pip 版本之间变化。([Pip Documentation][10])

实现策略：

```text
Discover:
  Windows:
    - 先尝试 py -m pip cache dir/info
    - 可选增强：py -0p 发现多个 Python，再分别查询 pip
  Unix/macOS:
    - python3 -m pip cache dir/info
    - python -m pip cache dir/info fallback

Execute:
  - py -m pip cache purge
  - 或 python -m pip cache purge

Risk:
  - Medium
  - command-backed irreversible
```

不要扫描并删除 `http-v2`、`wheels` 等内部目录，除非 provider 标记为 experimental。pip 官方已经明确 cache 结构是实现细节。([Pip Documentation][11])

### 7.3 Cargo home provider

Cargo home 比 npm/pip 更危险，因为它不只是下载缓存。官方文档说明 Cargo home 是下载和源码缓存，默认是 `$HOME/.cargo/`；它包含 `bin`、`credentials.toml`、`.crates.toml`、`git/db`、`git/checkouts`、`registry/index`、`registry/cache`、`registry/src` 等。([Rust Documentation][8])

同时 Cargo 环境变量文档说明，Windows 默认是 `%USERPROFILE%\.cargo`；并且 crate 一旦缓存，不会被 `cargo clean` 删除。([Rust Documentation][12])

实现策略：

```text
Discover:
  1. 读取 CARGO_HOME。
  2. 否则使用 home/.cargo。
  3. 分组件统计大小：
     - registry/cache
     - registry/src
     - registry/index
     - git/db
     - git/checkouts
  4. 永远不要把 bin、credentials.toml、config.toml、.crates.toml、.crates2.json 标为可清理。

Default:
  - inspect-only
  - 展示 Rust 1.88+ Cargo 自动 GC 提示

Advanced:
  - 可选清 registry/src：可从 registry/cache 重新解压或重新下载
  - 可选清 git/checkouts：可从 git/db 重新 checkout
  - 高风险清 registry/cache / git/db：会导致重新下载
```

Rust 1.88 起，Cargo 会对 Cargo home cache 做自动 GC：网络下载文件未访问 3 个月会清理，本地来源文件未访问 1 个月会清理；离线/冻结模式不会运行。你的工具应展示这一事实，并把 Cargo home 清理默认放到“高级”而不是“默认选中”。([blog.rust-lang.org][13])

### 7.4 Rust 项目 `target` provider

Rust 项目级清理优先用 `cargo clean`，不要直接删 `target`。Cargo 官方文档说明 `cargo clean` 会删除 target directory 中 Cargo 生成的 artifacts；不带选项时删除整个 target directory；`--dry-run -v` 可预览实际将删除的文件。([Rust Documentation][14])

实现策略：

```text
Discover:
  - 找到 Cargo.toml。
  - 尝试 cargo metadata --format-version 1，拿 workspace_root 和 target_directory。
  - fallback：workspace_root/target。

Dry-run:
  - cargo clean --manifest-path <Cargo.toml> --dry-run -v
  - 同时计算 target_directory size。

Execute:
  - cargo clean --manifest-path <Cargo.toml>
  - 如果 cargo 不在 PATH，fallback 为 MoveToTrash(target)，risk 提高到 Medium。
```

Cargo metadata 是机器可读的项目/工作区元数据接口，适合用来确认 workspace root 与 target directory，而不是靠字符串拼路径。([Rust Documentation][15])

### 7.5 pnpm provider

pnpm 官方文档说明 `pnpm store prune` 会移除 store 中不再被系统项目引用的 packages；官方也说它不伤害项目，但未来安装可能需要重新下载，因此建议偶尔运行，不要太频繁。([pnpm][16])

实现策略：

```text
Discover:
  - pnpm store path
  - 计算 store size

Execute:
  - pnpm store prune

Risk:
  - Medium-Low
  - command-backed
```

### 7.6 Yarn provider

Yarn 当前文档说明 `yarn cache clean` 会移除 cache 文件；`--mirror` 清全局 cache；`--all` 同时清全局 cache 和当前项目本地 cache。([Yarn][17])

实现策略：

```text
Discover:
  - yarn --version
  - yarn config get cacheFolder，失败则降级为命令 dry-run only

Execute:
  Yarn modern:
    - yarn cache clean --all
  Yarn classic:
    - yarn cache clean
```

Yarn v1 和 v2+ 行为差异明显，因此 provider 必须先检测版本。

### 7.7 Docker provider

Docker 官方文档说明 `docker builder prune` 用于删除 build cache，支持 `--all`、`--filter until=24h`、`--force`、`--keep-storage` 等选项。([Docker Documentation][18])

实现策略：

```text
Discover:
  - docker version
  - docker system df
  - 可选 docker builder du

Execute:
  Conservative:
    docker builder prune --filter until=168h --keep-storage 20GB --force

  Aggressive:
    docker builder prune --all --force
```

Docker volumes、containers、images prune 不进 MVP 默认功能。builder cache 可以做，volumes 不要默认碰。

---

## 8. 项目级扫描规则

### 8.1 关键规则：marker-first

不要看到目录名就删。必须有证据：

```text
node_modules     必须在 package.json / lockfile 项目下
target           必须由 Cargo.toml / cargo metadata 证明
.venv / venv     必须在 Python marker 项目下，或 pyvenv.cfg 存在
dist / build     必须结合 ecosystem marker，并默认不自动选中
__pycache__      可低风险清理，但仍应限制在扫描 root 内
```

### 8.2 去重规则

如果发现：

```text
project/node_modules
project/node_modules/.cache
```

只保留父 target，避免重复计数和重复删除。

规则：

```rust
fn deduplicate_targets(mut targets: Vec<CleanTarget>) -> Vec<CleanTarget> {
    // 1. canonicalize path if possible
    // 2. sort by path depth ascending
    // 3. if candidate path is inside an already selected parent path, drop child
}
```

### 8.3 symlink / junction / reparse point 策略

默认：

```text
不跟随 symlink
不跟随 Windows junction/reparse point
不删除 symlink 指向目标，只删除 symlink 本身时也要单独标记
```

理由：清理工具最容易犯的灾难性错误就是跨 root 删除。

### 8.4 目录规则表

| 规则 ID               | 匹配               | marker                      | 默认选中  | 删除方式                     |
| ------------------- | ---------------- | --------------------------- | ----- | ------------------------ |
| `rust.target`       | `target/`        | `Cargo.toml` + metadata     | 是     | `cargo clean`            |
| `node.node_modules` | `node_modules/`  | `package.json`              | 否，可配置 | trash                    |
| `node.next_cache`   | `.next/cache/`   | `package.json`              | 是     | trash                    |
| `node.turbo`        | `.turbo/`        | `package.json`              | 是     | trash                    |
| `python.pycache`    | `__pycache__/`   | Python root 或任意 Python 文件附近 | 是     | trash/permanent optional |
| `python.pytest`     | `.pytest_cache/` | Python root                 | 是     | trash                    |
| `python.venv`       | `.venv/`、`venv/` | `pyvenv.cfg` 或 Python root  | 否     | trash                    |
| `generic.build`     | `build/`         | marker required             | 否     | trash                    |
| `generic.dist`      | `dist/`          | marker required             | 否     | trash                    |

---

## 9. 配置格式

建议配置文件：

```text
Windows: %AppData%\YourOrg\rclean\config.toml
macOS: ~/Library/Application Support/...
Linux: ~/.config/rclean/config.toml
```

示例：

```toml
[scan]
roots = ["D:/code", "C:/Users/geekyhang/source/repos"]
max_depth = 12
follow_symlinks = false
respect_gitignore = false
threads = 8

[ui]
default_tab = "projects"
show_hidden = true
confirm_phrase = true

[execution]
default_mode = "trash"
allow_permanent_delete = false
command_timeout_secs = 600
audit_log = true

[selection]
auto_select_low_risk = true
auto_select_medium_risk = false
auto_select_high_risk = false

[providers.npm]
enabled = true
default_action = "verify" # verify | clean

[providers.pip]
enabled = true
python_launchers = ["py", "python", "python3"]

[providers.cargo_home]
enabled = true
mode = "inspect" # inspect | safe-components | aggressive

[providers.docker]
enabled = false
builder_prune_until = "168h"
keep_storage = "20GB"

[[project_rules]]
id = "rust.target"
ecosystem = "rust"
markers = ["Cargo.toml"]
match_dirs = ["target"]
action = "cargo_clean"
risk = "low"
selected_by_default = true

[[project_rules]]
id = "node.node_modules"
ecosystem = "node"
markers = ["package.json"]
match_dirs = ["node_modules"]
action = "trash"
risk = "medium"
selected_by_default = false
```

---

## 10. TUI 信息架构

### 主界面布局

建议 5 个 tab：

```text
[Dashboard] [Global] [Projects] [Rules] [Jobs/Logs]
```

主布局：

```text
┌────────────────────────────────────────────────────────────────────┐
│ rclean  Global: 4.2GB  Projects: 18.6GB  Selected: 7.1GB  [SCAN]  │
├───────────────┬─────────────────────────────────────┬──────────────┤
│ Categories    │ Targets                             │ Details      │
│               │                                     │              │
│ Rust          │ [x] D:\code\a\target        3.1 GB  │ Evidence     │
│ Node          │ [ ] D:\code\b\node_modules  1.8 GB  │ Risk         │
│ Python        │ [x] pip cache               620 MB  │ Command      │
│ Docker        │ [ ] docker builder cache    9.4 GB  │ Preview      │
│               │                                     │              │
├───────────────┴─────────────────────────────────────┴──────────────┤
│ Space select | Enter details | s scan | c clean | / filter | ? help │
└────────────────────────────────────────────────────────────────────┘
```

### 交互键位

| Key     | 行为              |
| ------- | --------------- |
| `s`     | 扫描              |
| `Space` | 选择/取消选择         |
| `a`     | 当前分类全选/取消       |
| `Enter` | 展开详情            |
| `d`     | 查看 dry-run      |
| `c`     | 清理选中项           |
| `/`     | 过滤              |
| `r`     | 切换风险过滤          |
| `p`     | 查看 cleanup plan |
| `l`     | 查看日志            |
| `?`     | help            |
| `q`     | 退出              |

### 确认弹窗

对于可回收清理：

```text
Move 12 selected targets to Trash?
Estimated reclaimable space: 7.1 GB
Type: clean
```

对于 irreversible command-backed 清理：

```text
This will run irreversible cache commands:
  npm cache clean --force
  py -m pip cache purge
  docker builder prune --filter until=168h --keep-storage 20GB --force

Type: CLEAN 7.1GB
```

对于 permanent delete：

```text
Permanent delete is disabled by default.
Re-run with --allow-permanent-delete or change config.
```

---

## 11. App 状态机

建议事件模型：

```rust
pub enum Event {
    Tick,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Worker(WorkerEvent),
}

pub enum Action {
    Quit,
    Scan(ScanRequest),
    ScanStarted(JobId),
    ScanProgress(ScanProgress),
    ScanFinished(Vec<CleanTarget>),
    ToggleTarget(TargetId),
    SelectByRisk(RiskLevel),
    OpenDetails(TargetId),
    OpenConfirm,
    ExecuteSelected,
    JobProgress(JobProgress),
    JobFinished(JobReport),
    ShowError(String),
}
```

主循环：

```rust
loop {
    terminal.draw(|frame| view(&mut app, frame))?;

    match event_rx.recv()? {
        Event::Key(key) => {
            let action = app.handle_key(key);
            app.update(action, &worker_tx)?;
        }
        Event::Worker(worker_event) => {
            let action = Action::from(worker_event);
            app.update(action, &worker_tx)?;
        }
        Event::Tick => {
            app.update(Action::Tick, &worker_tx)?;
        }
        _ => {}
    }

    if app.should_quit {
        break;
    }
}
```

TUI 渲染必须是纯视图逻辑，不要在 `view()` 里启动扫描、删除或计算大目录大小。ratatui/TEA 文档也强调 view 应根据 model 生成 UI 表示，副作用应放在 update/worker 流程之外。([Ratatui][2])

---

## 12. 扫描引擎

### 12.1 扫描流程

```text
1. 读取配置 roots。
2. 对每个 root 启动 ProjectScanner。
3. 并行遍历目录。
4. 对每个目录执行 rule matcher。
5. 对匹配目标计算 size、mtime、risk、evidence。
6. 去重。
7. 合并 GlobalProvider discover 结果。
8. 输出 CleanTarget 列表。
```

### 12.2 目录遍历注意点

`ignore` crate 默认会尊重 `.gitignore`，但 `node_modules`、`target`、`.venv` 通常正是被 `.gitignore` 忽略的内容，所以项目清理扫描应设置：

```rust
WalkBuilder::new(root)
    .hidden(false)
    .git_ignore(false)
    .git_global(false)
    .git_exclude(false)
    .parents(false)
    .follow_links(false)
```

这样它不会因为 `.gitignore` 错过你真正要清理的目录。`ignore` 的默认行为和 ignore 文件优先级在其文档中有说明。([Docs.rs][6])

### 12.3 size 估算

清理工具的体验取决于 size 估算速度。建议：

```text
小目录：同步计算
大目录：异步计算，先显示 "estimating..."
扫描进度：每 N 个目录或每 100ms 发一次事件
取消：worker 检查 CancellationToken
```

目录 size 计算要复用 traversal 结果，避免重复扫全树。

---

## 13. 执行引擎

### 13.1 三种执行方式

```text
CommandBacked
  npm cache clean --force
  py -m pip cache purge
  pnpm store prune
  yarn cache clean --all
  docker builder prune ...
  cargo clean ...

MoveToTrash
  node_modules
  .venv
  target fallback
  .pytest_cache
  .next/cache

PermanentDelete
  默认禁用
  只允许用户显式配置或 CLI flag
```

### 13.2 命令执行安全

不要用 shell 拼接命令：

```rust
Command::new("npm")
    .args(["cache", "clean", "--force"])
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
```

避免：

```rust
cmd /C format!("npm cache clean {}", user_input)
```

所有用户输入路径必须作为独立 arg 传入，不能进入 shell string。

### 13.3 audit log

每次执行写入：

```json
{
  "timestamp": "2026-06-08T...",
  "target_id": "rust.target:D:/code/foo",
  "action": "Command",
  "command": ["cargo", "clean", "--manifest-path", "D:/code/foo/Cargo.toml"],
  "estimated_bytes": 1234567890,
  "status": "success",
  "duration_ms": 2310
}
```

失败也要记录：

```json
{
  "status": "failed",
  "error": "Access denied: ...",
  "partial": true
}
```

---

## 14. Rust 项目级清理的具体策略

不要直接扫到 `target` 就删。正确流程：

```text
1. 发现 Cargo.toml。
2. cargo metadata --format-version 1。
3. 从 metadata 拿 workspace_root 和 target_directory。
4. 如果 target_directory 存在，生成 CleanTarget。
5. dry-run 使用 cargo clean --dry-run -v。
6. execute 使用 cargo clean。
```

如果 `cargo metadata` 失败：

```text
- 如果 Cargo.toml 能解析为 workspace/package：fallback root/target。
- risk 从 Low 提到 Medium。
- action 从 cargo_clean 降级为 MoveToTrash。
```

Cargo clean 支持 `--target-dir`、`--manifest-path`、`--dry-run`、`--release`、`--profile` 等参数；后续可以在详情面板里允许用户只清 release/doc/profile。([Rust Documentation][14])

---

## 15. Windows 专项设计

### 路径

要显式处理：

```text
%LocalAppData%\npm-cache
%LocalAppData%\pip\Cache
%USERPROFILE%\.cargo
```

这些路径分别来自 npm、pip、Cargo 官方文档。([npm Docs][9])

### Python launcher

Windows 上优先：

```powershell
py -m pip cache dir
py -m pip cache info
py -m pip cache purge
```

pip 官方文档对 Windows 就是这样写的。([Pip Documentation][10])

### 删除策略

Windows 上很多目录会被占用，例如：

```text
node_modules 中被 IDE / language server 占用的文件
target 中正在运行的 binary
Docker Desktop 文件锁
OneDrive 同步目录
```

处理方式：

```text
1. 失败时不要中断整个 job。
2. 标记 partial failure。
3. UI 提示哪个 path 失败。
4. 支持 retry。
5. 支持 "open in Explorer" 后续扩展。
```

### 长路径

Rust `std::fs` 在 Windows 长路径场景可能遇到问题。建议内部统一使用 `PathBuf`，不要字符串拼接；必要时封装 Windows extended-length path 处理，但 MVP 可先记录失败并提示。

---

## 16. CLI 设计

TUI 之外必须提供 CLI，否则难以测试和自动化。

```bash
# 进入 TUI
rclean tui

# 扫描当前目录
rclean scan .

# 扫描多个根并输出 JSON
rclean scan D:\code C:\Users\me\source\repos --json > plan.json

# 只扫全局缓存
rclean scan --global

# 只扫项目产物
rclean scan --projects D:\code

# 执行 plan，默认 dry-run
rclean clean --plan plan.json

# 真正执行
rclean clean --plan plan.json --execute

# 允许永久删除，默认不允许
rclean clean --plan plan.json --execute --allow-permanent-delete
```

建议默认：

```text
scan 只生成计划
clean 默认 dry-run
clean --execute 才执行
permanent delete 必须二次 flag
```

---

## 17. Repo 结构

建议：

```text
rclean/
  Cargo.toml
  src/
    main.rs
    cli.rs
    app.rs
    action.rs
    model.rs
    tui/
      mod.rs
      layout.rs
      widgets.rs
      dashboard.rs
      global.rs
      projects.rs
      details.rs
      confirm.rs
      help.rs
    events.rs
    scanner/
      mod.rs
      project.rs
      size.rs
      dedupe.rs
      markers.rs
    rules/
      mod.rs
      builtin.rs
      config.rs
      matcher.rs
    providers/
      mod.rs
      npm.rs
      pip.rs
      cargo_home.rs
      cargo_project.rs
      pnpm.rs
      yarn.rs
      docker.rs
    executor/
      mod.rs
      command.rs
      trash.rs
      plan.rs
      audit.rs
    fs/
      mod.rs
      path.rs
      platform.rs
      symlink.rs
    config/
      mod.rs
      defaults.rs
    error.rs
  tests/
    fixtures/
      rust_workspace/
      node_project/
      python_project/
    scanner_tests.rs
    rules_tests.rs
    plan_tests.rs
```

---

## 18. 内置规则实现示例

```rust
pub fn builtin_rules() -> Vec<ProjectRule> {
    vec![
        ProjectRule {
            id: "rust.target".into(),
            ecosystem: Ecosystem::Rust,
            markers: vec!["Cargo.toml".into()],
            dir_names: vec!["target".into()],
            action: RuleAction::CargoClean,
            risk: RiskLevel::Low,
            selected_by_default: true,
        },
        ProjectRule {
            id: "node.node_modules".into(),
            ecosystem: Ecosystem::Node,
            markers: vec!["package.json".into()],
            dir_names: vec!["node_modules".into()],
            action: RuleAction::Trash,
            risk: RiskLevel::Medium,
            selected_by_default: false,
        },
        ProjectRule {
            id: "python.pytest_cache".into(),
            ecosystem: Ecosystem::Python,
            markers: vec!["pyproject.toml".into(), "requirements.txt".into()],
            dir_names: vec![".pytest_cache".into()],
            action: RuleAction::Trash,
            risk: RiskLevel::Low,
            selected_by_default: true,
        },
    ]
}
```

---

## 19. 风险分级策略

建议内置：

| 风险        | 定义                                                                      | 默认选中  |
| --------- | ----------------------------------------------------------------------- | ----- |
| Low       | 明确可重建、低价值缓存，例如 `__pycache__`、`.pytest_cache`、`.next/cache`              | 是     |
| Medium    | 可重建但成本较高，例如 `node_modules`、`.venv`、npm/pip cache、Rust `target` fallback | 否或按配置 |
| High      | marker 不充分、可能包含用户产物，例如 generic `build/`、`dist/`                         | 否     |
| Dangerous | 可能包含不可恢复数据，例如 Docker volumes、Cargo home 整体、无 marker 目录                  | 禁止或隐藏 |

Rust `target` 如果用 `cargo clean`，可以是 Low；如果 fallback 为直接 trash，则提升为 Medium。

---

## 20. 测试方案

### 单元测试

覆盖：

```text
规则匹配
marker 识别
路径 canonicalize
父子 target 去重
风险分级
config merge
command plan generation
```

### fixture 集成测试

准备：

```text
fixtures/rust_workspace/
  Cargo.toml
  crates/a/Cargo.toml
  target/debug/foo

fixtures/node_project/
  package.json
  package-lock.json
  node_modules/pkg/index.js
  .next/cache/foo

fixtures/python_project/
  pyproject.toml
  .venv/pyvenv.cfg
  __pycache__/x.pyc
  .pytest_cache/
```

断言：

```text
扫描结果包含应有 target
不会匹配无 marker 的 build/target
不会重复统计父子 target
默认选中只包含 Low
```

### TUI 测试

ratatui 支持 TestBackend，可用于测试 UI 输出而不需要真实终端。官方 backend 文档列出了 TestBackend 用于 UI 单元测试。([Ratatui][19])

建议：

```text
1. 给定 AppState。
2. 用 TestBackend render。
3. snapshot 测试关键 UI。
4. 不测试颜色细节，只测试文字、布局、选中状态。
```

### 平台测试矩阵

```text
Windows PowerShell
Windows Terminal
Linux bash
macOS zsh
Git Bash / MSYS2 可作为附加
```

Windows 必须是第一优先级，因为你的需求重点就在 Windows。

---

## 21. 发布形态

建议三个发布入口：

```text
cargo install rclean
GitHub Releases: rclean-x86_64-pc-windows-msvc.zip
包管理器后续：Scoop / Winget / Homebrew
```

但 MVP 不必先做所有包管理器。先保证：

```text
单二进制
无服务进程
无后台 daemon
配置和日志位置明确
```

---

## 22. 分阶段实施路线

### Phase 0：骨架

交付：

```text
cargo project
clap CLI
ratatui 空 TUI
config loader
tracing log
```

命令：

```bash
rclean tui
rclean scan --json
```

### Phase 1：项目扫描

交付：

```text
ProjectScanner
Rust target 检测
Node node_modules 检测
Python cache 检测
size estimator
dedupe
JSON plan 输出
```

先不执行删除，只输出 plan。

### Phase 2：执行引擎

交付：

```text
MoveToTrash
CommandBacked
dry-run
audit log
failure report
```

先支持：

```text
cargo clean
trash node_modules
trash __pycache__
```

### Phase 3：全局 provider

交付：

```text
npm provider
pip provider
pnpm provider
yarn provider
Cargo home inspect provider
Docker builder provider optional
```

所有 command-backed provider 都必须先展示命令和路径。

### Phase 4：TUI 完整体验

交付：

```text
Dashboard
Global tab
Projects tab
Details panel
Confirm modal
Jobs/Logs tab
Filter/search
Keyboard help
```

### Phase 5：安全增强和发布

交付：

```text
Windows locked-file handling
symlink/reparse point test
snapshot tests
GitHub Actions matrix
release zip
README
```

---

## 23. MVP 验收标准

MVP 可以这样定义：

```text
1. 在 Windows 上扫描 D:\code，正确发现 Rust target、node_modules、Python caches。
2. 能显示每项大小、路径、风险、证据、将执行动作。
3. 默认不删除任何东西。
4. 用户选择后可把 node_modules/.venv/cache 目录移动到回收站。
5. Rust target 优先通过 cargo clean 清理。
6. npm/pip/pnpm/yarn provider 通过官方命令清理，不硬删内部目录。
7. 每次执行生成 audit log。
8. 扫描/清理过程中 TUI 不冻结，可取消。
```

---

## 24. 最重要的设计决策

我建议你强制采用这几个决策：

```text
1. 全局缓存优先 command-backed。
2. 项目目录优先 trash-backed。
3. Rust target 优先 cargo clean。
4. Cargo home 默认 inspect-only。
5. 不尊重 .gitignore 扫描清理目标，因为目标经常在 .gitignore 里。
6. 所有候选项必须有 evidence。
7. cleanup plan 必须可导出 JSON。
8. permanent delete 默认禁用。
```

这会让工具比 Mole-like 简单清理器更可靠，也比 Kondo-like 项目清理器覆盖面更大。

---

## 25. 最小可用版本的推荐功能组合

第一版不要贪多。建议只做：

```text
Project:
  Rust target
  node_modules
  .next/cache
  .turbo
  __pycache__
  .pytest_cache
  .mypy_cache
  .ruff_cache
  .venv

Global:
  npm cache verify / clean
  pip cache purge
  pnpm store prune
  yarn cache clean
  Cargo home inspect
```

第二版再加：

```text
Docker builder cache
Cargo home safe-components clean
Poetry / PDM / uv
Gradle / Maven
Go build cache / module cache
Conda
HuggingFace / torch / transformers cache
```

---

## 26. 结论

技术上完全可行。关键不是 ratatui，而是 **清理语义建模**：

```text
CleanTarget = path/command + size + risk + evidence + reversible + action
```

只要这个模型立住，TUI、CLI、JSON plan、provider 扩展都会自然生长出来。

我会按这个优先级做：

```text
1. ProjectScanner + CleanTarget model
2. Rust target via cargo clean
3. node_modules/.venv/cache trash cleanup
4. npm/pip/pnpm/yarn command-backed global cleanup
5. ratatui TUI
6. Docker/Cargo home 高级清理
```

信心：**中高**。ratatui 和 Rust 生态足够支撑该工具；真正的风险在误删和跨平台路径语义，因此必须把 dry-run、evidence、risk、trash、audit log 作为核心，而不是后期补丁。

[1]: https://docs.rs/ratatui/latest/ratatui/ "ratatui - Rust"
[2]: https://ratatui.rs/concepts/application-patterns/the-elm-architecture/ "The Elm Architecture (TEA) | Ratatui"
[3]: https://docs.rs/clap/latest/clap/_derive/index.html?utm_source=chatgpt.com "clap::_derive - Rust"
[4]: https://docs.rs/directories?utm_source=chatgpt.com "Crate directories - Rust"
[5]: https://serde.rs/?utm_source=chatgpt.com "Overview · Serde"
[6]: https://docs.rs/ignore?utm_source=chatgpt.com "Crate ignore - Rust"
[7]: https://docs.rs/trash?utm_source=chatgpt.com "trash - Rust"
[8]: https://doc.rust-lang.org/cargo/guide/cargo-home.html "Cargo Home - The Cargo Book"
[9]: https://docs.npmjs.com/cli/v8/commands/npm-cache "npm-cache | npm Docs"
[10]: https://pip.pypa.io/en/stable/cli/pip_cache/ "pip cache - pip documentation v26.1.2"
[11]: https://pip.pypa.io/en/stable/topics/caching/ "Caching - pip documentation v26.1.2"
[12]: https://doc.rust-lang.org/cargo/reference/environment-variables.html "Environment Variables - The Cargo Book"
[13]: https://blog.rust-lang.org/2025/06/26/Rust-1.88.0/ "Announcing Rust 1.88.0 | Rust Blog"
[14]: https://doc.rust-lang.org/cargo/commands/cargo-clean.html "cargo clean - The Cargo Book"
[15]: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html?utm_source=chatgpt.com "cargo metadata - The Cargo Book"
[16]: https://pnpm.io/cli/store "pnpm store | pnpm"
[17]: https://yarnpkg.com/cli/cache/clean "yarn cache clean | Yarn"
[18]: https://docs.docker.com/reference/cli/docker/builder/prune/ "docker builder prune | Docker Docs"
[19]: https://ratatui.rs/concepts/backends/ "Backends | Ratatui"
