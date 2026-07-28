# 许可证基线：LICENSE、Cargo metadata 与设计 provenance

> 父任务：07-28-audit-remediation ｜ 审计条目：F-10（P1 发布/法律阻断） ｜ 对应审计 Milestone M1
> 二轮评审 CORR-002/ARCH-004 拆分：F-10 从原 release-supply-chain（P2）拆出为独立 P1；CI/发布工件归 07-28-release-supply-chain。

## Goal

关闭公开分发的法律缺口：确定并提交 LICENSE、补全 Cargo 分发 metadata、记录对标 Mole（GPL-3.0）的设计 provenance，使"公开安装、社区贡献、参考 Mole 补齐"不再是阻断项。

## 背景与证据（当前 HEAD）

1. 仓库无 `LICENSE`/`COPYING`；GitHub 未识别 license。无许可证时外部用户默认不获得使用/修改/再分发授权（F-10）。
2. `Cargo.toml:1-5` 缺 `license`、`repository`、`readme`、`rust-version`。
3. 对标对象 Mole 为 GPL-3.0（`Mole/LICENSE:1-18`）：参考其公开行为、问题定义与安全原则可以；直接复制/轻改其源码、测试、规则表、文案可能触发衍生作品义务。目前整改 PRD 均基于审计报告的不变量描述而非 Mole 实现，需要正式记录这一 provenance 立场。

## Requirements

1. **许可证决策（决策 D4，待维护者拍板）**：建议 `MIT OR Apache-2.0` 双许可（Rust 生态惯例）；决策后提交 LICENSE 文件（双许可则 LICENSE-MIT + LICENSE-APACHE）。
2. `Cargo.toml` 补 `license`、`repository`、`readme`、`rust-version`（MSRV 值与 release-supply-chain 的 CI MSRV job 保持一致）。
3. 设计 provenance 记录：在 docs（如 `docs/provenance.md` 或 design.md 附注）声明"对标 Mole 仅借鉴不变量与验收方法，实现独立完成，未复制其源码/fixtures/文案"，为后续正式 license review 留证。
4. README 增补 License 章节。

## Acceptance Criteria

- [ ] 仓库根存在 LICENSE 文件，GitHub 识别出 license
- [ ] `cargo package --list` 无 license/repository/readme/rust-version 相关告警
- [ ] provenance 声明已入库并被父任务集成评审引用
- [ ] README 含 License 章节
- [ ] 本任务不改任何 `src/` 代码（纯文档/元数据变更，天然低风险）

## 约束与依赖

- 唯一外部输入是维护者的许可证选择（决策 D4）；除此之外无依赖，可立即执行。
- 本任务为 lightweight（PRD-only 即可启动，无需 design.md/implement.md）。
- 本 PRD 不构成法律意见；若未来引入 Mole 衍生内容，须重新评估 GPL 义务。
