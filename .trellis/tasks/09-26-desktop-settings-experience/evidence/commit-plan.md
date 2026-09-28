# 本地提交与归档计划

记录时间：2026-09-27T23:27:01.365098-05:00

授权：当前用户要求提交并归档本组设置任务。只暂存下列任务文件。提交渠道为本地 git commit -F，不推送。

消息采用中文正文和现有仓库 scope。功能提交含 Why；两项工作提交均保留 Agent-Task、Agent-Model 与 Generated-By trailer，不添加 [AI] 标记。

## 1. 功能提交

```text
feat(desktop): ✨ 优化设置导航、字体和主题

Why: 提供独立设置入口、可搜索的主机字体和完整的共享配色。
将设置放在状态右侧，统一选择控件和主题预览。
接入 DirectWrite 字体目录与 V2 偏好迁移，保持主窗口和 HUD 同步。
修复中文组合输入误保存字体，补充回归测试和规范。
just ci 因核心测试程序缺失受阻，原因未查明；真实 Windows 输入法候选窗口尚未人工验收。

Tested: 前端 387 项、Node 5 项、桌面 Rust 83 项、Clippy、Windows 构建及原生矩阵通过
Agent-Task: 09-26-desktop-settings-experience
Agent-Model: gpt-6
Generated-By: agent
```

文件清单：67 项

- .trellis/spec/backend/database-guidelines.md
- .trellis/spec/backend/desktop-preferences.md
- .trellis/spec/backend/index.md
- .trellis/spec/desktop-frontend/component-guidelines.md
- .trellis/spec/desktop-frontend/index.md
- .trellis/spec/desktop-frontend/state-management.md
- .trellis/spec/desktop-frontend/type-safety.md
- Cargo.lock
- code_map.md
- crates/devsweep-core/src/desktop_preferences/legacy.rs
- crates/devsweep-core/src/desktop_preferences/mod.rs
- crates/devsweep-core/src/desktop_preferences/store.rs
- crates/devsweep-core/src/desktop_preferences/tests.rs
- desktop/package-lock.json
- desktop/package.json
- desktop/public/THIRD_PARTY_NOTICES.txt
- desktop/scripts/check-palette-contrast.mjs
- desktop/scripts/generate-types.mjs
- desktop/src-tauri/Cargo.toml
- desktop/src-tauri/src/desktop_preferences.rs
- desktop/src-tauri/src/fonts.rs
- desktop/src-tauri/src/hud.rs
- desktop/src-tauri/src/lib.rs
- desktop/src-tauri/src/service_boundary.rs
- desktop/src-tauri/src/tray.rs
- desktop/src-tauri/src/wire_parity.rs
- desktop/src/App.test.tsx
- desktop/src/App.tsx
- desktop/src/api/bridge.test.ts
- desktop/src/api/bridge.ts
- desktop/src/api/contract.ts
- desktop/src/api/fixtures/contract-variants.json
- desktop/src/api/fixtures/desktop-fonts-unavailable.json
- desktop/src/api/fixtures/desktop-fonts.json
- desktop/src/api/fixtures/desktop-preferences-patches.json
- desktop/src/api/fixtures/desktop-preferences-themes.json
- desktop/src/api/fixtures/desktop-preferences-updated.json
- desktop/src/api/fixtures/desktop-preferences.json
- desktop/src/api/types.gen.ts
- desktop/src/app-shell/AppShell.test.tsx
- desktop/src/app-shell/AppShell.tsx
- desktop/src/components/SettingsCombobox.tsx
- desktop/src/components/SettingsSelect.tsx
- desktop/src/components/settings-choices.test.tsx
- desktop/src/hud/hud.css
- desktop/src/main.tsx
- desktop/src/modes/analyze/styles.css
- desktop/src/modes/clean/styles.css
- desktop/src/modes/optimize/styles.css
- desktop/src/modes/software/styles.css
- desktop/src/modes/status/styles.css
- desktop/src/preferences/FontPicker.tsx
- desktop/src/preferences/PreferencesProvider.tsx
- desktop/src/preferences/SettingsPage.tsx
- desktop/src/preferences/appearance.css
- desktop/src/preferences/appearance.ts
- desktop/src/preferences/fixture.ts
- desktop/src/preferences/fonts.test.tsx
- desktop/src/preferences/fonts.ts
- desktop/src/preferences/palettes.test.tsx
- desktop/src/preferences/preferences.test.tsx
- desktop/src/stage/styles.css
- desktop/src/styles.css
- desktop/src/styles.test.ts
- desktop/src/support/styles.css
- resources/i18n/en.json
- resources/i18n/zh-CN.json

## 2. 验收记录提交

```text
docs(task): 📝 保存设置改造验收记录

记录四个子任务的规划、实现、独立复审与原生验证证据。
用户在获知验证限制后要求提交并归档；保留核心 CI 阻塞和人工输入法未验收状态。

Agent-Task: 09-26-desktop-settings-experience
Agent-Model: gpt-6
Generated-By: agent
```

