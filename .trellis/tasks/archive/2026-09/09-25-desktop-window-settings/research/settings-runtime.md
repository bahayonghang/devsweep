# Research: Desktop settings and runtime preferences

- Query: Identify useful Settings controls for theme, fonts, animation, Status, and HUD. Trace persistence, bounds, defaults, apply timing, compatibility, and runtime ownership. Separate application preferences from Clean and Analyze resource contracts.
- Scope: internal. Source inspection and planning only.
- Date: 2026-09-25 (host local date).
- Task: .trellis/tasks/09-25-desktop-window-settings.
- Assigned implementation owner: planning child 09-25-desktop-settings-preferences. Main-window controls belong to sibling 09-25-desktop-custom-window-controls.

## Findings

### Confirmed product decision

The user confirmed full dark, light, and system themes across all pages and the HUD. Dark remains the default. The main session relayed that decision during this research. Full light mode is accepted scope; the existing dark-only spec must change with implementation.

### Current settings route and persistence

The app already has a supporting Settings route at #/settings. The route renders a language selector inside AppShell. The brand-menu label currently presents the route as Language. Extend that route into a real Settings page; do not add a sixth primary mode. Sources: desktop/src/app-shell/AppShell.tsx:103, :426, :450-483; .trellis/spec/desktop-frontend/state-management.md:18-36.

Core owns an exact, closed, language-only V1 document:

- Path: %LOCALAPPDATA%/DevSweep/settings/presentation-v1.json.
- Fields: schema_version = 1; language = en, zh-CN, or null.
- Missing file returns the null-language default. Missing LOCALAPPDATA reports unavailable.
- Unknown fields, unknown language tags, malformed JSON, and unsupported versions fail decoding. Save validates existing bytes while holding the cross-process transaction lock and refuses to overwrite unsupported bytes.
- Save commits with a flushed same-directory temporary file and atomic replacement.

Sources: crates/devsweep-core/src/presentation_settings.rs:22-68, :159-223, :237-273, :421-465. The frozen public contract also appears in .trellis/spec/desktop-frontend/state-management.md:56-72.

Desktop IPC serializes settings work within the process and delegates file transactions to core. The frontend decoder requires exactly the language field. The frontend applies a language only after save succeeds and retains the prior locale on failure. Sources: desktop/src-tauri/src/commands.rs:22-59; desktop/src/i18n/index.ts:59-65, :155-180; desktop/src/App.tsx:96-118, :148-185.

The TUI shares the same core file. Its explicit session locale wins over the persisted choice, followed by the existing resolver. TUI persistence also writes the language-only type. Adding theme or performance fields to presentation-v1.json breaks old readers and the current TUI contract. Sources: crates/devsweep-cli/src/tui/shell/mod.rs:155-158, :231-244; crates/devsweep-core/src/presentation_settings.rs:49-61.

Recommendation: retain presentation-v1.json for shared interactive language. Add a separate, core-owned, closed DesktopPreferencesV1 document at %LOCALAPPDATA%/DevSweep/settings/desktop-preferences-v1.json. The name and path are proposals. Reuse the existing transaction semantics. Do not introduce a database, browser-local persistence, or a second persisted language field. A separate file requires no migration of valid V1 language bytes and remains compatible with existing TUI readers.

Use typed field updates under the document lock, or an equivalent read-modify-write contract, so stale windows cannot overwrite unrelated preference changes. Reject invalid stored enum values and out-of-range numbers; preserve corrupt or future bytes. Missing files can return defaults. Report save failures and keep the last committed UI values. Language and desktop preferences are separate transactions; the UI must not claim that a combined two-file save is atomic.

### Theme, font, and motion consumers

Main-window colors already use shared CSS tokens, but the root is dark-only. The root font stack is Segoe UI Variable, Segoe UI, system-ui, sans-serif. Body text is 14 px, and many navigation, table, and label rules use fixed pixel sizes. Sources: desktop/src/styles.css:1-42, :59-100, :140. A font-size setting that changes only the root rem size will leave those controls unchanged.

The HUD has an independent CSS root, independent color tokens, a Segoe UI stack, and 13 px body text. It is a separate React entry. Sources: desktop/src/hud/hud.css:1-24; desktop/src/hud/main.tsx:12-29. Full theme and font support therefore needs a shared appearance application contract consumed by both entries. A main-window CSS class alone cannot satisfy the confirmed scope.

