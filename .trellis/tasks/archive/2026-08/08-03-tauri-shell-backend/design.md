# 技术设计:Tauri 桥接层

> 细化父任务 `design.md` §3。契约以 `devsweep-core`(经子任务 2 扩展)为单一事实源。

## 1. 目录与集成

- `desktop/src-tauri`:crate 名 `devsweep-desktop`,入 workspace members;
  `desktop/src`:Vite + React + TS 占位页(真实 UI 在子任务 4)。
- Tauri 2 默认模板起步;仅注册本设计列出的 command,不引入官方 fs/shell/dialog
  插件(后续 UI 需要文件选择器时由子任务 4 评估 dialog 插件并更新 capability)。

## 2. 状态与并发模型

```rust
struct ScanState {                       // tauri::State<Mutex<…>>
    running: Option<ScanHandle>,         // cancel flag + join handle
}
```

- `scan_start`:锁内检查 running → 占位 → `spawn_blocking` 跑
  `Sweeper::full_scan_report`,进度回调经 `std::sync::mpsc` →
  异步任务转发 `app.emit("scan://progress", …)`;结束清 running。
- 并发第二次调用:立即返回 `Err(CommandError::ScanAlreadyRunning)`。
- `scan_cancel`:置 `FlagCancelObserver`;幂等,无扫描时为 no-op 成功。

## 3. 错误契约

统一 `CommandError`(serde 枚举,tagged):`ScanAlreadyRunning` /
`ScanFailed{message}` / `InvalidPlan{issues}` / `StaleConfirmation` /
`UnknownTarget{id}` / `InspectOnlyTarget{id}` / `Io{message}`。
core 错误 → CommandError 的映射集中一处;前端只见结构化枚举。

## 4. dry-run / execute

- 两者共用"validate → 构造 ExecutionRequest"前置;`plan_dry_run` 用
  `execute:false` + 计算 `confirmation_digest` 一并返回;
- `plan_execute` 要求 `digest` 参数,重算比对,不匹配 → `StaleConfirmation`;
  `expected_digest` 同时传入 core(双保险,以 core 校验为准)。
- audit log:桌面端默认写入 app data 目录下 `audit/` JSONL,路径通过
  command 返回值透出(为子任务 4 的 history 迭代留钩子,MVP 不做 UI)。

## 5. 打包与 CI(决策项,结果回写父任务 §5)

- 本地:`cargo tauri build` → NSIS 未签名 bundle;安装/启动冒烟留档。
- CI 建议基线(实施时定稿):Windows job 加 Node LTS setup + `npm ci` +
  `cargo tauri build --no-bundle` 作为编译门;Linux 不装 webkitgtk、桌面 crate
  用 `cargo check -p devsweep-desktop` 降级门或直接排除,记录原因。
- lockfile:`desktop/package-lock.json` 独立提交;Node 版本写入
  `desktop/package.json` engines 字段。

## 6. 回滚

- desktop/ 整目录纯新增,可整体删除;workspace members 去掉一行即回滚。
