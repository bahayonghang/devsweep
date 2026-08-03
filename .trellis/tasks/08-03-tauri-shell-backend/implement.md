# 执行计划:Tauri 桥接层

## 步骤

1. 脚手架:`cargo tauri init`(或 create-tauri-app)落 `desktop/`;入
   workspace;`cargo tauri dev` 起空窗口。
2. 状态模型 + `scan_start`/`scan_cancel` + `scan://progress` 转发;
   devtools 手工验证事件流,并发拒绝与取消行为写成单测/集成测试
   (core 层可注入慢速 fixture Sweeper)。
3. `CommandError` 枚举与映射层 + 单测。
4. `plan_dry_run` / `plan_execute` + digest 强制校验 + 边界错误用例测试。
5. `protection_list_get/set`。
6. Job Objects 冒烟:fixture 计划含 command-backed 动作,桌面进程内执行,
   对照 CLI 行为,记录到本任务 `research/`。
7. 打包:`cargo tauri build`,安装启动冒烟,产物与记录留档。
8. CI 决策落地(design.md §5)并回写父任务 design.md §5。

## 验证命令

```bash
just ci
cargo tauri dev      # 手工冒烟
cargo tauri build    # 未签名 bundle
cargo test -p devsweep-desktop
```

## review 门

- command 契约(名称/错误枚举/事件负载)在步骤 4 后人工 review,再进入 5-8。

## 回滚点

- 每步独立提交;desktop/ 可整体删除回滚,不影响 core/cli。