Current HUD language loads once at entry startup, with a fallback to null language after load failure. Tray menu copy is chosen during setup. Tray tooltip copy is refreshed when sampling starts or the HUD hides. Current code does not provide a general live settings broadcast. Sources: desktop/src/hud/main.tsx:12-24; desktop/src-tauri/src/tray.rs:150-168, :274-307. Main-window settings load failure instead displays the unavailable gate: desktop/src/App.tsx:109-112. Preserve and document these existing language behaviors unless the new task explicitly changes them.

Recommendation: load and apply committed desktop preferences before rendering both entries. After a successful settings write, notify both windows and have each consume the authoritative settings snapshot through a closed decoder. Handle a HUD created after a save, an already open HUD, and out-of-order load/save results. System theme changes must update both entries while system mode is selected. Forced colors must remain an accessibility override.

The decorative Planet is a real animation consumer. It caps drawing at 30 FPS, rotates once per 96 seconds, and uses capped source geometry. It draws one still frame when inactive or reduced motion is selected by the OS. It stops its frame loop while the document is hidden and on cleanup. Sources: desktop/src/stage/Planet.tsx:13-16, :58-83, :90-124; desktop/src/stage/planet-renderer.ts (MAX_SOURCE_DIAMETER owner). The current media-query result is read inside an effect whose dependencies are mode and active; a changed app motion preference must trigger an actual update. CSS also has reduced-motion rules at desktop/src/styles.css:236-237 and desktop/src/stage/styles.css:186-190.

### Compact configuration candidate matrix

All new names and option sets below are recommendations, unless marked existing or confirmed. Defaults preserve current behavior.

| Candidate | Options / validation | Default | Real consumer and owner | Apply timing / scope |
| --- | --- | --- | --- | --- |
| Language (existing) | en, zh-CN; null already means resolver default | null in a missing file | Shared core language store; desktop i18n and TUI | Keep existing successful-save behavior. An optional Follow system choice requires tracking stored selection separately from resolved locale; App currently tracks only the resolved locale. |
| Theme (confirmed) | dark, light, system | dark | Shared appearance tokens in main and HUD; core persists the tag | Apply after successful save in both windows; react to OS theme changes in system mode. Include every mode, support page, errors, dialogs, and HUD. |
| UI font family | Closed local presets; proposed system/current, Segoe UI, Microsoft YaHei UI. Every preset has a system fallback. No file import or font enumeration in initial scope. | current stack | Shared appearance application in both entries | Immediate after commit. Keep font loading local. Preserve full Chinese/English fallback and tabular numeric rendering. |
| UI text scale | Proposed 100%, 110%, 125%; validated closed values | 100% | Font-size tokens for labels, controls, tables, and headings in both entries | Immediate after commit. Convert relevant fixed font sizes to the shared scale; retain distinct main/HUD base sizes. Do not change Windows display scale or use viewport width as font scale. |
| Motion | system, reduced | system | Planet animation and existing CSS transition consumers | Immediate. OS reduced motion always wins; reduced renders a still Planet and suppresses decorative transitions. |
| Planet frame cap | 15 or 30 FPS; disabled while effective motion is reduced | 30 FPS | Planet.tsx frame scheduling | Immediate. Retain the existing source-resolution cap, 96 s turn duration, hidden-document stop, and forced-colors behavior. No 60 FPS option without a separate measured reason. |
| Status live interval | Existing UI choices: 1, 2, 5, 10, 30, 60 seconds. Core allows integral seconds from 1 to 60. | 2 seconds | StatusWorkbench -> bridge.statusLiveStart -> core start_live | Persist the default for the next explicit live start. Keep the current in-mode interval control synchronized. Existing active interval changes must cancel/join before replacement starts. |
| Status returned process rows | Core range 1-100; proposed UI choices 5, 15, 30, 50, 100 | 15 | Both statusSnapshot and statusLiveStart processLimit arguments; core process ranking/truncation | Next snapshot/live start. This is a real returned-row bound; do not promise a proportional collection CPU reduction. |
| HUD interval | Proposed 2, 5, 10 seconds; separate from Status live interval | 2 seconds | Rust HudSampler loop owned by HudState | Next show, or stop/join and restart if a visible HUD must apply immediately. Hidden HUD sampling remains forbidden. |
| Pause Status live while main window is hidden | Optional bounded application preference; false/true | false, preserving the inspected behavior | New Status-only visibility handling through the existing coordinator | On hide, cancel and join the live Status operation. Show a stopped/paused state and require explicit restart on return for the smallest initial contract. Never pause or cancel Clean execution through this setting. |

