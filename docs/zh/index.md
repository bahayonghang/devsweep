---
layout: home

hero:
  name: DevSweep
  text: 安全优先的开发环境清理
  tagline: 先检查磁盘占用，再生成可审计计划；仅在明确复核后执行。
  actions:
    - theme: brand
      text: 快速开始
      link: /zh/guide/getting-started
    - theme: alt
      text: 查看命令参考
      link: /zh/reference/cli

features:
  - title: 先计划，再清理
    details: 扫描会生成带有目标证据、风险、大小完整性和类型化清理意图的版本化报告。
  - title: 默认拒绝不确定操作
    details: 执行前会验证计划，并依据 DevSweep 内置规则目录重建操作。
  - title: 全程可检查
    details: 盘点、规则清单、演练执行和审计日志让每一步清理决定都可见。
---

## 常规工作流

1. 对项目、全局提供方或两者执行[扫描](/zh/guide/scan)。
2. 复核生成计划中的健康诊断和大小估算边界。
3. 先对保存的计划运行不带 `--execute` 的[`clean`](/zh/guide/clean)。
4. 仅在已确认保存计划及默认选中目标后添加 `--execute`。

DevSweep 有意采取保守策略。它不会永久删除文件，不会清理 Docker，并将
Cargo home 视为仅检查位置。执行前请先阅读[安全模型](/zh/guide/safety-model)。

## 切换语言

本站根路径为英文。可使用导航栏中的语言菜单切换回[英文文档](/)。
