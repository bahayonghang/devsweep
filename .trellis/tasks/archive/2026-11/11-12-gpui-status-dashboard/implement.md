# 实施计划

开始前读：`.trellis/spec/backend/index.md`、`bounded-delivery.md`、`quality-guidelines.md` 里与 Status 和数据契约有关的部分；`.trellis/spec/guides/cross-layer-thinking-guide.md`；本任务 `design.md`。

## 步骤

### A. core 与 CLI（可先于界面进行）

1. [x] 调研并写 `research/collectors.md`：每个新指标的系统接口、是否需要提权、失败时的原因码；V1 增量还是 V2 的结论。
2. [x] 扩展 `status/mod.rs` 的模型和 `sampler.rs`、`system.rs`、`network.rs`、`pdh.rs` 的采集。每项补"有值"和"不可用"两类测试。
3. [x] CLI：`commands/status.rs` 的人类可读输出与双语文案；`cli_contract.rs`、`five_mode_contract.rs` 的契约断言。
4. [x] Tauri：更新 `desktop/src/api/contract.ts`、生成类型和夹具，跑 `desktop` 的测试。
5. [x] 跑 `tools/status-sink-smoke.py` 和 `tools/summarize-native-status.py`。
   回滚点 A：后端新增字段独立可用。

### B. GPUI（在 airy-glass-shell 之后）

6. [x] `status_panel.rs`：环形缓冲和测试。
7. [x] `shell.rs`：可见即采样、不可见即停；`main.rs:62` 的托盘可见位改为假；更新相关测试。
8. [x] `page_status.rs`：指标格、趋势绘制、健康格、进程表列。
9. [x] 文案：新指标和不可用原因的中英文。
   回滚点 B：界面改动。

## 验证

```powershell
cargo test --locked -p devsweep-core status
cargo test --locked -p devsweep-cli --test cli_contract
cargo test --locked -p devsweep-cli --test five_mode_contract
just ci
just gpui
```

- AC4、AC5：在 GPUI 测试里用现有的订阅计数（`status_subscription_starts`、`status_subscription_stops`）断言；再实机观察一次。
- AC6：用不可用的夹具快照渲染，核对文字。
- AC7：沿用 `11-04-gpui-airy-glass/evidence/validation.md` 的空闲测法，记录前后数值。
- 原生截图：英文与中文各一组，存入 `evidence/`。

## 评审关口

- 步骤 1 的调研结论（哪些指标能拿到、版本策略）先给用户过目，再实现步骤 2。