文件清单：151 项

- .trellis/tasks/09-26-desktop-settings-controls/check.jsonl
- .trellis/tasks/09-26-desktop-settings-controls/design.md
- .trellis/tasks/09-26-desktop-settings-controls/evidence/check-report.md
- .trellis/tasks/09-26-desktop-settings-controls/evidence/ime-check-report.md
- .trellis/tasks/09-26-desktop-settings-controls/evidence/ime-repair-report.md
- .trellis/tasks/09-26-desktop-settings-controls/evidence/implementation-report.md
- .trellis/tasks/09-26-desktop-settings-controls/evidence/native-ime-before-fix.json
- .trellis/tasks/09-26-desktop-settings-controls/implement.jsonl
- .trellis/tasks/09-26-desktop-settings-controls/implement.md
- .trellis/tasks/09-26-desktop-settings-controls/prd.md
- .trellis/tasks/09-26-desktop-settings-controls/task.json
- .trellis/tasks/09-26-desktop-settings-experience/check.jsonl
- .trellis/tasks/09-26-desktop-settings-experience/design.md
- .trellis/tasks/09-26-desktop-settings-experience/evidence/ci-first-attempt.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/ci-first-attempt.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/ci.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/ci.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/clippy-corrected.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/clippy-corrected.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/clippy-final.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/clippy.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/closure-decision.md
- .trellis/tasks/09-26-desktop-settings-experience/evidence/commit-plan.md
- .trellis/tasks/09-26-desktop-settings-experience/evidence/core-gate-diagnostic.md
- .trellis/tasks/09-26-desktop-settings-experience/evidence/core-isolated-artifacts.jsonl
- .trellis/tasks/09-26-desktop-settings-experience/evidence/core-isolated-build.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/core-isolated.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/core-native-cargo.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/core-serial-diagnostic.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-build.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-build.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-build.pre-ime.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-build.pre-ime.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-test.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-test.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-web-check.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-web-check.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-web-check.pre-ime.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-web-check.pre-ime.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/integrated-source-final.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/integrated-source.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/integration-check-report.md
- .trellis/tasks/09-26-desktop-settings-experience/evidence/integration-report.md
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/native-composition-english-label-assertion.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/native-composition-no-explicit-commit.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/native-composition.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/native-flow-first.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/native-flow-restart.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/native-matrix.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/native-navigation.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/native-settings-screenshots.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/restart-expected.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/session-first.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/session.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/settings-accessibility-en.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/settings-accessibility-zh-CN.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/settings-catppuccin_latte.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/settings-catppuccin_mocha.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/settings-claude.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-final/settings-codex.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/closed-popup-inspection.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/native-composition-original-timing.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/native-composition.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/native-flow-first.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/native-flow-restart.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/native-matrix.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/native-navigation-harness-enter.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/native-navigation-harness-first.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/native-navigation-harness-hidden.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/native-navigation.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/restart-expected.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/session-first.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/session.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/settings-accessibility-en.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native-pre-ime/settings-accessibility-zh-CN.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/dialog-catppuccin_latte.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/dialog-catppuccin_mocha.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/dialog-claude.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/dialog-codex.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/hud-catppuccin_latte.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/hud-catppuccin_mocha.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/hud-claude.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/hud-codex.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/hud-custom-font.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/native-visuals.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/session-pre-ime.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/settings-catppuccin_latte.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/settings-catppuccin_mocha.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/settings-claude.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/native/settings-codex.png
- .trellis/tasks/09-26-desktop-settings-experience/evidence/preferences-final.log
- .trellis/tasks/09-26-desktop-settings-experience/evidence/progress.md
- .trellis/tasks/09-26-desktop-settings-experience/evidence/rust-focused-final.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verification-os.json
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/cdp.mjs
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/native-composition.mjs
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/native-flow.mjs
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/native-launch.mjs
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/native-matrix.mjs
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/native-navigation.mjs
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/native-settings-screenshots.mjs
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/native-visuals.mjs
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/run-gate.ps1
- .trellis/tasks/09-26-desktop-settings-experience/evidence/verify/settings-ui.mjs
- .trellis/tasks/09-26-desktop-settings-experience/implement.jsonl
- .trellis/tasks/09-26-desktop-settings-experience/implement.md
- .trellis/tasks/09-26-desktop-settings-experience/prd.md
- .trellis/tasks/09-26-desktop-settings-experience/research/approval.md
- .trellis/tasks/09-26-desktop-settings-experience/research/catppuccin-LICENSE
- .trellis/tasks/09-26-desktop-settings-experience/research/catppuccin-pinned.json
- .trellis/tasks/09-26-desktop-settings-experience/research/dependency-check.md
- .trellis/tasks/09-26-desktop-settings-experience/research/design-proposal.md
- .trellis/tasks/09-26-desktop-settings-experience/research/external-references.md
- .trellis/tasks/09-26-desktop-settings-experience/research/repository-findings.md
- .trellis/tasks/09-26-desktop-settings-experience/task.json
- .trellis/tasks/09-26-desktop-settings-host-fonts/check.jsonl
- .trellis/tasks/09-26-desktop-settings-host-fonts/design.md
- .trellis/tasks/09-26-desktop-settings-host-fonts/evidence/check-report.md
- .trellis/tasks/09-26-desktop-settings-host-fonts/evidence/focused-tests.json
- .trellis/tasks/09-26-desktop-settings-host-fonts/evidence/focused-tests.log
- .trellis/tasks/09-26-desktop-settings-host-fonts/evidence/font-interaction-measurement.json
- .trellis/tasks/09-26-desktop-settings-host-fonts/evidence/implementation-report.md
- .trellis/tasks/09-26-desktop-settings-host-fonts/implement.jsonl
- .trellis/tasks/09-26-desktop-settings-host-fonts/implement.md
- .trellis/tasks/09-26-desktop-settings-host-fonts/prd.md
- .trellis/tasks/09-26-desktop-settings-host-fonts/task.json
- .trellis/tasks/09-26-desktop-settings-navigation/check.jsonl
- .trellis/tasks/09-26-desktop-settings-navigation/design.md
- .trellis/tasks/09-26-desktop-settings-navigation/evidence/browser-navigation.json
- .trellis/tasks/09-26-desktop-settings-navigation/evidence/check-report.md
- .trellis/tasks/09-26-desktop-settings-navigation/evidence/implementation-report.md
- .trellis/tasks/09-26-desktop-settings-navigation/implement.jsonl
- .trellis/tasks/09-26-desktop-settings-navigation/implement.md
- .trellis/tasks/09-26-desktop-settings-navigation/prd.md
- .trellis/tasks/09-26-desktop-settings-navigation/task.json
- .trellis/tasks/09-26-desktop-settings-palettes/check.jsonl
- .trellis/tasks/09-26-desktop-settings-palettes/design.md
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/check-lint.txt
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/check-report.md
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/check-typecheck.txt
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/contrast.json
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/dialog-repair-report.md
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/final-focused-tests.json
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/focused-before-fix.txt
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/focused-tests.json
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/implementation-report.md
- .trellis/tasks/09-26-desktop-settings-palettes/evidence/production-freeze.json
- .trellis/tasks/09-26-desktop-settings-palettes/implement.jsonl
- .trellis/tasks/09-26-desktop-settings-palettes/implement.md
- .trellis/tasks/09-26-desktop-settings-palettes/prd.md
- .trellis/tasks/09-26-desktop-settings-palettes/task.json

