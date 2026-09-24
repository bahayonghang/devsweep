---
skill: trellis-plan-review
version: 0.5.0
task_dir: D:/Documents/Code/Rust/Exp/devsweep/.trellis/tasks/09-20-desktop-mole-tauri-performance
task_name: 09-20-desktop-mole-tauri-performance
task_status: planning
review_scope: task-tree
task_count: 5
task_members:
  - 09-20-desktop-mole-tauri-performance
  - 09-20-desktop-mole-workbench-ux
  - 09-20-desktop-tauri-cli-service
  - 09-20-desktop-operation-performance
  - 09-20-desktop-native-acceptance
task_statuses:
  09-20-desktop-mole-tauri-performance: planning
  09-20-desktop-mole-workbench-ux: planning
  09-20-desktop-tauri-cli-service: planning
  09-20-desktop-operation-performance: planning
  09-20-desktop-native-acceptance: planning
verdict: 需返回规划
blocking: 3
should_fix: 4
notes: 1
generated_at: 2026-09-21T17:05:00+08:00
---

# Trellis 规划审阅报告

## 审阅范围

根任务为 `09-20-desktop-mole-tauri-performance`，模式为 `task-tree`，共 5 个成员。以下顺序来自 `task.json.children` 的根优先遍历，表达归属，不表达执行依赖。

| 顺序 | 任务目录（相对于 `.trellis/tasks/`） | 状态 |
| --- | --- | --- |
| 1 | `09-20-desktop-mole-tauri-performance` | planning |
| 2 | `09-20-desktop-mole-workbench-ux` | planning |
| 3 | `09-20-desktop-tauri-cli-service` | planning |
| 4 | `09-20-desktop-operation-performance` | planning |
| 5 | `09-20-desktop-native-acceptance` | planning |

审阅基准为当前工作树，Git HEAD 为 `fc4269fe5c1bc65bf53df97e5c09c6b36081deda`。5 个任务的 31 个规划/研究文件在初次检查与报告写入前的 SHA-256 比较中均未变化；它们本来就是未跟踪文件，不能把“未跟踪”误报为本次审阅产生的改动。

完整读取了所有成员的 `task.json`、`prd.md`、`design.md`、`implement.md`、两个 JSONL 清单，以及父研究材料；按代码地图核对 React、Tauri、CLI/core、资源采样脚本、桌面规范和引用的历史验收协议。两个只读审阅员分别检查 UX/native 与 service/performance，主线程复核、合并并独立处理父任务断言。只产生这一份合并报告。

Pass 0 已执行 `plan_precheck.py <root> --include-descendants`：5 个成员、4 条边、全部 planning、阻断 0、无占位符、报告路径被 Git 忽略。随后对每个成员执行 `task.py validate`，全部通过，清单真实条目总数为 43。结构检查没有证明 AC 已有正确机制；预检没有识别部分标题格式中的 R，也没有给研究文件中的代码引用自动背书，相关内容由本次人工逐项核对。

Pass 1–6 覆盖全部成员；Pass 7 对全部成员均不适用，因为它们尚未开始实施。本轮没有运行产品测试、构建、资源基准、UI、安装器或清理操作，也没有修改任务状态、规划或产品代码。

## 结论

需返回规划 — 阻断 3 / 应修 4 / 提示 1

主要问题是验收链条尚不能证明其承诺：Clean 的现有基准比较有假通过路径，Status 的现有停止实验没有测量桌面取消与 join，包装身份没有落到子任务的验证机制。共享进程内 core/service 的既定选择、UX 的呈现层边界以及原有安全核心不需要推翻。

## 问题清单

### TPR-01 · 阻断 · Clean 基线与资源差值门槛未被指定采样器真实判定

**Task:** cross-task  
**Affected tasks:** `09-20-desktop-mole-tauri-performance`、`09-20-desktop-operation-performance`、`09-20-desktop-native-acceptance`

**Location:** `09-20-desktop-mole-tauri-performance/prd.md:134-137`、`design.md:93-105`、`implement.md:44-53`；`09-20-desktop-operation-performance/design.md:5-15`、`implement.md:13-18,27-28`；`09-20-desktop-native-acceptance/implement.md:5-8,16`。本报告省略目录前缀的同段引用仍属于该段明确点名的任务。

