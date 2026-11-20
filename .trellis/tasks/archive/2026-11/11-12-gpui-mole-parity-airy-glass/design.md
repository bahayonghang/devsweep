# 设计：跨子任务的契约

各子任务的 `design.md` 写自己的细节。这里只写它们共同遵守的部分。

## 分层

```
devsweep-core   能力与数据契约（唯一实现）
   ├── devsweep-cli     子命令、JSON 契约、契约测试（先交付）
   ├── desktop-gpui     进程内调用 core（后交付）
   └── TUI              进程内使用类型
```

- "CLI 先行"的含义：一项能力先以 core 函数加 CLI 子命令的形式存在并有契约测试，GPUI 再调用同一个 core 函数。GPUI 不派生 `devsweep.exe`：`desktop-gpui/src/process_spawn.rs` 记录了无控制台的窗口程序启动控制台程序会触发终端交接失败（`0x800700E8`）。
- 每个子任务的 `design.md` 以"等价 CLI"表开头，把桌面动作对到 core 函数和 CLI 命令。父任务验收 AC2 按这些表检查。

## 新增的 CLI 面

| 命令 | 子任务 |
| --- | --- |
| `status snapshot`、`status live` 的新增字段 | status-dashboard |
| `software inventory` 的图标与安装日期字段 | software-evidence |
| `analyze largest`；`analyze roots` 的容量字段 | analyze-insights |
| `optimize plan` 可重复的 `--operation`；`optimize run` 的逐项结果 | optimize-batch |
| `history totals` | tray-panel |
| `clean rules` 里的新规则 | clean-windows-cache-coverage |
| `software plan / preview / uninstall` 支持 MSI（按产品代码调用 `msiexec /x`） | host-actions |

## 数据契约的兼容

- 新字段一律可选，缺失表示不可用并带原因，不用 0。
- V1 能否容纳纯增量字段，按 `.trellis/spec/backend/` 的规则判断；第一个涉及的子任务（status-dashboard）得出结论后，其余子任务沿用，并在父任务 `research/` 里留一份结论。
- 已保存的计划文件、观察文件和偏好文件保持可读。

## 授权链

所有会改变主机状态的界面动作走同一形状：选择 → 生成计划 → 预览得到实时摘要 → 二次确认 → 执行 → 结果与历史。选择或主机状态变化后摘要失效。界面不提供跳过预览的路径。

## 设计契约

- 来源：`research/DESIGN.md`，由 airy-glass-shell 提升为 `desktop-gpui/DESIGN.md`。
- Token 层：`desktop-gpui/src/ui.rs`。视图代码不写字号、间距、圆角和颜色字面量，由源码守卫测试约束。
- 其余界面子任务使用 airy-glass-shell 提供的 `page_bar`、`sheet_frame` 和内容区组件，不各自发明布局。

## 顺序

| 波次 | 内容 | 说明 |
| --- | --- | --- |
| 0 | 各子任务的 core 与 CLI 部分 | 互不依赖，可随时进行 |
| 1 | airy-glass-shell | 其余界面工作的基础 |
| 2 | status-dashboard、software-evidence、clean-review-flow 的界面部分 | 互不依赖 |
| 3 | host-actions | 依赖 D1；界面依赖波次 2 的 Software 与 Status |
| 4 | analyze-insights、optimize-batch、tray-panel 的界面部分；clean-windows-cache-coverage | tray-panel 依赖 status-dashboard |

父子结构不表达依赖；依赖写在各子任务 `prd.md` 的"依赖"一节。

## 回滚

每个子任务的 core 与 CLI 部分和 GPUI 部分各有回滚点，互不牵连。撤回某个界面子任务不影响已交付的 CLI 能力。