The first Settings page can group appearance and motion together, then Status/HUD sampling. Process rows and visibility can be advanced controls. Every included control must reach the named consumer in the same implementation; omit controls whose consumer work is deferred.

### Status bounds and lifecycle

Core uses a 500 ms snapshot measurement window, a 2,000 ms default live interval, and 1,000-60,000 ms live bounds. Intervals round down to a whole second before clamping. Process rows default to 15 and clamp to 1-100. Enumeration remains capped at 4,096 processes and process-detail work at 150 ms. Process data refreshes every 4,000 ms; volume/power data refreshes every 30,000 ms. Sources: crates/devsweep-core/src/status/mod.rs:24-42, :495-500, :529-536.

The frontend already exposes interval choices and restarts live work after a changed interval. The operation coordinator grants a lease and owns cancellation/join. Sources: desktop/src/modes/status/state.ts:22-25; desktop/src/modes/status/StatusWorkbench.tsx:262-321, :403-416. Keep one persisted interval owner; do not leave a Settings value and an unrelated mode-local default that disagree.

Both Tauri commands accept process_limit, and the frontend bridge already carries that argument. The current snapshot button omits the argument; live always passes DEFAULT_PROCESS_LIMIT. Both callers must change for a process-row preference to be effective. Sources: desktop/src/api/bridge.ts:168-180; desktop/src-tauri/src/status.rs:173-218; desktop/src/modes/status/StatusWorkbench.tsx:295-310.

Core ranks and truncates the process group after detail collection and reports requested_limit, enumeration_ceiling, detail_budget_ms, and truncation reasons. Sources: crates/devsweep-core/src/status/process.rs:47-82, :252. A row-limit preference controls output and presentation. The preference must not be described as a CPU quota or a replacement for the fixed enumeration/detail bounds.

No Status hidden-document handling was found in the inspected StatusWorkbench, App, or lifecycle modules. The lifecycle bridge currently exposes close requests and a debug fault mode. Sources: desktop/src/lifecycle.ts:11-35; desktop/src/modes/status/StatusWorkbench.tsx:253-321. Hidden-window pause therefore requires new behavior. Do not infer native minimization behavior from browser inspection alone; the window-control child owns the native boundary and evidence.

HUD sampling is separate from the main Status coordinator. HudSampler owns at most one thread, stores an immutable interval at construction, and stops through cancellation and join. The tray passes a process limit of 1 even though the HUD displays no process list. Sources: desktop/src-tauri/src/hud.rs:19-20, :38-74, :106-129; desktop/src-tauri/src/tray.rs:40-47, :267-307. Do not connect the Status row-limit preference to HUD collection. An adjustable HUD interval needs a real sampler input/update path. The existing hidden-HUD stop is an invariant, not an optional battery setting.

### Fixed Clean and Analyze budgets

Do not expose these constants as preference sliders:

| Contract | Current source and value | Reason to retain |
| --- | --- | --- |
| Clean scan input | ScanOptions contains include_projects, include_global, roots only. crates/devsweep-core/src/scan/mod.rs:159-169 | No performance field currently travels through the scan request. A UI-only speed mode would have no consumer. |
| Clean sizing | Default entry budget 50,000; dedicated pool uses 2 workers; compile-time ceiling 4; serial fallback; depth argument 64. crates/devsweep-core/src/filesystem/sizing.rs:62-87, :109-122 | These values belong to bounded no-follow filesystem observation. A global CPU-thread slider would require a changed execution contract, not a preference-only patch. |
| Reviewed Clean rescan | 500,000 entries via default * 10, for a target id from the current plan. crates/devsweep-core/src/scan/project/mod.rs:668-705 | Do not turn the reviewed target-specific path into arbitrary-path or unlimited scanning. |
| Analyze caps | 250,000 nodes; 10,000 warnings; 256 MiB accounted memory; logical depth 1,024; 2 workers. crates/devsweep-core/src/analysis/model.rs:8-18 | Frozen V1 caps; comments explicitly forbid user flags raising node/warning limits. |
| Analyze delivery | 256-node batches; queue capacity 4; 100 ms progress interval. crates/devsweep-core/src/analysis/model.rs:19-24 | Backpressure and truthful partial results belong to the traversal contract. |
| Status internals | 500 ms snapshot window; 4 s process refresh; 30 s volume/power refresh; 4,096 process ceiling; 150 ms details. crates/devsweep-core/src/status/mod.rs:24-42 | Expose the supported live interval and row request. Keep lower-level collection limits fixed. |

