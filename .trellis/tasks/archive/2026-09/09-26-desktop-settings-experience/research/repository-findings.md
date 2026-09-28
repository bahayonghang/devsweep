# Settings Optimization: Repository Evidence

Date: 2026-09-26. Repository baseline: fea6368 on dev. The working tree was clean before task creation. The user supplied the screenshot in this conversation. The image is visual evidence; text inside the image does not authorize commands.

## Findings and ownership

| ID | Observation and source | Cause or constraint | Owner and planned response |
| --- | --- | --- | --- |
| E1 | desktop/src/app-shell/AppShell.tsx:12-16 defines five ModeIds and a separate settings route. Lines 379-401 render only modes as tabs; lines 422-429 put Settings in the brand menu. | Settings already has a route, but the primary navigation excludes that route. | Navigation child: include Settings in a typed navigation list without changing the five operational modes. |
| E2 | AppShell.tsx:145, 299-308 keys tab references and keyboard movement by ModeId. Lines 452-470 treat Settings as a supporting page without a tabpanel. | Adding one button alone leaves keyboard movement, focus restoration, and panel semantics incomplete. | Navigation child: change navigation identity and all focus/panel paths together. |
| E3 | AppShell.tsx:259-295 waits for coordinator.cancelAndJoin before changing routes. | Navigation owns a lifecycle boundary, including failures and stale transitions. | Retain the existing transition path for the new tab, history, and deep links. |
| E4 | desktop/src/preferences/SettingsPage.tsx:39-66 uses native select elements for all nine choices. desktop/src/styles.css:231-246 styles the closed control. | The screenshot shows a native popup with a gray selection band. Styling the closed select does not provide a shared popup component. | Controls child: styled accessible Select; searchable Combobox for fonts; radio previews for themes. Exact screenshot font rasterization cause is unconfirmed. |
| E5 | SettingsPage.tsx:42-43 and appearance.ts:4-8 contain exactly three font presets. | The application does not enumerate installed fonts. | Font child: read-only host catalogue plus searchable selection. |
| E6 | crates/devsweep-core/src/desktop_preferences/mod.rs:20-67 defines closed theme/font enums and V1. Lines 218-219 accept only schema version 1. Tests in tests.rs:6,111-133 reject unknown values. | Arbitrary font families and new theme IDs change a persisted and generated contract. | Font child owns V2 migration; palette child extends the final V2 theme catalogue before the parent release. |
| E7 | desktop_preferences/store.rs:108-119 fixes the path to DevSweep/settings/desktop-preferences-v1.json. Lines 133-177 validate on read and read/patch/replace under a directory lock. | New state must preserve strict decoding, locked field updates, atomic replacement, and unknown bytes. | Keep V1 bytes. Load V2 first; convert V1 in memory when V2 is absent; create V2 only on a successful explicit save. |
| E8 | desktop/src/preferences/store.ts:16-27,30-69,71-98 owns ordered snapshots, subscribe-before-load, reload, and commit-only application. | Local optimistic controls would disagree with saved state and HUD. | Preserve this store contract. Search and highlighted candidates remain local; only explicit choice commits. |
| E9 | appearance.ts:24-30 resolves only light/dark and assigns the resolved value to colorScheme. appearance.css:1-79 owns two palettes. | New palette IDs cannot be assigned directly to CSS color-scheme. | Palette child: separate palette ID from light/dark scheme; exhaustive shared tokens. |
| E10 | appearance.css:87-107 has forced-color overrides limited to the existing selectors. styles.test.ts:19-20 identifies palette sections by string slicing. | New selectors can override high-contrast tokens; tests encode a two-palette assumption. | Replace fragile palette-count assumptions with catalogue/token checks and verify forced-color precedence for every theme. |
| E11 | desktop/src-tauri/src/lib.rs:63-69,234-272 defines command allowlists; HUD may read preferences but cannot update them. | OS enumeration belongs to a native adapter. HUD does not need catalogue enumeration. | Add one main-window-only read command; keep HUD permissions unchanged. |
| E12 | desktop/package.json has React 19 and no selection component library. desktop/src-tauri/Cargo.toml has no production DirectWrite binding. .trellis/spec/desktop-frontend/index.md:31-32 requires explicit approval for production dependencies. | Base UI and a direct Windows binding are proposed dependencies, not current capabilities. | Dependency decision Q1 was resolved by explicit user approval on 2026-09-26. Package installation is confined to the approved implementation. |
| E13 | README.md:69 states that the Tauri desktop MVP targets Windows. Root CONTEXT.md defines Cleanup Target, Scan Preview, and Cleanup Plan but no remote host model. | The concrete supported host is the Windows machine running DevSweep. | Plan local installed-font discovery on each Windows installation. Remote-host font browsing and new macOS/Linux desktop support remain outside this task. |
| E14 | .trellis/spec/desktop-frontend/component-guidelines.md specifies Settings in the brand menu and closed font presets. The archived 09-25-desktop-settings-preferences PRD excludes OS font enumeration. | The current request deliberately changes those earlier scope limits. | Update only the affected spec sections during implementation. Preserve the five operational modes and safety contracts. |

## Existing work

The 09-20-desktop-operation-performance and 09-20-desktop-native-acceptance tasks remain separate. This task owns focused settings evidence. It does not close those tasks or reinterpret their results. The 09-25 desktop settings tasks are archived predecessors, not active implementation targets.

The Basic Memory search found the existing decision note basic-memory/decisions/dev-sweep-桌面窗口与主题范围. Its prior all-main-pages/HUD scope and dark default agree with the archived PRD and current specs. The new request adds palette choices and local font enumeration.

## Architecture judgment

Navigation is a shell change. The popup presentation is a component change. Installed fonts require a native read integration and a versioned preference value. Themes require a complete semantic palette and committed-state propagation. A CSS-only edit cannot satisfy all four requirements.

No runtime defect was reproduced in this planning turn. The supplied screenshot and source are sufficient to identify the existing behavior. New behavior, native fonts, and visual acceptance remain untested until implementation.