**Claim:** 用当前 release、同 host/build/fixture 的记录重新执行冻结阈值；环境变化后采集新基线。性能子任务指定 `tools/measure-resources.ps1 -Protocol five-mode-v1`，native 子任务消费其结果。

**Evidence:**

| 位置 | 实际行为或契约 |
| --- | --- |
| `.trellis/tasks/archive/2026-09/08-29-five-mode-native-integration/design.md:36-39,55` | Clean 要比较 accepted baseline 的同名统计：median 比值 ≤1.20、private bytes ≤baseline+64 MiB、threads ≤baseline+2。 |
| `tools/measure-resources.ps1:1381-1392` | 基线写死为 `39999.052` ms；可比性仅检查逻辑核数为 24、Windows build 为 26200、根路径等于仓库。没有核对本轮基线的完整 host/build/fixture 身份。 |
| `tools/measure-resources.ps1:1393-1396` | 只比较 elapsed ratio；threads 和 private bytes 的 gate 直接传 `$true`，说明是“当前已记录”，并非两个差值阈值通过。 |
| `tools/measure-resources.ps1:1399-1403,1488` | 不可比较时仍写 `pass = $true`；总结果仅取 failures 是否为空。 |

复算：该历史常量下的 elapsed 上限为 `39999.052 × 1.20 = 47998.8624 ms`，算术本身正确；问题是常量并非本轮 accepted baseline，且 `64 MiB = 67108864 B` 与 `+2 threads` 没有进入判据。父 `implement.md:48-49` 泛称扩展采样器，但没有识别或替换这些现有假通过分支；其余当前设计也没有补足基线来源与三个 comparator。

**Impact:** 照当前执行步骤可以得到总 PASS，却没有可比基线，也没有证明两个资源阈值。该问题直接破坏父 AC4，而不是“尚未实施所以没有测量”。

**Route:** 在性能设计和步骤中明确本轮 accepted baseline 的来源、采集及身份关联，承接三个真实比较；不可比较应阻止对应 gate 通过。父整体验收和 native 消费端必须采用相同证据有效性条件。保留冻结阈值，不用历史常量或“记录完成”替代通过。

### TPR-02 · 阻断 · Status 停止实验测量已退出 CLI，不能证明桌面 quiescence

**Task:** cross-task  
**Affected tasks:** `09-20-desktop-mole-tauri-performance`、`09-20-desktop-operation-performance`、`09-20-desktop-native-acceptance`

**Location:** `09-20-desktop-mole-tauri-performance/prd.md:67-72,134-141`、`design.md:95-111`；`09-20-desktop-operation-performance/design.md:5-11,19-24`、`implement.md:27-28`；`09-20-desktop-native-acceptance/prd.md:37-40,48-50`、`design.md:17-18`。

**Claim:** 使用现有协议测得桌面取消、worker/producer join 和停止后静止状态，并以此证明跨模式生命周期门槛。

**Evidence:** `tools/measure-resources.ps1:1102-1107` 启动的是 `devsweep status live` CLI；`:1108-1115` 通过 `CloseMainWindow`、`Stop-OwnedProcess`/`Kill` 结束该进程；`:1117-1118` 随后才开始采集 5 秒窗口。`:1126-1135` 在 PID 不存在时保留 CPU=0、threads=0，再由 `:1378-1379` 判断停止 gate。它没有触发持续运行桌面中的 `status_cancel`。

真实桌面入口在 `desktop/src-tauri/src/status.rs:219-225`；producer 取消与 `control.join()` 位于同文件 `:143-168`。引用协议 `.trellis/tasks/archive/2026-09/08-29-five-mode-native-integration/design.md:41-48` 要求从 stop acknowledgement 起算完整 25 个计划样本，并验证最后五个样本连续满足阈值。CLI 被杀后的零值样本不是这个桌面观察窗口。

**Impact:** 即使桌面仍采样、取消卡住或 producer 未 join，CLI 已退出的实验也能通过。仅添加截图、原始文件和 PID 字段不能修复被测对象及起止事件错误。

