# 常青项目审查报告：DevSweep × 五套 Harness

审查日期：2026-09-07。结论：**适合先修复开发与交付契约，不需要先大改业务代码。**
本轮完成只读源审查、现有测试与失败复现，并创建待批准任务树。产品、项目说明与 skill 副本尚未修改。

## 基线与结构

`dev@ff9f57eeda7e610ee326e81b80bf011e57a76fd0`，初始工作区干净。入口先读 code_map.md，再读 Cargo.toml、justfile、README、CONTEXT、三层 Trellis index、CLI/执行/契约关键路径、桌面 package/生成测试与 CI。

| 层 | 当前职责与关键文件 |
| --- | --- |
| Workspace | Cargo.toml：core + CLI/TUI + desktop Tauri，统一 Rust 版本 0.3.0/MSRV 1.88 |
| CLI/TUI | crates/devsweep-cli/src/application/cli.rs 定义 Clean/Software/Optimize/Analyze/Status/History；modes/ + tui/；cli_contract 与 five_mode_contract |
| Core | model/plan/rules/execution 保持权限边界；scan/inventory/analysis/software/optimize/status/history 为独立业务职责 |
| Desktop | desktop/src-tauri/src/commands.rs + 各模式 IPC；desktop/src/api 解码/生成类型；state/app-shell/modes 处理状态与工作协调 |
| 工程与知识 | justfile、.github/workflows/ci.yml、docs、AGENTS/CLAUDE、skills/devsweep-inspect；.trellis 保存规划与规范 |

安全链的静态审查及既有测试支持继续保持：未信任 plan → registry 重建 → opaque ValidatedPlan → 选择/live digest/确认 → durable audit → 受控 dispatch。program/argv 分离；永久删除关闭、Cargo home inspect-only、Software 仅当前用户 MSIX；Analyze/Status 不创建清理权限。没有据此声称已经穷尽所有安全路径。

## 优先发现

### F1 · P1：桌面门禁可假绿

证据：justfile:127、justfile:128 将五个 npm 检查写在一个 PowerShell 分号链。真实 recipe 的隔离失败注入表明，前四项任一 exit 23，整个入口仍 exit 0；只有最后 build 失败返回非零。
根因：退出状态只由后续/末次原生命令决定。CI 的 .github/workflows/ci.yml:79 多行 npm 同样应拆分或显式检查原生退出码；本轮未触发 hosted 失败注入，不能说当前远端真的曾被该问题误绿。
改造：validation-gates 子任务；逐项失败传播，生成漂移在覆盖前校验，文档构建可见。

### F2 · P1：CLI 文档、开发启动与发布冒烟仍使用旧契约

证据：README.md:28、README.md:34、README.md:47；justfile:113、justfile:117、justfile:122；AGENTS.md:11。旧 scan/tui/clean --plan 的本地实测全部 exit 2，当前命令定义在 crates/devsweep-cli/src/application/cli.rs:40 和 :63。
同类现行说明散布于 docs/guide/getting-started.md、scan.md、safety-model.md、tui.md、docs/reference/plan-and-report.md 及 zh 镜像。它们还讲旧 --audit-log 与旧存储，和 docs/guide/clean.md:28 的固定 V1 store 相冲突。
根因：五模式 breaking migration 已更新解析器及部分页面，但旧引导/脚本没同步。当前解析器 fail-closed，没有现时数据损失证据。
改造：cli-documentation + release-contract；不恢复旧别名，不用仅 --help 替代完整非破坏性发布冒烟。

### F3 · P1：本地 canonical ci 隐式更新锁文件

证据：justfile:11 的 sync-lock 执行 cargo update --offline；justfile:160 的 ci 依赖它。Hosted check/test 使用 --locked。
根因：依赖维护与检查混在同一入口，本地通过可能依赖先行修复锁状态。
改造：validation-gates 分离显式 sync-lock；验证标准 gate 前后锁哈希不变。本轮按四个既有子 gate 完成实际检查，没有调用 mutating aggregate。

### F4 · P1：实际发现的 skill 副本与规范源指令相反

证据：skills/devsweep-inspect/SKILL.md:10、:40 要求 PATH 全局 binary，禁止仓库 cargo run；.agents/skills/devsweep-inspect/SKILL.md:35 的 v1 副本却推荐仓库 cargo run；.claude 同样过期。源 v2，16 个同名文件不同，7 个源文件未分发。Grok inspect 已确认其实际发现 stale .agents 副本；主线程逐文件复核见 skill-distribution-snapshot.json。
根因：justfile:30 的安装流程只复制，缺少可独立失败的同步完整性检查；发现目录被 .gitignore:1 忽略，源的提交不更新本地副本。
改造：skill-integrity；保持两个已证实共享发现根，不为五个工具复制五份业务正文。暂无数据损失证据，因此不是 P0。

