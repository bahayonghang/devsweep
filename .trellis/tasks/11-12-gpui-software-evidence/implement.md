# 实施计划

开始前读：`.trellis/spec/backend/index.md`、`provider-authority.md`、`quality-guidelines.md` 里与软件清单有关的部分；`.trellis/spec/guides/cross-layer-thinking-guide.md`；本任务 `design.md`。

## 步骤

### A. core 与 CLI

1. [ ] 调研并写 `research/contract.md`：`DisplayIcon` 与 `InstallDate` 的实际格式样本、MSIX 的 logo 与安装时间来源、版本策略、`discover_software_leftovers` 的调用前提。
2. [ ] `software/model.rs` 加字段；ARP 与 MSIX 采集补解析；夹具测试覆盖有值、缺失、格式错误。
3. [ ] CLI：`software inventory` 的 JSON 契约测试与人类可读输出（中英文）。
4. [ ] Tauri：解码器、生成类型、夹具；跑 `desktop` 测试。
   回滚点 A。

### B. GPUI（在 airy-glass-shell 之后）

5. [ ] 图标取图与缓存，带单元测试（路径校验、缓存上限、失败回退）。
6. [ ] `page_workbench.rs` 的行、排序、筛选；可读来源名称与中文摘要。
7. [ ] 残留只读预览（按步骤 1 的结论做或移交）。
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

## 评审关口

- 步骤 1 的调研结论先给用户过目。
