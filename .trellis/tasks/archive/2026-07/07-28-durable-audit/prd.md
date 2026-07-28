# Durable audit：action 边界持久化与记录权威

> 父任务：07-28-audit-remediation ｜ 审计条目：F-08、F-22 ｜ 对应审计 Milestone M1 / PR10

## Goal

把 audit 从"job 末尾才 flush 的 best-effort 日志"改成 action 前后的 durable journal：副作用发生前 `action_started` 已在稳定介质上，之后 `action_finished` 记录结果与真实执行权威（canonical action path、cwd、resolved executable、exit code、run/sequence、plan digest），使崩溃/盘满/断电后可以回答"到底动了什么"。

## 背景与证据（当前 HEAD）

1. **副作用先于持久化（F-08）**：action 在 `src/executor.rs:211` 先执行，outcome 之后才写入 `BufWriter`；`AuditLog::write` 每条不 flush（`executor.rs:332-339`），整个 job 只在结尾 flush 一次（`executor.rs:251`）。崩溃或盘满时"目录已移动、audit 为空"。
2. **写失败丢整份报告（F-08）**：`audit.write(...)?`（`executor.rs:218,223,237`）失败直接抛出，已完成 target 的 `ExecutionReport` 全部丢失。
3. **记录的不是执行权威（F-08/F-22）**：`AuditRecord`（`executor.rs:348-359`）没有真实 `action.path`、cwd、run ID、序号、plan digest、resolved executable、exit code、actual bytes；`MoveToTrash` 只能从可伪造的 `target_id` 猜路径。
4. **文件安全（F-22）**：默认写 cwd 下 `devsweep-audit.jsonl`（`executor.rs:193-195`；TUI 经 `runtime.rs:195` 传 `None` 同样落 cwd）；无私有权限、no-follow、writer lock；并发 append 可损坏 JSONL。

## 持久化保证等级与故障矩阵（决策 D2，二轮评审 CORR-001 修正）

flush 只保证进入 OS 缓存，不等于稳定介质持久化。默认保证等级定义如下（design.md 可加更高档位，不得降低默认）：

| 记录 | 默认保证 | 语义 |
|---|---|---|
| `action_started` | write + flush + **`sync_data` 落盘成功之后**才允许执行副作用 | "断电后凡已发生的副作用必有 started 记录"成立的唯一方式 |
| `action_finished` | write + flush，`sync_data` 尽力而为 | 结果记录 |

故障矩阵（全部需有故障注入测试）：

| 故障点 | 必须行为 |
|---|---|
| `started` 的 write/flush/sync 任一失败 | **不执行该副作用**；该 target 记为 audit-blocked；不再启动任何新 action；已完成部分以结构化 report 返回 |
| 副作用已执行、`finished` 持久化失败 | 该 target outcome 标记 **unknown**（不得谎报 success/fail）；停止后续 action；UI 明示"action 结果已知 / audit 持久化失败"两种情形的区别 |
| 崩溃发生在 started 与 finished 之间 | 重放 audit 能识别"已开始未确认"的 action 并提示人工核对 |

## Requirements

1. 实现上表写入协议与故障矩阵；`Canceled`/`Skipped` 也有终结记录。
2. record 字段：run ID、序号、plan digest（消费 plan-validation 的 canonical digest 契约）、真实 canonical action path、cwd、resolved executable、exit code、estimated/actual bytes、sanitized error（消费 ProcessRunner core 的净化函数）。
3. 默认位置改为 app-data 私有目录（`--audit-log` 可覆盖）；打开时 no-follow、用户私有权限、writer lock（同机第二实例拒绝或排队）。
4. audit path 注册进 SafetyPolicy 的 ProtectedSubtree（不可被清理目标包含）。
5. TUI 与 CLI 共用同一 audit 通道与默认位置。

## Acceptance Criteria

- [ ] 故障注入：`started` 在第 N 个 target 上 write/flush/sync 分别失败 —— 第 N 个副作用未执行、runner 未调用，前 N-1 个 target 在 report 与 audit 中完整
- [ ] 故障注入：`finished` 持久化失败 —— outcome = unknown，后续 action 不启动，UI 文案区分两种失败
- [ ] 崩溃模拟（started 与 finished 之间中断）：重放识别"已开始未确认"
- [ ] 每条 record 含 run/seq/digest/canonical path/exit code；`MoveToTrash` 记录实际传给 trash runner 的路径
- [ ] 并发写测试：writer lock 生效，JSONL 无交错损坏
- [ ] 默认 audit 文件位于 app-data 私有目录且权限正确（Windows ACL / Unix 0600 等价）；symlink 替换 audit path 被拒绝
- [ ] README 的 "auditable" 声明与实际保证等级一致（文档同步）

## 约束与依赖

- 依赖 07-28-plan-validation（digest 契约）；sanitized stderr 消费 07-28-process-runner-cancellation 的净化函数（接口先行约定即可并行）。
- `sync_data` 每 action 一次的性能代价可接受（action 本身是重量级 Trash/命令）；如需豁免档位只能显式配置降级，默认不降。
- 审计预估 3–5 日。