**Route:** 在规划中分开 CLI 退出证据与桌面停止证据，指定真实桌面取消 acknowledgement、worker/producer join 与同一存活 app PID 的后续采样路径。强杀或 PID 缺席不能作为桌面 quiescence 通过依据；native 子任务必须消费这个明确的桌面窗口。

### TPR-03 · 阻断 · packaging identity 没有落到 native 子任务的机制和判据

**Task:** cross-task  
**Affected tasks:** `09-20-desktop-mole-tauri-performance`、`09-20-desktop-native-acceptance`

**Location:** `09-20-desktop-mole-tauri-performance/prd.md:142-145,159`、`implement.md:61-62`；`09-20-desktop-native-acceptance/task.json:5`、`prd.md:18-52`、`design.md:3-25`、`implement.md:10-28`。

**Claim:** 父 AC6 要求原生包装身份记录，子任务描述和归属表也将 packaging 交给 native acceptance。

**Evidence:** 完整读取 native PRD/design/implement，并检索父/native 工件中的 `packag|identity|install|taskbar|icon`：子任务矩阵只定义模式、语言、尺寸/比例、交互和 authority/lifecycle；没有定义包身份要观察什么及如何核对。父步骤的 “package/build checks” 也没有绑定产物和通过条件。

实际包契约是 `desktop/src-tauri/tauri.conf.json:3-5,28-46` 的 product/version/identifier、NSIS、publisher、图标和 currentUser 安装模式。`desktop/package.json:15` 的 build 仅运行 TypeScript/Vite；`justfile:226` 的 `ci` 仅运行 Cargo 的 fmt/check/test/clippy。Tauri 包构建另在 `justfile:199-200`。历史报告 `docs/validation/five-mode-native.md:47` 把复制 ICO 与 hash 列为旧图标证据，但这不证明本轮包装产物或运行时身份。

**Impact:** 执行全部已列命令并填满现有矩阵，仍无法判定父 AC6 的 packaging identity 子句。它也没有对应 native R1–R3 中的明确验收义务。

**Route:** 在 native 任务补齐包装身份的范围、产物来源、核验入口和结果条件，并同步父 AC6/交付归属；仅采用完成该现有要求所需的最小检查，不把本项扩成签名、发布或包装重构。安装等外部写入若确实成为选定机制，需要其自身授权，报告不预先授权。

### TPR-04 · 应修 · “唯一 React IPC 入口”的事实与规划边界不一致

**Task:** cross-task  
**Affected tasks:** `09-20-desktop-mole-tauri-performance`、`09-20-desktop-tauri-cli-service`

**Location:** `09-20-desktop-mole-tauri-performance/prd.md:28-30,55-61,130-133`、`design.md:118-119`、`research/current-state-and-mole-reference.md:28-30`；`09-20-desktop-tauri-cli-service/prd.md:21-31`、`design.md:20-23`、`implement.md:13-20`。

**Claim:** `desktop/src/api/bridge.ts` 已是且将保持唯一 React→Tauri 边界，所有桌面操作均跨越 `DesktopBridge`。

**Evidence:** 对 `desktop/src` 全树检索 `invoke|@tauri-apps`，除 bridge 外，`desktop/src/i18n/index.ts:165-167` 的 `PresentationSettingsBridge` 在生产路径直接 invoke 设置读写；`desktop/src/lifecycle.ts:21-32` 另有窗口事件适配及 DEV-only fault invoke。`desktop/src/App.tsx:68-74,95-117,119-138` 将三种 bridge 分别注入并使用。前者是真实生产查询/写入，不能全部归入调试例外。

**Impact:** “五模式由类型边界调用 core”这一观察成立，但据此推导“所有 React IPC 只在一个文件”不成立。现有设置/lifecycle 适配是否例外、是否要归并没有被定义，执行者会对全量边界检查及 service 子任务的完成范围作出不同判断。这里没有证据表明组件直接绕过了清理授权，也不据此重开已选定的进程内 shared service 决策。

**Route:** 明确业务操作、展示设置和窗口生命周期适配的边界，统一事实表述、R2/AC3 与 service 工作清单。可保留明确的 typed 适配例外，也可在已批准目标确实要求单入口时列出归并及测试范围；不能继续把当前多入口描述为已经单入口。