## 3. 归档与日志

依次运行标准 Trellis archive 命令，每个任务产生一项归档提交：

- 09-26-desktop-settings-navigation
- 09-26-desktop-settings-controls
- 09-26-desktop-settings-host-fonts
- 09-26-desktop-settings-palettes
- 09-26-desktop-settings-experience

最后运行 add_session.py 记录工作提交和验证限制。

未识别的脏文件：无。忽略的 target/dist 构建产物不在提交清单中。所有候选文件均小于 1 MB；未发现密钥扩展名或安装包候选。

## 入库格式处理

以下四个文本记录仅移除文件末尾多余空行。正文、诊断行和测试结果保持不变。原始字节副本保存在本地 Git 元数据目录中；表中记录前后 SHA-256。

| 路径 | 原始 SHA-256 | 入库 SHA-256 |
| --- | --- | --- |
| .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-build.log | 9ed2e37d1b9c1d0bdc88f5f516be27f8a1976eac63904625917890b9ce8baf84 | 3ac3f4c4a86b51f35643505d1cced61f65b650b042eb0a2e78254fe14a6d657a |
| .trellis/tasks/09-26-desktop-settings-experience/evidence/desktop-build.pre-ime.log | 343b4f3099da586a0b676d6eee264623326c5e98575c14cefe69e1f7a760e916 | bcfff7ac6e0c507ba4d4a8e0211e389e0af90da5bce3bd942691ca779e7e131e |
| .trellis/tasks/09-26-desktop-settings-experience/research/catppuccin-LICENSE | 473ccad090584c2f9ac65edb73e42d8b8ee48090f285a97cc437f08c45fafcb5 | 814096d2c34cc216c624738a49356f32b7237733b4f7edb0685f4e50ef5074ba |
| .trellis/tasks/09-26-desktop-settings-palettes/evidence/focused-before-fix.txt | bebe6f30c7ee5a0ace6f31ca1239c3b22e32b93582ce9bff1d0366d85020e652 | 54a4e9ddab2eed028c2077828c719ca0ba23589a77488a8418e8dc43aef0a732 |
