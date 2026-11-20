# Windows 缓存候选家族证据

- 问题：哪些产品、根目录、可再生缓存及所有者进程有足够证据进入 D4 评审？
- 范围：现有规则、厂商文档及上游源码。没有读取用户浏览器、编辑器配置或缓存，没有安装、启动或终止候选软件。
- 日期：2026-11-20。

## D4 建议

首批建议 Chrome Beta、Chrome Canary、Edge Beta、Edge Dev。四项的标准路径及进程可查，能复用已有 Chromium 注册配置文件验证。先只允许已注册配置文件的直接孩子 `Cache` 和 `Code Cache`；新通道无需扩大 GPU/着色器覆盖。也可缩小为 Chrome 两通道或更小子集。Brave、Vivaldi 适合在具备产品专属保留样本后扩充；本轮四个编辑器尚缺独立、安全的叶目录及所有者证据。最终采用范围以用户 D4 决定为准。

| 候选 | 默认根、所有者或当前可证明线索 | 可再生与必须保留 |
| --- | --- | --- |
| Chrome Beta | `%LOCALAPPDATA%/Google/Chrome Beta/User Data`，`chrome.exe` | 仅注册 profile 的 HTTP 缓存与生成代码缓存。保留下面的浏览器哨兵。 |
| Chrome Canary | `%LOCALAPPDATA%/Google/Chrome SxS/User Data`，`chrome.exe` | 同上；不可把 SxS 根或整个 profile 作为可移动目标。 |
| Edge Beta | `%LOCALAPPDATA%/Microsoft/Edge Beta/User Data`，`msedge.exe` | 同上；保留登录、扩展、用户网站数据和 Edge 收藏。 |
| Edge Dev | `%LOCALAPPDATA%/Microsoft/Edge Dev/User Data`，`msedge.exe` | 同上；共享进程名时，其他 Edge 通道运行也足以跳过。 |
| Brave 稳定 | 项目维护者给出 `%LOCALAPPDATA%/BraveSoftware/Brave-Browser/User Data`，Windows 产品程序为 `brave.exe` | 仅原理上可沿用上述两个叶名。保留 Wallet、Rewards、AI 配置和所有 profile 数据，先补专属夹具，排除其他通道和自定义目录。 |
| Vivaldi 标准安装 | 官方 Windows 崩溃报告路径证实 `%LOCALAPPDATA%/Vivaldi/User Data`，程序 `vivaldi.exe`；另有 standalone | 相同 Chromium 叶名是候选。保留书签、Notes、邮件、联系人、面板、会话；不能追踪 standalone。建议等独有数据与辅助进程夹具。 |
| Trae IDE | 候选为基于编辑器的缓存布局；官方还分别发布 TRAE Work、APP，当前材料未证明 IDE 的标准 Windows 根、准确主/辅助进程组合及产品身份 | 未批准任何叶名。保留历史、会话、全局记忆、工作区、登录和设置。不要遍历其他 TRAE 产品。 |
| Qoder IDE | 官方排错示例有 `%LOCALAPPDATA%/Programs/Qoder`、`.qoder/bin` 及 `Qoder.exe` 后端。该信息描述安装/运行部件 | 缺少独立缓存根和产品/辅助进程持有证据。保留后端、安装目录、配置、索引、会话，不能将整目录重置作为缓存清理。 |
| Windsurf | 官方记录 `~/.codeium/windsurf/hooks.json` 与项目的 `.windsurf` 设置。当前材料未证明标准缓存根、独立叶和读写进程组合 | 不能直接复制 Cursor 的四个名字。保留 Cascade 对话、记忆、hooks、工作区/全局存储、登录及扩展。 |
| Zed | 文档说明 `%LOCALAPPDATA%/Zed` 及 `zed.exe`，支持自定义数据根；这是原生编辑器 | `db` 是项目、标签和会话，`extensions` 是安装内容及其运行数据，logs 是诊断。缺少验证可独立安全重建的叶集合。 |