### TPR-05 · 应修 · 父任务禁止改变显示设置，native 子任务却依赖手动改变

**Task:** cross-task  
**Affected tasks:** `09-20-desktop-mole-tauri-performance`、`09-20-desktop-native-acceptance`

**Location:** `09-20-desktop-mole-tauri-performance/prd.md:102-104`、`design.md:107-111`；`09-20-desktop-native-acceptance/prd.md:29-33,56`、`design.md:14-18`。

**Claim:** 在不改变用户 display configuration 的前提下完成 Windows 100/125/150/200% 验收；native 子任务却要求 operator 手动改变 Windows display scale。

**Evidence:** 父约束没有限定“仅 agent 不得改变”，而子设计明确将改变系统比例交给 operator。既有文档 `docs/validation/five-mode-native.md:3-6,43-44` 记录的实际方法是 CSS device metrics override 与 WebView2 `--force-device-scale-factor`，host OS scale 保持不变；`.trellis/spec/desktop-frontend/index.md:78-82` 也采用 WebView device scale。它们不能证明实际 Windows display scale 已改变。

**Impact:** 按子步骤操作违反父 no-change 约束；沿用旧捕获方法则不能诚实称为实际 Windows 四档缩放。相同截图可能被不同验收者记成不同证据级别。

**Route:** 统一 actual Windows scale、WebView device scale、CSS viewport 三种证据及权限前提。可以保持父 no-change 边界并使用既有独立环境/记录未取得档位；若必须由 operator 改变系统设置，应在规划确认后明确该前提与父约束的变化。不得把 device emulation 提升为 OS 缩放证据。

### TPR-06 · 应修 · cancel-to-join bound 没有可判定的范围与计时条件

**Task:** cross-task  
**Affected tasks:** `09-20-desktop-mole-tauri-performance`、`09-20-desktop-tauri-cli-service`、`09-20-desktop-operation-performance`、`09-20-desktop-native-acceptance`

**Location:** `09-20-desktop-mole-tauri-performance/prd.md:67-74,134-137`、`implement.md:99-100`；`09-20-desktop-tauri-cli-service/prd.md:28-31,56-57`；`09-20-desktop-operation-performance/design.md:19-24`、`prd.md:47-51`；`09-20-desktop-native-acceptance/prd.md:48-50`。

**Claim:** “cancel-to-join bound” 必须通过，性能测量需要比较 cancellation-to-join latency。

**Evidence:** 全读当前 5 个任务工件并检索 `cancel|join|500|latency|bound|timeout`，没有为该全局 bound 给出数值、适用操作、起止事件及统计量。所引用历史协议 `.trellis/tasks/archive/2026-09/08-29-five-mode-native-integration/design.md:41-48,56` 分别定义 Status 的 5 秒 post-stop 窗口与 Analyze 的 cancel acknowledgement p95≤500 ms；二者都不是全模式 cancel-to-worker-join 的统一阈值。

真实代码还区分“主动取消”和“等待完成”：`desktop/src/modes/clean/CleanWorkbench.tsx:98-105,125-132` 的 dry-run/execute cancel callback 是 no-op；`desktop/src-tauri/src/clean.rs:40,75` 为 `cancel: None`；`desktop/src/state/operation-coordinator.ts:102-110` 仍 await join。这不能被简化成已经存在全模式主动取消时限，也不能反过来断言它不 join。

**Impact:** 实现者可以按取消请求返回时间、worker 结束时间或 post-stop 窗口分别判定同一结果。当前 AC4/Gate C 无法对“bound 已通过”给出唯一答案。

**Route:** 明确每类适用操作的取消/完成语义及测量起止点，为本轮确实要求的 bound 指定阈值与统计口径；对不可中断动作明确其等待完成验收。复用既有阈值时保持原本的作用域，不把 Analyze acknowledgement 的 500 ms 自动推广到所有 join。不得以超时强杀替代安全终止。

### TPR-07 · 应修 · 已测阈值失败可被子 AC 的 “or UNVERIFIED” 消解

**Task:** cross-task  
**Affected tasks:** `09-20-desktop-mole-tauri-performance`、`09-20-desktop-operation-performance`、`09-20-desktop-native-acceptance`

