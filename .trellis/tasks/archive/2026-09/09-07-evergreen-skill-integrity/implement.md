# P1 保证 Skill 源与发现副本一致 — Implementation plan

## Approval and sequence

在 validation-gates、release-contract 之后修改 justfile。harness-alignment 的最终发现矩阵依赖本任务修复，规则文案可以先准备。

1. 取得本父子任务最新方案的明确实施批准后，复核 Git/任务状态与本任务证据；仅激活本子任务。
2. 强模型确认 `design.md` 文件范围、事实和改造机制，分配独占文件责任。
3. 先保存最小失败证据/反例，再完成设计中的最小改动；不扩大权限或加入兼容旧根。
4. 按下面命令运行适用检查，保留命令、环境、退出码和未验证项。
5. 强模型独立核验每条 AC 的每个子句和回写；若低成本模型遇到跨模块语义/权限/两次失败，升级给主审，不继续堆修补。
6. 形成验收记录供父任务汇总。本方案不授权 push、安装应用、签名、发布或额外知识库写入。

## Required checks

- python -m unittest discover -s tools/tests -p test_skill_distribution.py
- just check-skills（旧副本应失败）；just install-skill；just check-skills（两份恢复一致）
- python -X utf8 skills/devsweep-inspect/scripts/output_eval.py skills/devsweep-inspect
- 重跑只读 Grok inspect discovery（若可用），核对 resolved path 对应新副本；Codex/Claude/Kimi/OMP fresh-session 检查按 harness 子任务记录

## Final evidence

本任务修改 justfile，因此还必须满足父 implement.md 中从 backend CI/release 场景继承的最终共享门禁：clean/custom --target-dir workspace 测试、Windows/Unix process-runner（Unix 动态树终止）、生成 dist ignored 验证。父集成在最后一次 justfile 改动后集中运行一次；该证据齐全前不把本任务完整完成/归档。测试进程不继承 CARGO_TARGET_DIR，不以当前基线 hosted PASS 替代新代码证据。

记录 AC → 文件/行为 → 命令/输出映射。通过结构校验不等于实现验收；对 native/provider 等未运行项保留 UNVERIFIED，不能靠删除或弱化 AC 过关。
