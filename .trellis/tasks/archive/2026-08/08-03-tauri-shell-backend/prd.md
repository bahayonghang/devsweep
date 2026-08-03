# Tauri 应用骨架与后端命令桥接

> 父任务:`08-03-tauri-desktop-app`(总体设计见父任务 `design.md` §3、§5)。
> 依赖:`08-03-core-contract-extension` 必须先完成(需要 serde 契约、
> 逐目标明细与 digest API)。

## Goal

在 `desktop/` 下搭建 Tauri 2 应用骨架,实现后端 command / event 桥接层,
并交付未签名 Windows 打包验证与 CI 决策。

## Requirements

- 脚手架:Tauri 2 + React + TS + Vite;`desktop/src-tauri` 作为 workspace 成员,
  依赖 `devsweep-core`,不依赖 CLI crate。
- Commands(契约细节见父任务 `design.md` §3,含 selected_ids 四条边界):
  - `scan_start`:`spawn_blocking` + `scan://progress` event;并发时第二个调用
    返回结构化错误 `scan_already_running`(固定为拒绝,不排队);
  - `scan_cancel`:触发 `FlagCancelObserver`;
  - `plan_dry_run`:返回 `{ report, digest }`;
  - `plan_execute`:必经 validate + digest 校验,无免 digest 执行路径;
  - `protection_list_get` / `protection_list_set`。
- 安全:不启用 fs / shell 前端插件;capability 最小化;CSP 默认严格。
- IPC 载荷直接用 core serde 模型,不建 DTO 层。
- 冒烟验证 `process` 模块(Win32 Job Objects)在 Tauri 进程内执行 command-backed
  清理动作与 CLI 行为一致(feasibility 风险表)。
- 构建与 CI(本任务所有,决策回写父任务 `design.md` §5):
  - `cargo tauri build` 产出未签名 Windows bundle(NSIS 或 MSI)并可安装启动;
  - 决策并落地:Tauri 构建是否入 CI、Node/npm 版本门禁方式、前端 lockfile
    布局、Ubuntu runner webkitgtk 依赖策略(若决定 Linux 不入 CI,记录原因)。

## Acceptance Criteria

- [x] `cargo tauri dev` 可启动应用窗口(占位前端,本任务不做 UI)。
- [x] devtools 调用 `scan_start` 收到 `scan://progress` 事件流并返回完整
      `ScanReport`;并发第二次调用得到 `scan_already_running` 错误(有自动化
      测试或留档的手工验证脚本)。
- [x] 合成 fixture(复用子任务 1 的 fixture)上:扫描中调用 `scan_cancel`,
      最迟在下一次进度回调边界停止且不超过 5 秒,返回取消状态或部分结果。
- [x] `plan_dry_run` → `plan_execute` 闭环:正确 digest 放行;修改选择集后旧
      digest 得到 `stale_confirmation`;未知 id / inspect-only 选中得到对应
      结构化错误(各有测试)。
- [x] dry-run 与 execute 的 `ExecutionReport` 与 CLI 对同一计划的行为语义一致
      (fixture 对照,留档)。
- [x] 未签名 Windows bundle 构建成功且能安装启动(留档产物路径与验证记录)。
- [x] `just ci` 全绿;command 层有最小单元测试;CI 决策已落地并回写父任务。

## Notes

- 界面交互属于 `08-03-desktop-frontend-ui`;本任务前端仅为调试占位页。
- 完成后解锁 `08-03-desktop-frontend-ui`。