No setting may bypass Inspect Only, change default cleanup authority, skip a dry run, disable confirmation, enable permanent delete, disable no-follow checks, or raise executor authority. Scanner and Analyze preferences must never construct cleanup actions. Related contracts: .trellis/spec/desktop-frontend/state-management.md:88-100; .trellis/spec/backend/quality-guidelines.md:189-206, :539-541.

### Ownership and compatibility recommendation

1. Core owns the new desktop preference DTO, validation, defaults, fixed path, and atomic persistence. Keep language in its current store. The new store contains application preferences only.
2. Tauri owns get/update commands, process-local serialization, and notifications to main/HUD windows. Rust remains the source of validation truth. Components use the bridge and closed decoder.
3. The Settings page owns editable state and save/error presentation. A shared appearance adapter applies theme/font/motion to each document. Mode reducers retain domain results and operation identities.
4. Status consumes committed interval/row preferences at operation start. HUD consumes its own interval through HudState. Changing runtime parameters must preserve cancel-and-join and stale-event rejection.
5. Existing exact V1 language fixtures remain valid. New preference fixtures cover missing defaults, valid tags, malformed/unknown/newer bytes, rejected bounds, concurrent field updates, and successful-save application. These are implementation acceptance recommendations; no tests were run during research.

New copy belongs in the existing English/Chinese catalogue pair, imported by desktop/src/i18n/index.ts:1-6. Preserve binary units, accessible labels, and catalogue-owned metadata. The pure appearance settings must not alter Cleanup Plans, digests, or machine-readable output.

### Relationship to existing performance and acceptance tasks

The operation-performance task remains in progress and owns measured lifecycle, backpressure, renderer, and cancellation defects. Its PRD requires current release evidence, raw 200 ms sampling, at least five repeats, real paired Clean baselines, and same-live-PID Status post-stop evidence. It prohibits threshold changes and authority changes. Sources: .trellis/tasks/09-20-desktop-operation-performance/task.json:6; prd.md:5-16, :20-63; design.md:139-158 in the same task.

The settings child owns user-selectable appearance and bounded sampling preferences. Adding a 15 FPS option or longer HUD interval does not prove that existing performance failures are fixed. Existing performance gates must still use the required default workload and record preference values. Do not select reduced motion or a slower interval merely to make an old default-performance row pass. If implementation discovers a general coordinator/backpressure defect, coordinate with the existing performance owner.

The native-acceptance task remains in progress and owns final native Windows evidence against a frozen binary. It consumes performance evidence and routes product defects to owning children. Its current PRD records a measured idle thread count of 45 against a maximum of 40, with operator rows OP-1 to OP-13 still open. Sources: .trellis/tasks/09-20-desktop-native-acceptance/task.json:6; prd.md:5-16, :20-52, :100-109; evidence/operator-checklist.md:11-23 in the same task. These are recorded findings; no native measurement was repeated here.

New settings need their own functional acceptance for both windows, both languages, all three themes, font scale, save failures, restart, and runtime consumers. The new settings task cannot close the older performance/native tasks. A changed binary also cannot inherit their old artifact-bound evidence without the owning task deciding which rows require a new run. The existing rule against changing the user's Windows display scale remains in force: .trellis/tasks/09-20-desktop-native-acceptance/prd.md:33-40.

### Related specs and explicit changes needed

