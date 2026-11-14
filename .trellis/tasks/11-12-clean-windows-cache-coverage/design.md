# 设计：Windows 缓存家族扩展

只改 core 规则与 CLI 呈现。扫描层只产生计划，这一点不变。

## 结构

- 新家族加入 `WINDOWS_CACHE_RULES`（`crates/devsweep-core/src/rules/definitions.rs:5-30`），实现沿用 `rules/windows_caches.rs` 的形状：家族枚举值、根目录解析、封闭的子目录名单、所有者进程名（`owner_names`，`:139`）、运行状态检查（`owner_state`，`:851`）。
- Chromium 系的预发布通道和衍生浏览器共用 Chrome 的缓存子目录名单，只换用户数据根目录和进程名。先确认现有 Chrome 实现能否参数化；能就复用，不复制一份。
- 开发工具（基于 VS Code 的编辑器）同理复用 VS Code 与 Cursor 的名单。
- 每个家族的目录证据写入 `research/families.md`：安装后的实际目录、哪些子目录是缓存、哪些必须保留、可再生的依据（厂商文档或实测重建）。没有证据的家族不实现。

## 等价 CLI

`devsweep clean rules list`、`devsweep clean rules show <id>`、`devsweep clean scan --scope global`。

## 兼容

- 规则 ID 一旦发布不再改名。新 ID 沿用 `windows.<产品>.cache`。
- 已保存的观察与计划文件不受影响。
- 覆盖遗漏表里对应的条目（如有）同步移除，其余保留。

## 风险

- 把配置文件误认作缓存会丢用户状态。名单必须是白名单，目录名逐个列出，不用通配。
- 进程名不全会在应用运行时清理。每个家族列出主进程和已知的辅助进程，并用夹具测试运行中的情形。