### F5 · P2：桌面/安装包版本落后于 workspace

Cargo.toml:6 = 0.3.0；desktop/package.json:4、desktop/package-lock.json:3/:9、desktop/src-tauri/tauri.conf.json:4 = 0.2.0。
改造：release-contract 对齐当前 workspace 版本并在一个可信入口检查元数据一致；本轮没有构建新安装器，安装器实际版本资源尚未验证。

### F6 · P2：五套工具的说明、发现与运行证据混在一起

CLAUDE.md:1 正确引用 AGENTS.md；AGENTS.md:58 专谈 Codex hooks。Trellis workflow 描述多平台，但项目平台识别仅 Claude/Codex/Reasonix；没有 Grok/Kimi/OMP 原生目录。Grok 的兼容 discovery 实际可用，不能由目录缺失推断不可用。Kimi/OMP 共享入口有官方支持，但本项目实际模型会话未验证。
共享规则还有两项具体缺口：AGENTS.md:19 暗示 `--allow-permanent-delete` 可传入，当前 parser 与 backend 规范却明确不暴露该 flag；AGENTS.md:60 只路由 backend/TUI，遗漏独立的 `.trellis/spec/desktop-frontend/index.md`。harness 子明确修正二者，保持永久删除禁用且不引入旧旗标兼容。
改造：harness-alignment，明确官方能力/文件/发现/执行四级；保持 Trellis 0.6.12，不自动用本机 0.7.0-beta.3 升级。详见 harness-matrix.md。

### F7 · P2：导航与证据路径不再代表当前仓库

code_map.md:73 指向已不存在的 application/commands.rs，并缺五模式新增核心模块；docs/validation/five-mode-native.md:62 仍指活动任务，真实数据已在 archive/2026-09/08-29-five-mode-native-integration。docs/ci.md 的 jobs 表还遗漏 Desktop。
改造：cli-documentation 更新地图/证据入口，validation-gates 更新 jobs 说明。历史证据只证明其记录的 commit。

## 测试结论与失败分类

- PASS：Rust 568；Desktop 181（31 文件）；skill fixtures 4；fmt/check/clippy；cargo audit 和 deny；当前 HEAD 的 [9 项 hosted CI](https://github.com/bahayonghang/devsweep/actions/runs/33588352167)。
- FAIL：docs:build（本地缺 VitePress），3 条旧命令（exit 2），desktop gate 失败注入揭示错误 exit 0。
- 非故障：3 个 ignored Rust 入口；18 allowed audit warnings 单列，不升级为已确认漏洞利用。
- 历史 CI 失败已定位到环境/CRLF、平台预期、lint、许可证、误纳 browser profile，已有修复且当前 HEAD PASS。详见 test-results.md。

文档缺依赖属于环境前置失败，本轮没有擅自 npm ci。原生交互/安装/真实危险操作、未运行 provider/hooks 保持 UNVERIFIED。

## 可批准的改造范围

| 顺序 | 子任务 | 范围 | 必须通过 |
| --- | --- | --- | --- |
| 1 · P1 | 09-07-evergreen-validation-gates | justfile 的 gate、CI、双语 CI 说明、隔离回归 | 五处失败全部非零；正常 gate 通过；锁不变；类型漂移能失败；docs 构建 |
| 2 · P1 | 09-07-evergreen-cli-documentation | README、中英文当前指南、地图、归档证据链接 | CLI 契约测试；非破坏示例；docs build；逐条链接与语义审查 |
| 3 · P1 | 09-07-evergreen-release-contract | dev/release 配方、桌面版本/锁元数据、版本回归 | 精确新归档 smoke；版本一致/反例；构建安装包但不安装 |
| 4 · P1 | 09-07-evergreen-skill-integrity | install/check skill、源说明、两份 ignored 本地副本 | 文件集合/哈希一致；漂移与越界反例；4 output fixtures |
| 5 · P2 | 09-07-evergreen-harness-alignment | AGENTS、CLAUDE 引用、五工具指南 | managed block 保真；逐工具 discovery 或明确缺口；强/低成本分工 |

前三个涉及 justfile 的实现（gate → release → skill）严格顺序，docs 可以并行；harness 最终验收在文档与 skill 后。每个子任务已有 PRD/design/implement 和明确文件责任。

## 回写与审批边界

批准后才修改项目说明/skill 分发。稳定结论写回 AGENTS、docs/agents/harnesses.md、双语 CI/CLI 指南与 skill README，并按工具标注适用范围；不把尚待批准计划写入团队知识库或用户全局记忆。无需引入新业务模块、通用平台、依赖或历史兼容层。

本轮只新增本父子任务规划材料和测试证据；所有新任务保持 planning，无 task.py start、提交、发布、升级、安装或清理行为。