**Location:** `09-20-desktop-operation-performance/prd.md:47-49`；`09-20-desktop-native-acceptance/prd.md:48-52`；`09-20-desktop-mole-tauri-performance/design.md:104-105`、`implement.md:52-53,99-102`。

**Claim:** 性能阈值通过，或每个 unmet/native-only criterion 有 owner 并标为 `UNVERIFIED`，即可满足该子 AC；native AC 也使用 meet protocol “or marked UNVERIFIED”。

**Evidence:** 父设计把 threshold miss 明确规定为 stop/go failure，父 Gate C 要求 passing thresholds；native 自身 `design.md:22-24` 同样要求失败停止 promotion。性能 `prd.md:47-49` 把已测不达标的 unmet 与没有证据的 native-only 并列，native `prd.md:48-50` 未限制替代条件。native 又在 `prd.md:12-13` 依赖 accepted performance child。

**Impact:** 同一个已测 FAIL 可以依子 AC 被标为 `UNVERIFIED` 后移交，但依父 Gate C 不能通过。这会产生冲突的子任务接受/父任务完成状态，而非单纯报告措辞差异。

**Route:** 统一已测 FAIL、未测 UNVERIFIED、允许交给 native 的待验证项对移交和完成的作用。可以有带缺口的证据移交，但不得把已知失败改成未知或据此通过父 gate；在父与两个消费子任务同步写明。

### TPR-08 · 提示 · 最终类型验证命令没有使用已有的只读一致性检查

**Task:** cross-task  
**Affected tasks:** `09-20-desktop-mole-tauri-performance`、`09-20-desktop-tauri-cli-service`、`09-20-desktop-native-acceptance`

**Location:** `09-20-desktop-mole-tauri-performance/prd.md:146-147`、`implement.md:61-62,73-77`；`09-20-desktop-tauri-cli-service/implement.md:26-29`；`09-20-desktop-native-acceptance/implement.md:22-28`。

**Claim:** 完成 generated IPC checks；具体命令表只列 `types:generate`，native 命令表没有单列生成物一致性检查。

**Evidence:** `desktop/scripts/generate-types.mjs:327-347` 表明普通 generate 会写回 `types.gen.ts`，`--check` 才比较并在不一致时失败。仓库已有 `justfile:181-200` 的 `desktop-web-check`，其中 `:185` 使用 `npm run types:generate -- --check`；CI 也在 `.github/workflows/ci.yml:80` 使用这一参数。

**Impact:** 实施期间重新生成类型本来合理，不构成独立阻断；但最终审查若直接生成再测试，会把“原产物一致”与“检查时被更新”混在一起，影响变更归因。

**Route:** 最终一致性核验可直接采用现有 `--check` 或 `desktop-web-check`；需要更新时把生成及 diff 审阅留在实施步骤。无需新建验证脚本。本项为可选提示，不作为额外开始门。

## 未能核实

| 项目 | 未核实原因与边界 |
| --- | --- |
| 当前二进制的五模式视觉、键盘、双语、长文本、reduced-motion、forced-colors | 本轮是只读规划审阅，没有启动 UI。已有测试代码不等于本次运行通过。 |
| 当前资源数值、回压、取消时延、无重叠、worker/process tree、UAC | 没有运行基准或实际操作。TPR-01/02 只证明指定测量路径有缺口，不推断当前产品必然泄漏或超标。 |
| 实际 Windows 100/125/150/200% 缩放、包装与安装后身份 | 没有改变显示设置、构建/安装包或操作桌面。WebView/CSS 模拟与历史 ICO hash 不补足这些直接证据。 |
| 当前 release 的真实 provider、软件执行及所有失败路径 | 未执行 Software/Optimize/清理。native 的 deterministic fixtures 不必等于开发版 fixtureBridge；仅因 `desktop/src/main.tsx:8-11` 的 fixtureBridge 限于 DEV，不能断言 release 原生文件系统夹具不可实现。 |
| `just ci`、frontend gates、生成物检查是否通过 | 本轮只核实命令与配置存在，未执行产品 gate。Trellis validate 的通过只覆盖任务上下文文件结构。 |

