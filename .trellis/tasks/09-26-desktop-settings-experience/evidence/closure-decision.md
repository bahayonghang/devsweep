# 提交与归档决定

记录时间：2026-09-27T23:27:01.365098-05:00

## 用户指令

用户在收到核心 CI 受阻和真实 Windows 输入法候选窗口未验收的报告后，明确要求：

> 请提交改动和归档任务

本次提交范围为 desktop-settings-experience 父任务及 Navigation、Controls、Host Fonts、Palettes 四个子任务。使用本地 Git 提交，随后归档上述五个任务并记录项目开发日志。其他活动任务保持原状态。

## 验证状态

- 实现、独立代码复审、前端 387 项测试及 Node 5 项测试通过。
- 桌面 Rust 83 项测试通过，2 个测试入口按设计忽略；Clippy 和 Windows release/NSIS 构建通过。
- 原生字体目录、持久化、重启、双语导航和输入法渲染器事件检查通过。矩阵覆盖 44 个页面、156 个弹层和 28 个媒体条件。
- canonical just ci 仍为 BLOCKED。核心测试可执行文件缺失；独立目录构建也复现文件缺失，原因未查明。
- 真实 Windows 输入法候选窗口未获得人工验收结果。原生高对比度与系统缩放交互未以 Windows 设置切换方式验证；报告保留渲染器模拟边界。

## 归档语义

按用户指令关闭并归档任务。Trellis archive 命令将任务状态设为 completed；meta.closure 明确保留 all_acceptance_checks_passed=false、canonical_ci=blocked 和 manual_windows_ime=unverified。历史测试结果和独立审查报告保持原结论。

用户归档指令允许结束本组任务流程。归档操作不把未执行或失败的验证记录改为通过，也不构成发布验证完成。后续如继续诊断核心测试程序，应从 core-gate-diagnostic.md 的现有证据开始。

## 提交顺序

1. 功能、回归测试、依赖锁文件、规范与代码地图。
2. 五个任务的规划、实施、审查和验证记录，包括本决定。
3. 按顺序归档四个子任务，再归档父任务。
4. 记录本次项目开发日志。

## 工作提交

功能提交：5e73a4fb2e329c305d400efbb7670684f998d709
