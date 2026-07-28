# 扫描遍历预算、剪枝修正与排名语义

> 父任务：07-28-audit-remediation ｜ 审计条目：F-12、F-18、F-19 ｜ 对应审计 Milestone M2
> 二轮评审 ARCH-004 拆分：P1 错误模型归 07-28-scan-reliability；本任务承接遍历性能/预算与排名产品决策。

## Goal

为 discovery 与 size 遍历加预算与正确剪枝（深树/大扇出/网络盘不再挂死或跑偏），修正 `cache` 误剪枝与 VCS 目录漏剪枝，并落地排名语义决策（D3）。

## 背景与证据（当前 HEAD）

1. **size walker 无预算且语义粗糙（F-12）**：`estimate_tree` 每层 `collect` + Rayon 递归（`src/fs_size.rs:27-32`），无 depth/entry/deadline/cancel 预算；logical bytes 重复统计 hardlink、夸大 sparse file；深树/巨目录可长时间占满线程池。
2. **剪枝错误（F-18）**：`should_stop_descent` 把任意 basename `cache` 一律剪枝（`src/scanner.rs:351-369`）——名为 `cache` 的目录下的真实项目会被漏掉；反而不跳过 `.git/.hg/.svn`，白白遍历海量对象；无 max depth/deadline/mount boundary。
3. **排名语义含混（F-19）**：定义了 size×age 的 `target_score`（`src/ranking.rs:15-22`），但排序先按 bytes 降序、score 仅作同 bytes 的 tie-breaker（`ranking.rs:66-77`）。经查 `design.md` 无任何排名意图记载——语义决策无据可依，须显式定案。

## Requirements

1. 迭代式 bounded walker：max depth、max entries、root deadline、cancellation 检查点（消费 true-cancellation 的 token 接口；该任务未落地前接 no-op token）；size 按顶层 target 并行，不再逐层创建 Rayon 任务。
2. 遍历预算触发时产出 diagnostic（消费 scan-reliability 的 `ScanOutcome`/`SizeEstimate` 契约，标记 incomplete），不静默截断。
3. 剪枝修正：`.git/.hg/.svn` 无条件跳过；`cache` 仅在已被识别为 cleanup target 时剪枝，不再按裸名字全局剪。
4. hardlink/sparse 的已知偏差在文档与 estimate warnings 中明示（allocated bytes 支持可作后续扩展，不在本任务强制）。
5. **排名语义定案（决策 D3）**：默认采纳"保持 size-first 主序，`target_score` 降级为显式命名的 freshness tiebreaker 或删除"；终案连同理由记入本任务 design.md，测试与文档同步。

## Acceptance Criteria

- [ ] fixture：`cache/` 目录下的 Cargo/Node 项目能被发现；`.git` 内不再产生遍历（可用计数 probe 验证）
- [ ] 深树/大扇出 fixture（合成 10k+ entries）在预算内完成并报告截断 diagnostic；无预算前会挂住的用例作为 fail-red
- [ ] size 与 discovery 的预算触发均反映为 incomplete，不出现"看似完整的部分值"
- [ ] 排名单元测试与所选语义一致；`target_score` 要么改名/文档化为 tiebreaker，要么删除——不存在"定义主 score 却只在同 bytes 生效"的隐藏行为
- [ ] `just ci` 全绿

## 约束与依赖

- 依赖 07-28-scan-reliability 的错误模型契约先落地（incomplete/diagnostic 的承载结构）。
- cancellation token 接口由 07-28-true-cancellation 拥有；本任务先以 no-op token 接口预留。
- 性能 SLO（审计 §7.3：10k entry p95 < 1 s 等）作为基准目标记录，实测在三平台 CI/本机分别标注；达不到不阻断合入，但须记录差距。