没有把 native 的 390/800 宽度单列为缺陷：`desktop/src-tauri/tauri.conf.json:18-19` 的原生窗口最小值为 900×600，而 native `prd.md:32` 已写 `where supported` 并允许 unavailable reason。后续证据须区分窗口尺寸、CSS viewport 和 DPR，不能把模拟宽度当成实际窗口尺寸。

## 可靠部分

### 结构、来源和既有决策

| 检查 | 已核实结果 |
| --- | --- |
| 任务树与上下文 | 1 父 + 4 子、回链一致、无归档同名歧义、全部 planning。真实清单条目分别为父 4/5、UX 3/3、service 6/5、performance 5/4、native 4/4，共 43。不再沿用历史“seed-only”状态。 |
| 明确执行依赖 | 父 `implement.md:17-60` 明确 service→UX→performance→native；service PRD:12 声明先行，performance PRD:12-13 声明依赖 service，native PRD:12-13 声明依赖其余三项。没有仅靠 children 顺序推断依赖。 |
| Mole 来源 | `git -C ref/Mole rev-parse HEAD` 匹配 `014a25f88db3fd2e5ef65011c38954b827c9a8b1`，引用 checkout clean，`.gitignore:35` 排除 ref。`ref/Mole/README.md:31,47,460-462` 区分 macOS CLI、实验 Windows 分支与独立商业 Mac 产品；`TRADEMARK.md` 说明名称/品牌边界。 |
| 外部表面核对 | 本次读取 [Mole 官方中文页](https://mole.fit/zh/)，确认五工具入口、先审查选择、分析导航及状态展示这些有限事实。这里只核对引用内容，不将其 macOS 能力或视觉资产视为 DevSweep 的实现契约，也不作版权法律结论。 |
| shared service 选择 | `desktop/src-tauri/Cargo.toml:20-24` 依赖 core；`.trellis/spec/backend/directory-structure.md:131-153` 限定 core process runner 与薄 Tauri 适配。当前桥接与 CLI 同调 core 的事实支持既有进程内方案，不需要重新选择外部 CLI subprocess。 |

### 已有机制与反证检查

UX 的呈现边界基本成立：`desktop/src/app-shell/registry.ts:26-88` 已注册五模式与支持路由；`AppShell.tsx:129-142,220-234,258-321` 有路由、焦点及键盘机制。`desktop/src/styles.css:133-140,163-166,211` 确有可被此次 polish 处理的 light fallback。保留既有 reducer/DTO/callback 的设计有明确机制，不能仅因没有新代码就判成缺失。

Clean 的授权机制仍有代码依据：`desktop/src/state/app-state.ts:73-95,194-226` 处理 inspect-only、digest 失效和确认；`desktop/src-tauri/src/clean.rs:31-43,66-78` 使用 `validate_plan`、Executor 和 expected digest。CLI 的 saved-plan 文件/`--confirm` 与桌面内存计划/确认 dialog 是不同承载方式；`crates/devsweep-cli/src/application/commands/clean.rs:70-102,132-144` 与 `CleanWorkbench.tsx:117-132` 支持这种区别。本报告没有从措辞差异推导“桌面已经绕过 core 授权”。

`OperationCoordinator` 的 start/stop/complete 确实序列化并等待 join，见 `desktop/src/state/operation-coordinator.ts:32-77,102-110`。no-op cancel 与 no-join 是不同命题，TPR-06 只指出时限判据未定义。Scan 也已有 100 ms 合并与末尾 flush，见 `desktop/src-tauri/src/scan.rs:18,167-192,227`，无需为“回压”一词直接发明第二缓冲协议。

父统一 Channel/sequence 的表述需按实际操作理解：bridge 中 Software/Optimize 与 Status snapshot 是 result-only，Scan/Analyze/Status live 才带 Channel（`desktop/src/api/bridge.ts:21-37,62-138`）。service R2 已允许经说明的 versioned schema change，故本次不把“保留”误读成禁止所有必要协议改动，也没有单列推测性阻断。实现前的现状映射应保留这些区别。

### 28 条 AC 的逐子句追溯

下表逐项拆开并列义务再映射。`P/U/S/F/N` 仅为本报告的任务缩写：父、UX、service、performance、native；后三类未编号 AC 以出现顺序引用，不是给规划添加新编号。文档路径均在上方 5 个成员目录内。

| AC（原文位置） | 子句 → 要求与机制 | 审阅结果 |
| --- | --- | --- |
| P-AC1，父 PRD:122-124 | commit/URL/来源边界 → R1/R6、research:3-18；拒绝复制 → R1、research:52-63、UX R2/implement:6 | 来源已核实；未来产物不复制仍须实施后检查。 |
| P-AC2，父 PRD:125-129 | 五模式/持久导航/标题 → R1、UX R1/design:5-12；action/detail/states → R1/R5、父 design:70-86、UX design:14-23；双语/四宽度 → R1/R6、UX R4/implement:21-22 | 机制存在；运行验收未执行。 |
| P-AC3，父 PRD:130-133 | typed bridge/decoder/error/CLI parity fixtures → R2、service R1/R2/R4；无组件 invoke → R2、现有桥接；plan/digest/确认 → R4、父 design:45-59 | 安全与 parity 有机制；全桌面单入口范围见 TPR-04。 |
| P-AC4，父 PRD:134-137 | raw+manifest+workloads → R3、父 design:93-103、F-R1；冻结阈值 → F-R3；cancel bound/no-overlap/quiescence → R3、coordinator、性能设计 | TPR-01/02 为验证机制缺口；TPR-06 为不确定判据；TPR-07 为失败状态冲突。 |
| P-AC5，父 PRD:138-141 | single-flight/stale/route/unmount → R3/R5、coordinator/spec；inspect-only/selection/rescan → R4/R5、Clean reducer/spec；partial/unknown → R4/R5、各 mode DTO | 可追溯；R3/R4 虽未写在 AC 括号中但实际有对应义务，不按预检文本匹配误报缺 R。 |
| P-AC6，父 PRD:142-145 | keyboard/focus/locale/a11y → R6、UX/native；scaling → R6、native R2；UAC/process → R4/R5/R6、native R3；缺证据保留未知 → R6、native matrix；packaging identity → native 归属 | TPR-03 缺包装机制及子需求；TPR-05 缩放前提冲突。 |
| P-AC7，父 PRD:146-147 | Rust/frontend/type gates → R6、父 implement:68-86；保留无关 dirt → R6/父 preconditions | 命令真实存在；生成物一致性提示见 TPR-08。 |
| P-AC8，父 PRD:148-150 | 全树产物/真实 manifests/validate/review-before-start → 父 Key decisions:116-118、implement:5-13 | 结构和本次审阅已完成；本报告不构成开始批准。 |
| U-AC1，UX PRD:41-42 | 五模式/无 placeholder → R1、registry；route/back/focus/keyboard → R1、design:5-8,25-31、现有 AppShell tests | 有机制，未运行。 |
| U-AC2，UX PRD:43-45 | 统一 dark grammar → R2、design:10-12；无 Mole copy/light pane/虚假 freed 文案 → R2/R3、父 design:83-86、desktop copy spec | 有机制；no-copy 为实施后检查。 |
| U-AC3，UX PRD:46-47 | inspect-only/selection invalidation/digest/二次确认 → R3、design:16-18、既有 reducer/callback | 有机制，无须从头重建授权。 |
| U-AC4，UX PRD:48-49 | Analyze read-only、Software/Optimize 现有能力、Status unsupported 非零填充 → R3、design:19-23、各 mode adapter | 有机制；StatusWorkbench.tsx:49-79 已区分 unavailable。 |
| U-AC5，UX PRD:50-52 | 双语/断点/a11y media/state → R4、design:25-31、implement:9,21-22；长数据 accessible/copyable → R5、component spec:80-82 | 有机制，不能以 selector 存在冒充原生视觉 PASS。 |
| U-AC6，UX PRD:53-55 | frontend gates → implement:14-19；跨 Rust/CLI 修改必须可追溯 → ownership:14-17、out-of-scope:59-60 | 边界与命令一致。 |
| S-1，service PRD:49-50 | 无 CLI child/import、同 core owner → R1、design:5-23、implement:13-16,30-31 | 有机制且现状支持。 |
| S-2，service PRD:51-52 | request/event/result/error/id/cancel/audit parity → R2/R4、design:14-18,27-32、implement:17-20 | 有机制；parity 是核心语义，CLI/Tauri envelope 不必字节相同。 |
| S-3，service PRD:53-55 | plan/live digest/显式确认 → R3、design:31-32、父 design:45-59；preview 不能单独授权 → 同安全机制 | 核心机制存在，须按 CLI/desktop 的承载方式核验，不误报现有绕过。 |
| S-4，service PRD:56-57 | 取消终态已 join、旧 completion 不替代新操作 → R2/R4、design:28-30、coordinator | 有机制；全局延迟 bound 另见 TPR-06。 |
| S-5，service PRD:58-60 | contract/types/CI → R4/父 R6、implement:24-31；native 缺口不得推定 → 父 native 交付 | 有机制；只读生成检查见 TPR-08。 |
| F-1，performance PRD:45-46 | 相同环境可复现、workload/sample count → R1、design:5-15、父 design:93-99 | 原则已给出；Clean 的实际基线机制见 TPR-01。 |
| F-2，performance PRD:47-49 | 冻结阈值 → R3、引用协议；unmet/native-only 状态和 owner → evidence handoff | TPR-01/02/07。 |
| F-3，performance PRD:50-51 | 无 stale/orphan/backlog → R2、diagnosis seams；无语义/authority/cancel 回归 → R3、design:26-31、focused tests | 有诊断路径；桌面停止证据不能用 CLI 替代，见 TPR-02/06。 |
| F-4，performance PRD:52 | focused coordinator/mode tests、CI → R3、implement:23-26 | 命令存在，未运行。 |
| F-5，performance PRD:53-54 | parent 收到 raw links/cause/fix/verification，早于 native → R1/R2/R3、implement:18-19、父执行顺序 | 已明确顺序。 |
| N-1，native PRD:44-45 | mode/locale/width/scale/interaction/result/证据或 unavailable reason → R1/R2、design:5-10 | 矩阵结构存在；scale 前提见 TPR-05。 |
| N-2，native PRD:46-47 | review-first、plan/digest/确认、不引入其它路径 → R3/父 R4、design:12-18、继承 Clean authority | 有机制，原生结果未核实。 |
| N-3，native PRD:48-50 | cancel/restart 无旧 completion/orphan/unexpected child → R3、design:17-18；resource protocol 或未知 → R3、引用协议 | TPR-02/06/07。 |
| N-4，native PRD:51-52 | automated/native gate、失败回送、parent links → dependencies、design:20-25、implement:12-28 | 有机制；不能补足未进入矩阵的包装子句，见 TPR-03。 |

逆向检查了 21 个 R 组：父 R1–R6、UX R1–R5、service R1–R4、performance R1–R3、native R1–R3 均有上述 AC 或执行门承接。没有把工具未识别标题、未标显式 R 的 AC 或测试尚未执行直接当成缺陷。

### 数值、单位和证据分层

200 ms ×25=5000 ms；协议最后五个采样时间桶为 1000 ms，CPU 增量还需对应前一时间戳，不能只用五个时间点的跨度替代。256 MiB=268435456 B、512 MiB=536870912 B。nearest-rank p95 对 5 次运行取第 5 个值，对 30 次导航取第 29 个值；5 次 warm-up 不进入 30 次 measured 样本。CPU 的 100% 表示一个逻辑核，200% 表示两个逻辑核，不是整机 CPU 的 200%。这些口径来自引用的历史协议与 `tools/measure-resources.ps1:83-93`，本次静态复算无误；统计口径成立不修复 TPR-01/02 的证据来源错误。

现有 native 设计已明确截图只能证明视觉、日志及进程/资源样本承担生命周期证据，失败归属回送对应子任务；这是应保留的分工。历史报告、fixture、CDP 和当前 release 原生证据仍需分别标注。

## 盲区

Agent 审阅另一个 Agent 的规划并不等于独立第二意见；即使分工复核，双方仍可能共享同类盲区。没有发现问题只表示本轮没有发现，不能证明规划完整；本报告是待分诊清单，不是批准。
