# 设计：软件清单证据与列表

## 等价 CLI

| 桌面动作 | core | CLI |
| --- | --- | --- |
| 载入清单 | 软件清单采集（`software inventory` 使用的同一入口） | `devsweep software inventory --format json` |
| 查看所选应用的残留 | `discover_software_leftovers`（`software/leftovers.rs:493`） | `devsweep software leftovers scan` |

桌面端在进程内调用 core。新字段先出现在 CLI 的 JSON 里，再接到界面。

## core 契约（`crates/devsweep-core/src/software/`）

`SoftwareEntryV1`（`model.rs:247`）增加两项可选证据：

| 字段 | 来源 | 说明 |
| --- | --- | --- |
| `icon: Option<IconEvidenceV1 { path, index }>` | ARP 的 `DisplayIcon`；MSIX 清单里的 logo 路径 | 只记录位置，core 不读取图像 |
| `installed_on: Option<日期>` | ARP 的 `InstallDate`；MSIX 包的安装时间 | 解析失败即为空，不猜 |

- 不读取、不输出任何卸载命令字段。core 关于"厂商卸载命令不是授权依据"的规则不变。
- 版本策略与 status-dashboard 相同：先按 `.trellis/spec/backend/` 的数据契约规则判断可选字段能否留在 V1，结论写入 `research/contract.md`。

## GPUI

- 图标：后台任务按 `IconEvidenceV1` 取图，容量有界的缓存（按软件 ID 键控），取不到就用现有字母头像。只接受本机固定盘上的路径；只读取资源，不执行文件，不访问网络路径。取图接口（`Win32_UI_Shell` 已在 `Cargo.toml` 启用）和 GPUI 在固定版本里显示位图的方式，在实施第一步确认；不可行时保留字母头像并在验收里说明。
- 行：图标、名称、`版本 · 发布者`、容量（带证据限定词，未知就写未知）、安装日期。来源显示可读名称，不显示 `arp:CurrentUser:Registry64` 这类内部 ID。
- 排序：名称、容量、安装日期（`page_workbench.rs:126` 现有两项）。没有日期的条目排在最后。
- 筛选：来源（ARP、MSI、MSIX）和可处理性。筛选与搜索可叠加。
- 残留预览：对选中的应用只读地列出残留。第一步先确认 `discover_software_leftovers` 能否在卸载之前调用；`plan_software_leftovers` 需要一次成功的卸载记录，本任务不触碰它。若发现只在卸载后有意义，这一项移交 host-actions。

## 兼容与回滚

- 旧清单 JSON 仍可读入（新字段缺省）。测试钩子 `DEVSWEEP_SOFTWARE_INVENTORY`（`software_review.rs:28`）的夹具补上新字段的有、无两种情形。
- 回滚 GPUI 不影响 core 与 CLI。

## 风险

- `DisplayIcon` 的格式不统一（带引号、带索引、指向已不存在的文件）。解析要有夹具测试，失败一律回退。
- 上千个应用时取图不能阻塞列表。首屏先出文字，图标陆续到达。