## 可追溯依据

- Chrome 默认配置根与渠道，Windows 可执行名，覆盖默认目录的意义。[Chromium 用户数据目录](https://chromium.googlesource.com/chromium/src/+/master/docs/user_data_dir.md) · [Google 通道路径测试](https://chromium.googlesource.com/chromium/src/+/HEAD/chrome/install_static/product_install_details_unittest.cc)
- Microsoft 官方表同时给出 Edge Beta、Dev 的默认用户数据根和各自 `msedge.exe` 路径。[Edge 通道路径](https://learn.microsoft.com/en-us/microsoft-edge/web-platform/devtools-mcp-server)
- Chromium 对 HTTP 缓存有专用 Cache_Data 和重建机制，Code Cache 是按 storage partition 初始化的生成代码缓存。选择注册 profile 的直接孩子，不从其他位置按目录名匹配，也不遍历所有 storage partitions。[HTTP 缓存目录常量](https://chromium.googlesource.com/chromium/src/+/HEAD/content/browser/network_service_instance_impl.h) · [生成代码缓存初始化](https://chromium.googlesource.com/chromium/src/+/d1a7d102945785a375d019b91994e704b2c8c7b6/content/browser/storage_partition_impl.cc)
- Brave 项目维护者说明 profile 下包含影响网站登录的持久数据；Vivaldi 官方列出默认 User Data、可执行名与独立安装变体。[Brave 问题跟踪](https://github.com/brave/brave-browser/issues/49300) · [Vivaldi Windows 路径](https://help.vivaldi.com/desktop/troubleshoot/reporting-crashes-on-windows/) · [Vivaldi 程序名](https://help.vivaldi.com/desktop/privacy/private-windows/) · [Standalone](https://help.vivaldi.com/desktop/install-update/standalone-version-of-vivaldi/)
- 编辑器信息只用于决定哪些路径必须保留。TRAE 的发行说明记载多产品和会话，Qoder 的整目录排错步骤缺少窄叶证明；Windsurf 的 hooks 和 Zed 的数据库、扩展存真实配置和用户工作。[TRAE](https://www.trae.ai/changelog) · [Qoder](https://docs.qoder.com/troubleshooting/common-issue) · [Windsurf](https://docs.windsurf.com/windsurf/cascade/hooks) · [Zed 工作区](https://zed.dev/docs/troubleshooting) · [Zed 扩展](https://zed.dev/docs/extensions/installing-extensions) · [Zed CLI](https://zed.dev/docs/reference/cli)

## 现有通路与验收

`crates/devsweep-core/src/rules/definitions.rs:5–64` 是现有六家族和刻意遗漏清单。`rules/windows_caches.rs:109–146,505–542,581–601,727–752` 区分已注册的浏览器配置、受保护路径及标准编辑器安装；浏览器读取 Local State 中合法 profile，逐一验证 Preferences。editor_installed 目前只区分两个硬编码产品，不可把未知产品走其 else 分支。扫描与预览/执行仍需实时核验、拒绝重解析/外部/不确定所有者。运行状态应报告 busy/unknown 并引导关闭后重扫，不为被拒的项目生成可执行目标。

每个选定通道测正确/未注册 profile、伪造 metadata、自定义或越界路径、同一程序名多通道 busy、所有者未知、隔离的正常目标。哨兵覆盖 Local State、Preferences、Cookies/Login Data、History/Bookmarks、Sessions、Extensions、Local Storage、IndexedDB、Service Worker/CacheStorage、着色器和 AI 替代存储；加上前端渲染及 CLI 中英文规则。对比现有夹具 `windows_caches.rs:1162–1258,1348–1396`，以及 `.trellis/spec/backend/scan-coverage.md`、`provider-authority.md` 和 `clean-advisories.md`。上述资料是规划证据；未证明这些程序在本机已安装，未运行任何清理。
