# 实施计划

开始前读：`.trellis/spec/backend/index.md`、`provider-authority.md`、`quality-guidelines.md` 里与软件清单有关的部分；`.trellis/spec/guides/cross-layer-thinking-guide.md`；本任务 `design.md`。

## 步骤

### A. core 与 CLI

1. [x] 调研并写 `research/contract.md`：`DisplayIcon` 与 `InstallDate` 的实际格式样本、MSIX 的 logo 与安装时间来源、版本策略、`discover_software_leftovers` 的调用前提。
2. [x] `software/model.rs` 加字段；ARP 与 MSIX 采集补解析；夹具测试覆盖有值、缺失、格式错误。
3. [x] CLI：`software inventory` 的 JSON 契约测试与人类可读输出（中英文）。
4. [x] 不再适用（Tauri 桌面已于 2026-11-14 移除）。
   回滚点 A。

### B. GPUI（在 airy-glass-shell 之后）

5. [x] 图标取图与缓存，带单元测试（路径校验、缓存上限、失败回退）。
6. [x] `page_workbench.rs` 的行、排序、筛选；可读来源名称与中文摘要。
7. [x] 残留只读预览（按步骤 1 的结论做或移交）。
   回滚点 B。

## 验证

```powershell
cargo test --locked -p devsweep-core software
cargo test --locked -p devsweep-cli --test cli_contract
just ci
just gpui
```

- 原生截图：英文与中文的列表、排序菜单、筛选后的列表，存入 `evidence/`。
- 实机：本机真实清单下首屏出现时间与滚动是否卡顿，记录观察。

### 完成记录（2026-11-15）

- `just ci`、`just gpui` 通过；GPUI 最终 204 项测试通过。
- Python 工具测试执行 23 项，仅有用户预先说明的两个 gitignore 子测试旧失败，未标为全绿。
- 中英文、浅深色及只读预览原生证据见 `evidence/native-acceptance.md`；检查记录见 `research/verification.md`、`research/final-review.md`。
- WindowsApps 父目录无法取得安全检查句柄的 MSIX 图标按约定回退头像；未声称 MSIX 位图渲染成功。首个图标逐帧时序未测量，异步文字优先由状态测试与源码验证。
- 先按用户要求归档并暂缓提交；2026-11-15 用户随后授权本地提交，仍不推送。

## 评审关口

- 步骤 1 的调研结论先给用户过目。
- 2026-11-15：步骤 1 调研及独立核查已完成（`research/contract.md`、`research/contract-review.md`）。用户已确认四项结论，并授权继续步骤 2–7、门禁、原生验收及归档；暂不提交。
