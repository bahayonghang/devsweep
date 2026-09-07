# 规范加载与注入截断

本轮 task validate 发现 `.trellis/spec/backend/quality-guidelines.md` 为 54377 字节，超过当前注入单文件 32768 字节上限。文件后段的测试约束可能被截掉，因此 manifests 注入较短 backend/index.md 与本说明；不提高全局上限、不改规范文件来掩盖问题。

批准实施后，后端/发布/CLI 相关执行者须先定位 quality-guidelines.md 的标题，再按相关标题完整读取（必要时分 150–200 行块）；至少完整读取 Forbidden Patterns、Five-mode CLI 场景、CI and release archive 场景（当前 736–849 行，含 clean/custom target 与 Windows/Unix 动态 process-runner 必需检查）和文末 Testing Requirements/Review Checklist。用 `rg -n '^#{1,4} ' .trellis/spec/backend/quality-guidelines.md` 定位，不能仅依赖被截断的注入内容。遇 JSON/执行/扫描变更时再读对应场景全文；本计划不授权这些业务语义变化。

相关已核实约束：旧根拒绝、machine output locale-neutral、执行需要 plan/live digest/confirm、scanner/plan 无执行副作用、program/argv 分离、永久删除关闭；修改命令表面要实际核对 help/CLI 契约，JSON 变更需对应序列化测试。此摘要是加载路由，不取代原规范。

前端任务读取 desktop-frontend/index.md 及其指向的三份指南。文档/skill/harness 子任务不需要把整个 Rust 质量规范塞进上下文；按它实际涉及的契约读取。
