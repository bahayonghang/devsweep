---
layout: home

hero:
  name: DevSweep
  text: 安全优先的开发环境清理
  tagline: 先检查磁盘占用并保存观察结果，再生成可审计计划；仅在实时预览摘要与 --confirm 之后执行。
  actions:
    - theme: brand
      text: 快速开始
      link: /zh/guide/getting-started
    - theme: alt
      text: 查看命令参考
      link: /zh/reference/cli

features:
  - title: 先计划，再清理
    details: 扫描会生成带有目标证据、风险、大小完整性和类型化清理意图的版本化观察结果。
  - title: 默认拒绝不确定操作
    details: 执行前会验证新保存的计划，并依据 DevSweep 内置规则目录重建操作。
  - title: 全程可检查
    details: 预览摘要、嵌套的保护与规则命令，以及固定的 V1 审计存储让每一步清理决定都可见。
---

## 常规工作流

1. 对项目、全局提供方或两者执行[扫描](/zh/guide/scan)，并**保存观察结果**。
2. 按精确目标 ID 选择，并保存一份**新**计划。观察 JSON 不是可运行计划。
3. 运行 [`clean preview`](/zh/guide/clean) 取得实时摘要。预览就是演练。
4. 仅在该实时摘要与 `--confirm` 同时具备时才执行。

DevSweep 有意采取保守策略。它不会永久删除文件，不会清理 Docker，并将
Cargo home 视为仅检查位置。执行前请先阅读[安全模型](/zh/guide/safety-model)。

## 切换语言

本站根路径为英文。可使用导航栏中的语言菜单切换回[英文文档](/)。
