# 执行计划:父任务最终集成审查

> 父任务不承载功能实现;本文件是四个子任务全部 archive 后、父任务 archive 前的
> 集成审查执行清单。在此之前父任务不进入 `task.py start`。

## 前置条件

- [ ] 四个子任务(core-api-extraction / core-contract-extension /
      tauri-shell-backend / desktop-frontend-ui)均已 archive。
- [ ] 各子任务遗留的设计偏差已回写父任务 `design.md`。

## 审查步骤(按序)

1. **全量质量门**:`just ci` 全绿;确认三平台 CI(windows/ubuntu/macos)与 MSRV
   作业在 workspace 布局下通过。
2. **CLI 回归**:按子任务 1 留档的等价比较方法,复跑
   `devsweep scan --json` fixture 对比;抽查 `clean --plan/--execute` 行为与
   拆分前语义一致。
3. **端到端复核**:按子任务 4 留档的 E2E 步骤走一遍完整流程
   (扫描 → 取消一次 → 重扫 → 勾选 → dry-run 取得 digest → 改选使 digest 失效 →
   重新 dry-run → 执行 → 报告),核对与父任务 prd 验收标准逐条对应。
4. **安全审查**(代码走读,非运行):
   - 执行链路唯一入口 `plan_execute`,必经 validate + digest 校验;
   - Tauri capability 清单最小化,无 fs/shell 插件;
   - 所有结果文案为回收站口径,全局搜索确认无"已释放"表述。
5. **文档与 spec**:确认桌面端构建/开发命令已写入 README 或 docs;
   走查 `.trellis/spec/`(backend 与新建的 desktop-frontend 层)是否收录了
   本任务树产生的新约定(Phase 3.3)。
6. **收尾**:父任务 prd 验收清单逐项勾选 → 提交 → archive 父任务。

## 验证命令

```bash
just ci
# E2E 与 JSON 等价比较命令以子任务 1 / 4 implement.md 留档为准
```

## 回滚点

- 集成审查发现跨层缺陷时:不在父任务内修复;重开或复用对应子任务
  (Trellis 允许从 archive 复制上下文新建修复任务),父任务保持未 archive。
