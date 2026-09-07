# 父任务执行计划（待批准）

## 本轮已完成的准备

- [x] 读取项目地图/关键文件/三层规范，初始 Git clean。
- [x] 强模型主线程 + 两位强模型专家独立只读审查。
- [x] 运行 Rust/desktop/skill/security 现有检查；记录文档失败和忽略/缺口。
- [x] 真实旧 CLI 解析失败；原版 just recipe 的隔离退出码失败注入。
- [x] 核对当前/历史 hosted workflow，当前 HEAD 9 jobs success。
- [x] 建立五子任务，保持 planning；不开始实现。
- [x] 完成六个任务结构校验与强模型独立语义审查；GO 仅表示方案可提交用户批准，尚未授权实施。

## 批准后顺序

1. 复核最新 Git 和该任务树；只激活选定子任务，不把父任务当成产品实施目标。
2. validation-gates → release-contract → skill-integrity 顺序实施，共享 justfile 逐项交接。
3. cli-documentation 可独立进行；双语 current docs 与 release 示例在集成时对照。
4. harness-alignment 汇总最终入口、能力边界、模型分工、实际发现与未运行项。
5. 各子逐条完成 AC；强模型最终复核有界执行者结果。
6. 稳定、批准、经验证的结论回写 design.md 表中的项目说明/skill 源，并注明适用工具。
7. 父集成检查后出验收报告。没有推送/发布/签名/安装授权；不得触发相应操作。

## 最终验收

- 运行经过修复的 just ci，锁文件保持不变；desktop checks、docs build、skill output/check-skills 完整通过。
- 门禁五处失败注入全部非零；类型漂移、skill 漂移和版本变异反例均被检测。
- Windows 新归档 release-smoke 走任务 fixture 的 scan→plan→preview，无真实执行；NSIS 只构建/检查元数据。
- CLI 契约测试与双语指南吻合；地图路径/归档证据定位正确；旧 native PASS 不推广到当前 HEAD。
- 五工具说明逐行有官方及项目证据；没有实际 provider/hook 会话的项目显式 UNVERIFIED，文档对齐通过不等于运行全通过。
- 对未来实际变更运行 git diff --check、任务 validate；只含被批准文件。
- 所有 justfile/CI/发布改动完成后，用全新独占输出目录运行一次 `cargo test --workspace --locked --all-targets --target-dir <fresh-path>`；测试子进程不得继承 `CARGO_TARGET_DIR`。不删除现有 target 来模拟干净构建。
- Windows 与至少一个 Unix 环境分别运行 `cargo test --locked -p devsweep-core --lib process::tests -- --nocapture`；Unix 日志必须包含 `macos_linux_process_group_tree_termination_dynamic` 的实际通过。若无获准的 Unix 环境/对应新提交 CI 证据，保留未完成项，不宣布相关子任务最终完成，不擅自 push 或创建远端环境。
- `git status --short --ignored` + `git check-ignore dist/<本次归档>` 证明新产物被忽略；共享最终门禁覆盖末次相关改动后再完成 gate/release/skill 子，避免重复干净构建但不漏门禁。
- 本轮交付检查仅新增六个任务目录，无产品改动、无 task start/commit/push/archive。（规划快照；批准后的产品提交与五子归档见 git 历史与 `research/parent-release-gates.md`）

## 发布门证据（实施后）

- 五子任务已归档；父任务在五子归档且本文件所列门禁记录完成前保持未归档。
- 命令、退出码与 UNVERIFIED 项：`research/parent-release-gates.md`。
- Unix `macos_linux_process_group_tree_termination_dynamic` 保持 **UNVERIFIED**，不宣布 process-runner Unix 完成。

## 失败处理

结果非零先保存原输出，缩到最小反例。不能以改测试为必绿、自动修锁、刷新类型后掩盖 drift、放宽清理权限或静默换模型消除失败。技术事实可直接核查；新增产品/权限/依赖/全局决定才需另行批准。