- .trellis/spec/desktop-frontend/component-guidelines.md:6-8 still requires native Windows chrome; the window-control child handles the accepted custom-control change.
- The same file :136-145 fixes the Segoe UI stack and dark-only canvas; :170-174 fixes Planet bounds and accessibility behavior. The settings child must update the appearance contract for the confirmed full theme/font support, while retaining accessibility and upper animation bounds.
- .trellis/spec/desktop-frontend/state-management.md:56-72 owns the exact shared locale store. Keep that contract and add the separate desktop preference contract.
- .trellis/spec/desktop-frontend/type-safety.md defines Rust DTO ownership and closed IPC decoding.
- .trellis/spec/backend/database-guidelines.md:69-87 requires version ownership, compatibility tests, rollback behavior, and explicit paths. Its overview at :9-12 predates the current presentation store; treat the current source and the specific presentation spec as stronger evidence.
- Root CONTEXT.md supplies Cleanup Target, Scan Preview, Scan Report, Cleanup Plan, Estimated Recoverable, and Inspect Only terminology. Settings copy must preserve those meanings.

### Files found

| File | Role |
| --- | --- |
| crates/devsweep-core/src/presentation_settings.rs | Shared language DTO and transactional file owner. |
| crates/devsweep-cli/src/tui/shell/mod.rs | TUI locale resolution and persistence adapter. |
| desktop/src-tauri/src/commands.rs | Presentation-setting IPC and serialization. |
| desktop/src/i18n/index.ts | Locale resolver, catalogue consumption, closed setting decoder. |
| desktop/src/App.tsx | Initial settings gate, generation checks, successful-save application. |
| desktop/src/app-shell/AppShell.tsx | Settings route and current language-only content. |
| desktop/src/styles.css | Main-window palette, fonts, sizes, accessibility rules. |
| desktop/src/stage/Planet.tsx | Decorative frame loop and visibility/reduced-motion behavior. |
| desktop/src/hud/main.tsx, hud.css | Independent HUD settings load, appearance, and entry. |
| desktop/src/modes/status/StatusWorkbench.tsx, state.ts | Status interval UI, operation lease, current hardcoded row count. |
| desktop/src/api/bridge.ts | Typed Status command arguments and event decoding. |
| desktop/src-tauri/src/status.rs | Status snapshot/live command adapters. |
| desktop/src-tauri/src/hud.rs, tray.rs | HUD cadence, single sampler, visibility stop, and tray copy. |
| crates/devsweep-core/src/status/mod.rs, process.rs | Status bounds and returned-process truncation. |
| crates/devsweep-core/src/filesystem/sizing.rs | Clean sizing worker pool and entry bounds. |
| crates/devsweep-core/src/scan/mod.rs, project/mod.rs | Closed scan request and reviewed rescan budget. |
| crates/devsweep-core/src/analysis/model.rs | Frozen Analyze caps and progress bounds. |

### External references and versions

No external documentation was required for the repository-behavior findings. No unverified framework API is proposed here. Repository manifests declare React ^19.1.1, @tauri-apps/api ^2.8.0, Tauri CLI 2.11.4, and Rust tauri dependency 2.11.3. These are manifest declarations, not claims about the newest published or installed versions. Sources: desktop/package.json:22-35; desktop/src-tauri/Cargo.toml:17-25. The window-control researcher owns external native-window API evidence.

### Questions that require user choice

The theme scope and dark default are already resolved. The repository supports conservative defaults for local font presets, text scale, motion, intervals, and row limits. None of those needs a planning blocker.

One optional product choice remains if hidden-window pausing is included: resume Status automatically when the main window returns, or leave Status stopped until the user starts live sampling again. The minimal recommendation is explicit restart, with the preference disabled by default. Automatic resume requires retained user intent, operation-generation checks, and coordination with the window lifecycle child. Do not infer permission to pause cleanup work from either choice.

## Caveats / Not Found

- Research wrote only this report. No product code, task records, specs, tests, builds, git operations, native operations, or user settings were changed.
- The report describes inspected source and recorded task evidence. No latency, CPU, memory, visual, or native-minimization result was measured.
- No existing desktop preference document, theme/font selector, runtime HUD interval setter, or general live appearance notification was found in the targeted files.
- System theme and custom font behavior require work across all main modes, supporting pages, and the separate HUD entry. Existing dark-only fixtures and visual assertions require deliberate updates under the confirmed product decision.
- The existing language-only V1 decoder is strict in core and frontend. Editing the existing V1 file shape without a coordinated compatibility design would be a breaking change.
- Native hide/minimize signals and close behavior are owned by the sibling window-control work. A visibility preference must use that verified boundary and must not weaken operation ownership.
