# Mole 仓库架构分析(来源:mole-explorer 子代理,2026-08-03)

## 1. 项目定位

`tw93/mole` —— **macOS 专用**终端系统维护工具箱("CleanMyMac + AppCleaner + DaisyDisk + iStat Menus 的单一 CLI 替代品"),GPL-3.0。子命令:`clean` / `uninstall` / `purge`(清项目构建产物,与 devsweep 最相关)/ `analyze` / `status` / `optimize` / `installer` / `history`。

⚠️ **认知修正:这不是 Go 项目,是 Bash 为主的项目。** Go 仅覆盖 `analyze` 与 `status` 两个 TUI(Bubbletea);清理逻辑 100% 是 Bash,`lib/` 下 42 个文件约 1.5MB。

## 2. 架构分层

- `mole` / `mo`(根 Bash 入口):路由子命令,`exec bin/<cmd>.sh`
- `bin/*.sh`:子命令实现层(`clean.sh` 74KB、`uninstall.sh` 66KB、`purge.sh` 11KB)
- `lib/core/`:基础设施 —— `file_ops.sh`(113KB,删除引擎与路径校验)、`app_protection.sh`(101KB)、`history.sh`(审计日志)、`sudo.sh`、`timeout.sh`、`ui.sh`
- `lib/clean/`:清理规则 —— `dev.sh`(165KB,开发者工具缓存)、`project.sh`(69KB,purge 扫描)、`purge_shared.sh`(规则常量)等
- `cmd/analyze`、`cmd/status`:Go TUI,由对应 `bin/*.sh` 调起

## 3. 交互形式

纯 CLI + 终端 TUI,**仓库内无任何桌面 GUI / Tauri / Electron / SwiftUI 代码**。README 提到的付费 Mac 应用(mole.fit)是闭源独立产品,不在仓库中。两个 TUI 均支持 `--json`。

## 4. 规则组织(对 devsweep 最相关)

硬编码常量为主 + 两个用户配置文件(`~/.config/mole/purge_paths`、`~/.config/mole/whitelist`)。

`lib/clean/purge_shared.sh` 最值得直接移植:

- `MOLE_PURGE_TARGETS` — 33 个构建产物目录名:`node_modules` `target` `build` `dist` `venv` `.venv` `.pytest_cache` `.mypy_cache` `.tox` `.nox` `.ruff_cache` `.gradle` `__pycache__` `.next` `.nuxt` `.output` `vendor` `bin` `obj` `.turbo` `.parcel-cache` `.dart_tool` `.zig-cache` `zig-out` `.angular` `.svelte-kit` `.astro` `coverage` `DerivedData` `Pods` `.cxx` `.expo` `.build`
- `MOLE_PURGE_PROJECT_INDICATORS` — 项目根标记文件:`package.json` `Cargo.toml` `go.mod` `pyproject.toml` `pom.xml` `build.gradle` `Package.swift` `build.zig` `.git` 等
- `MOLE_PURGE_MONOREPO_INDICATORS` — `lerna.json` `pnpm-workspace.yaml` `nx.json` `rush.json`
- 支持 **CACHEDIR.TAG 标准**(签名 `8a477f597d28d172789f06886806bc55`),识别第三方主动声明的缓存目录
- 扫描深度 min 1 / max 6,嵌套去重

工具链缓存规则在 `lib/clean/dev.sh`,声明式一行一条(如 `safe_clean ~/.cargo/registry/cache/* "Rust cargo cache"`),按语言分组。

## 5. 安全机制(六层)

1. **路径校验**(`file_ops.sh:264` `validate_path_for_deletion`):拒绝空路径、相对路径、`..` 穿越、控制字符
2. **关键路径拒绝表**:硬拒 `/` `/System` `/usr` 等及**用户家目录根**(防变量为空塌缩后 `rm -rf` 整个家目录)
3. **ancestor-symlink guard**:逐级检查祖先目录软链,物理解析后重跑拒绝规则,且 deny-only(解析后只收紧不放宽)
4. **上下文感知白名单**(`project.sh:397`):`bin/` 只在 .NET 上下文才删;`DerivedData` 只删项目内的
5. **活跃度分类 fail-closed**(`project.sh:603`):7 天内改动标记 `recent` 默认不勾选;**超时/读取失败一律 `uncertain`,不确定就不删**
6. **回收站 + 审计日志**:Trash 三级降级(`trash(8)` → 文件系统 → Finder);**trash 不可用时放弃删除而非降级为永久删除**;操作写 `operations.log`,`mo history` 可查;全局 `DRY_RUN`

## 6. 对 devsweep 的借鉴建议

**建议采纳(多数超出 Tauri 任务范围,属规则/核心层的后续迭代候选):**

- 构建产物目录清单 + 项目根标记清单可近乎原样移植(投入产出比最高)
- CACHEDIR.TAG 支持(业界标准,成本极低)
- fail-closed:`enum Activity { Recent, Old, Uncertain }`,仅 `Old` 允许默认勾选
- 回收站优先且不可用时拒绝降级(devsweep 已用 `trash` crate,语义需对齐)
- 上下文感知目录判定(`bin`/`build`/`vendor` 必须结合兄弟文件判断)
- junction / reparse point 防护(Windows 等价于 ancestor-symlink guard;devsweep README 称已跳过 reparse point,可对照验证覆盖面)
- **审计日志 + history 视图 —— 桌面 GUI 尤其需要"我刚才删了什么"的可查记录**(与本父任务直接相关,可列为桌面端后续迭代)
- 7 天活跃度阈值 + UI 中 `Recent` 默认不勾选

**不建议照搬:**

- Bash 巨型单文件不可维护;devsweep 规则应做成 Rust 数据表(const/RON/TOML),便于测试与 GUI 展示
- Mole 规则全是 macOS 路径,Windows 需重写
- Go TUI 对 Tauri 无参考价值,但其 `--json` 输出契约可作前后端 schema 参考

## 关键文件路径

- `ref/repo/Mole/lib/clean/purge_shared.sh` — 规则常量,最先读
- `ref/repo/Mole/lib/core/file_ops.sh` — 删除引擎/路径校验/Trash 路由
- `ref/repo/Mole/lib/clean/project.sh` — purge 扫描与活跃度判定
- `ref/repo/Mole/lib/clean/dev.sh` — 各语言工具链缓存规则
- `ref/repo/Mole/lib/core/history.sh` — 操作审计日志
- `ref/repo/Mole/cmd/analyze/delete.go` — Trash 三级降级
- `ref/repo/Mole/SECURITY_AUDIT.md` — 安全边界文档
