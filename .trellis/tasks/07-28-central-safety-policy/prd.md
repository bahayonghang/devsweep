# 中央 SafetyPolicy 与执行前实时重验证

> 父任务：07-28-audit-remediation ｜ 审计条目：F-04、F-05 ｜ 对应审计 Milestone M1 / PR5+PR6+PR7

## Goal

建立唯一、不可绕过的 `SafetyPolicy::authorize()` 执行漏斗：所有副作用在执行前对照**实时文件系统状态**重新验证路径身份、保护语义、link/reparse 与 marker；并让 Rust target 的展示、估算、守卫与 `cargo clean` 实际作用域指向同一对象。

## 背景与证据（当前 HEAD）

1. **唯一执行期防护是 self-exe guard 且 fail-open（F-04）**：`target_contains_current_exe` 在 `current_exe()` 失败时返回 `false` = 放行（`src/path_safety.rs:7-9`）；executor 仅此一处防护（`src/executor.rs:260-264`）。没有保护根、没有用户保护清单、没有 target 必须位于授权 scan root 的约束、没有执行前 link/reparse ancestor 检查、没有 marker recheck、没有 plan TTL。scanner 阶段跳过 symlink/reparse 正确，但防不了 scan 后 rename/recreate/junction replacement（TOCTOU）。
2. **Cargo 作用域不一致（F-05）**：scanner 只检查 `<dir>/target` 并按它估算展示（`src/scanner.rs:136-179`），动作却是 `cargo clean --manifest-path`；target directory 可被 `--target-dir`/`CARGO_TARGET_DIR`/`build.target-dir` 重定向，workspace 可共享 target。展示 A、清理 B，guard 也只看 A。

## 保护语义分类（二轮评审 SEC-001 修正）

一轮 PRD 笼统写"保护 home 整体、scan root、repository root"会在 descendant 匹配下拒绝**所有**合法目标（全局缓存在 home 下、项目产物在 scan root 下）。保护语义必须按类分开：

| 类别 | 成员（默认） | 匹配语义 |
|---|---|---|
| **ExactNode** | 文件系统/卷根、home 本身、Desktop/Documents/Downloads 本身、scan root 本身、repository root 本身 | 拒绝 target **等于**该节点，或 target 是该节点的**祖先**（清理包含 home 的目录更糟）。其**后代**不受此类约束——合法性由 AuthorizedFootprint 决定 |
| **ProtectedSubtree** | credentials 目录（`~/.ssh`、`~/.aws`、Cargo credentials 等）、package-manager 配置文件、当前 executable、audit log 路径 | 拒绝 target 等于、位于其内、或包含该子树根 |
| **OwnVcsMetadata** | 被扫描仓库**自身**的 `.git`/`.hg`/`.svn` | 拒绝 target 等于或位于其内；**合法 footprint 内嵌套的 `.git`**（如 `node_modules/<pkg>/.git`）不属于此类，不得误伤 |
| **UserProtectionList**（原"whitelist"更名，避免语义反转歧义） | 用户配置的永不清理路径，persistent、canonical | 拒绝 target 等于、位于其内、或包含清单项 |
| **AuthorizedFootprint**（正向门） | — | target 必须是 registry 认可的 rule footprint，且位于授权 scan root 之下（project scope）或匹配已知全局缓存 footprint（global scope）。这是让 `~/.gradle/caches` 通过而 home 本身被拒的那一层 |

## Requirements

1. 唯一漏斗：`SafetyPolicy::authorize(&validated_action, &live_fs_state, &policy) -> Result<AuthorizedAction, Denial>`；Command/Trash 两条执行路径都必须经过，类型上无法绕过；`Denial` 携带命中的保护类别与路径。
2. 按上表实现五类保护语义；优先级：ProtectedSubtree / OwnVcsMetadata / UserProtectionList / ExactNode 任一命中即拒绝，全部未命中且 AuthorizedFootprint 成立才放行（默认拒绝）。
3. 执行前实时重验证：路径存在性与身份（Windows file ID / Unix dev+inode 或等价指纹）、ancestor 链无新增 symlink/junction/reparse、root containment、marker 仍在；任一不满足 → 拒绝该 target 并要求 rescan。
4. `current_exe()` 等身份查询失败 → fail closed（拒绝执行，而非放行）。
5. Cargo 作用域：`cargo metadata --format-version 1 --no-deps` 解析真实 `target_directory` 与 `workspace_root`；展示/估算/守卫/动作全部绑定 resolved 路径，action 显式带 `--target-dir <resolved>`；execute 前重新解析，路径变化则要求 rescan/reconfirm；metadata 不可解析时降级为 Trash local target 或 inspect-only，不得静默运行范围未知的命令。

## Acceptance Criteria

- [ ] **正向 fixture（保护语义不误伤）**：home 下的 `~/.gradle/caches`、scan root 下的 `node_modules`、含嵌套 `.git` 的 `node_modules` —— 全部 authorize 通过
- [ ] **负向 fixture**：target == home、== scan root、== 卷根、是 home 的祖先、位于 `~/.ssh` 内、位于仓库自身 `.git` 内、命中 UserProtectionList —— 全部拒绝且 Denial 类别正确
- [ ] TOCTOU 套件：scan 后 rename/recreate、ancestor junction/symlink replacement、marker 消失、root escape —— 全部在副作用前被拒绝
- [ ] `current_exe()` 失败注入 → 拒绝执行（fail closed）
- [ ] Cargo fixtures：custom `--target-dir`、workspace 共享 target、`CARGO_TARGET_DIR` 三组中，display path、size path、guard path 与实际 action scope 完全一致；metadata 解析失败 → 动作降级且 UI/JSON 明示
- [ ] 全库检索证明 Command/Trash 副作用调用点 100% 经过 `authorize()`

## 约束与依赖

- 后于 07-28-plan-validation：`authorize()` 输入为 `ValidatedAction`，不做接受旧 DTO 的过渡接口。
- AuthorizedFootprint 的 registry 查询消费 plan-validation 的 registry contract；后续 rule-registry-providers 的 `safety_contract` 字段挂接本漏斗。
- Windows reparse/junction 需真实 fixture；无权限创建 link 时测试必须显式 fail/skip-with-reason，不得静默绿（implement.md 验证矩阵要求）。
- 审计预估 5–8 日（F-04）+ 2–4 日（F-05）；顺序建议：保护语义分类 → live revalidation → cargo scope。
