# Research: Clean home, scope lifecycle, and brand integration

- Query: Trace the current Clean home and identify the smallest safe boundary for independent global-cache and project-artifact workflows, clearer results/warnings, and an original animal brand.
- Scope: internal; screenshot observations separated from source-confirmed behavior. No product edits or runtime tests.
- Date: 2026-10-14

## Findings

### Confirmed planning decisions received during research

The main session relayed the user's chosen policy: warn only after a user-triggered scan when a single project's artifact total is **at least 5 GiB**, with a configurable threshold. The design direction is `design-md` **style:shadcn**, customized as a compact utility workbench with two independent entry panels and retained scope results, sharing one operation coordinator. Clean loses its large Earth hero; other modes are outside this visual rewrite. The parent will store the draft at `design-system/DESIGN.md` inside this task and eventually publish the app design at `desktop/DESIGN.md`, preserving historical root `design.md`. The original mascot direction is a small beaver; image generation is owned by the parent. These decisions supersede the optional threshold/style choices discussed below; implementation still needs an exact aggregation/unknown-size contract.

### 1. The reported scan-entry problem is confirmed in source

The scanner already accepts two scope flags. The home UI combines them into one form and one action, making a project setup prerequisite appear to apply to global caches.

| Evidence | Current behavior | Consequence |
| --- | --- | --- |
| `desktop/src/modes/clean/useScanScope.ts:8` | Initializes `{ include_projects: true, include_global: true, roots: [] }`. | Fresh setup begins with Projects selected but no usable project root. |
| `desktop/src/modes/clean/useScanScope.ts:23` | Initial preferences load changes only roots; flags are local state. | Flags are not persisted. A remount restores both flags. |
| `desktop/src/pages/ScanPage.tsx:98` | Projects and Global caches are adjacent checkboxes. | Their different readiness/cleanup semantics are flattened into one choice set. |
| `desktop/src/pages/ScanPage.tsx:104` | The sole Scan button is disabled whenever Projects is enabled and roots are empty. | Global-cache discovery is blocked until the user either adds a project root or unchecks Projects. |
| `desktop/src/modes/clean/CleanWorkbench.tsx:70` | Handler repeats the same project-root guard. | Styling the button differently alone does not fix the behavior. |
| `desktop/src-tauri/src/commands.rs:185` | Explicit absolute roots are required only when `include_projects` is true. | Global-only requests already have a valid backend path; do not remove project validation. |
| `desktop/src/modes/clean/coverage-results.test.tsx:19` | Existing test explicitly verifies combined empty-roots is disabled and global-only is enabled. | This is an old product contract, not evidence that global scanning itself is broken. Update the entry contract and its tests together. |

The current Chinese help says to select a folder or turn off Projects (`resources/i18n/zh-CN.json:21`). Its wording is accurate for the present implementation but forces the user to understand a dependency the new product should remove.

### 2. Root selection has safety and persistence behavior worth retaining

- `desktop/src/modes/clean/ScanRoots.tsx:12` renders a disabled fieldset while busy. It accepts a typed absolute path, lists removable selections, and exposes existing-folder suggestions. There is no folder-picker bridge in this component.
- `desktop/src/modes/clean/useScanScope.ts:43` validates roots before saving and fences overlapping writes. The generation/revision checks prevent late reads from overwriting a newer edit. A successful validation followed by save failure leaves roots usable for that session (`:55`–`:61`).
- `crates/devsweep-core/src/scan_preferences.rs:48` validates existing directories and each ancestor with no-follow/reparse safety. `:84` normalizes and deduplicates exact roots. Nested selections remain meaningful; do not silently convert removal of a root into an exclusion rule.
- Suggestions are metadata-only conventional containers, not recursive discovery or implicit selection (`crates/devsweep-core/src/scan_preferences.rs:65`). Stored roots can be loaded even after disappearing (`:37`), so project coverage must still disclose missing/unreadable roots when scanned.
- Only explicit selected roots are persisted to the separate clean-root store; do not piggyback scope, threshold, report, or warning state onto appearance settings without defining its ownership/schema.

### 3. Existing results expose scope, but the completed view uses a different hierarchy

- Streaming observations already split `scope.type === "project"` and `"global"` (`desktop/src/pages/ScanPreviewPage.tsx:58`), with separate tables (`:29`). They carry no selection callbacks.
- Completed review groups by `kind + scope.type` (`desktop/src/modes/clean/impact.ts:55`), then sorts by impact rank, verified bytes, lower-bound bytes, kind, and scope (`:70`). This can interleave global and project groups. It does not group projects by `scope.root`.
- `desktop/src/api/types.gen.ts:1209` already exposes the authoritative project root. Use that value for presentation grouping; do not infer a project identity by splitting `target.path` in JavaScript.
- Review's selected estimate and select-all cover the whole completed report, while search filters displayed rows only (`desktop/src/pages/ReviewPage.tsx:74`, `:83`, `:108`). Retain this distinction within whichever scope owns the current report.
- `desktop/src/components/TargetTable.tsx:71` caps rendered rows at 100 per table page. Evidence mounts on disclosure (`:58`). `desktop/src/state/large-results.test.ts:28` protects full-report selection for 10,000 targets. Any new project grouping must preserve full-report search/selection and bound both rows and the number of simultaneously mounted project groups.
- `desktop/src/modes/clean/ScanCoverage.tsx:15` handles incomplete coverage, provider/root statuses, and diagnostics independently of target totals. Partial zero-target reports must never look like a clean bill of health.
- Current `sizing_warnings` concern measurement failures/budgets, not capacity-too-large advice (`desktop/src/api/types.gen.ts:581`). Current risk badges are cleanup risk (`TargetTable.tsx:122`). A new oversized-artifact warning must be a separate concept, not reuse risk or sizing-error tags.

### 4. Recommended two-entry contract

Recommended initial scope, for parent planning rather than an implementation change:

1. **Global caches:** a visible independent panel/action, ready without project roots. It submits `include_global=true`, `include_projects=false`, and an empty roots list. Show provider identity, supported action/inspect-only state, coverage, measurement confidence, and estimated capacity. “Directly cleanable through DevSweep CLI” means an available trusted cleanup workflow; it does not bypass saved-plan/live-preview/confirmation rules.
2. **Project artifacts:** an independent panel/action with explicit root setup. It submits `include_projects=true`, `include_global=false`, using the validated selected roots. Group its report by authoritative project root, then artifact kind. Make the rebuild/reinstall cost and optional nature of cleanup visible. Warnings attract attention but never auto-select or execute targets.
3. Use one shared heavy-operation coordinator; two entry points do not imply simultaneous scans. Show the active scope and an explicit cancel state. Until a replacement contract is tested, disable the other start action while a scan is active rather than silently canceling it.
4. Keep results scoped. The straightforward presentation is one Clean controller with two scope-local report slots and one active operation identity. Do not merge separately returned plans, selected IDs, or digests into synthetic authority. Each cleanup review/dry-run/confirmation belongs to one completed report.
5. Specify selection policy in the parent PRD. If “on demand” means project artifacts start unselected, change the authoritative policy and cross-surface expectations together. The current frontend must project `selected_by_default` rather than inventing its own safe-to-delete decision (`desktop/src/state/app-state.ts:96`; desktop state spec `:122`).
6. Implement the confirmed **single-project aggregate >= 5 GiB** configurable warning, evaluated only after a user-triggered scan. Before coding, finish the exact contract for verified versus lower-bound comparison, unknown-size behavior, threshold persistence/validation, aggregation/deduplication, and whether report timestamps are needed. Do not invent a percentage-of-disk warning without a known denominator. A conservative rule can warn when a verified aggregate or measured lower bound reaches the configured threshold; otherwise show measurement limitations, not “small/safe”. Backend research must resolve shared Cargo/mbx ownership and double counting. No background watcher, scheduled scan, or notification service is implied.

### 5. State, cancellation, and digest hazards

- `CleanWorkbench.tsx:75` dispatches `scan_requested` before waiting for coordinator cancellation. `desktop/src/state/app-state.ts:137` clears current selection, dry run, and execution, retaining the last completed report only as a hidden fallback. Preserve this ordering for each scope.
- Progress uses opaque scan IDs and increasing sequence numbers (`app-state.ts:157`). Cancel is requested separately, and completion is still backend-authoritative (`:170`–`:181`). Stopped previews are read-only (`:182`–`:218`). Never install a stopped partial preview as the other scope's report.
- `desktop/src/state/operation-coordinator.ts:32` serializes starts; `:102` cancels and joins the old operation before replacement. It must remain unaware of plans and digests.
- `useScanScope.ts:37` invokes `scope_changed`; `app-state.ts:135` resets all Clean state when idle. With two scope-local results, editing project roots should invalidate the project report and pending authority immediately, without erasing an unrelated global result. Define this explicitly instead of copying the current single-report reset unchanged.
- `CleanWorkbench.tsx:136` captures one plan and selected IDs for dry run; `:168` copies the current returned digest for execution. `app-state.ts:261` rejects dry-run results without a pending request and matching selection; `:269` gates execution on confirmation and compares returned digest/selection. New scope switching and concurrent UI paths need explicit scope/report/operation identity fencing, since two scopes can have matching selection shapes even when one response is stale.
- Existing tests cover cancel-before-native-start while another mode joins (`CleanWorkbench.test.tsx:522`), default-selected dry run/confirmation (`:247`), and protection invalidation (`:427`). Preserve these meaningful tests and add scope-specific cases.
- **Source-level retention discrepancy:** the spec promises mode state across route/language changes (`.trellis/spec/desktop-frontend/state-management.md:27`), but `AppShell.tsx:474` renders only the active adapter; the registry creates a `CleanWorkbench` element (`registry.ts:29`) whose reducer and scope state are local (`CleanWorkbench.tsx:51`). Switching to a different component type unmounts it. There is no persistent external Clean controller in this path. This strongly indicates route transitions reset non-persisted reports/flags; runtime reproduction was not performed here. Include a real Clean → Settings/other mode → Clean test before choosing the state-retention implementation. Preserve domain ownership in Clean rather than moving plan authority into shell navigation.

### 6. Screenshot and style evidence

**Source-confirmed:**

- The dominant Earth/layout follows the current design: `Stage.tsx:43` always supplies `Planet` unless a hero is passed. `stage/styles.css:2` sets a 232 CSS-pixel planet; `clean/styles.css:22` makes the large viewport layout a 180–280px hero column plus a 560px control column. DPI, text scale, and window size can affect screenshot proportions; screenshot pixels are not CSS pixels.
- A concrete input-style mismatch exists: `ScanRoots.tsx:18` omits `type`, while `styles.css:91` styles only `input[type="search"]` and `input[type="text"]`. HTML's default text behavior does not create a matching type attribute. The root field therefore misses the intended background, border, min-height, and padding rules; the screenshot's gray native-looking input is consistent with this. Fix the selector/markup contract in the redesign and verify computed styles.
- The first action is visually dominant but disabled, while the visible ready status remains “Ready”; this is a hierarchy/copy problem even though the conditional behavior is intentional.

**Unverified from screenshot alone:**

- The Chinese and Latin text look serif-like. The declared system stack is `"Segoe UI Variable", system-ui, "Microsoft YaHei UI", sans-serif` (`preferences/appearance.ts:22`; `appearance.css:28`). Saved installed fonts intentionally override it (`appearance.ts:25`, `:46`). It is not justified to label the user's font preference a bug or silently reset it. Native acceptance should record committed preference, computed font family, available family/fallback, and text scale, then assess the layout using both the default and a non-default saved font.
- No native screenshot recreation, actual Windows scale transition, renderer performance measurement, or user settings-store read was done in this research.

### 7. Spec conflicts that must be resolved deliberately

| Current rule | Source | Required planning decision |
| --- | --- | --- |
| One stage-first hero/state/action stack; first screen has no cards | `.trellis/spec/desktop-frontend/component-guidelines.md:60` | Allow a Clean-specific two-entry overview and define its compact information hierarchy. |
| The decorative planet is the only stage hero; Clean must show locally bundled geographic Earth | same file `:267` | Supersede Clean's mandatory Earth hero if the new animal brand or information-first layout replaces it. Do not rewrite other modes' planet lifecycles without scope. |
| No sidebar or page header block; mode h1 is visually hidden | same file `:8`, `:54` | Preserve capsule navigation by default; explicitly revise the Clean header rule only if the chosen design needs a visible title. Avoid silently introducing a whole-shell navigation rewrite. |
| Original product icon plus name; original inline semantic glyphs; no image sprite | same file `:13`, `:90`, `:264` | Use ImageGen for an original mascot/app mark, while preserving clear vector semantic controls unless an explicit icon-system exception is selected. |
| No copied Mole assets | same file `:293`; desktop index completion checklist | “Reference Mole” can guide friendliness, simplicity, and utility positioning. Create a distinct animal silhouette/palette/props; do not import its mark or treat its branding as project assets. |
| Preserve saved font/theme, shared tokens, seven theme choices, high contrast, reduced motion | desktop component/state specs and backend desktop-preferences contract | Restyle through existing tokens and preserve preference values. Scope the new design to Clean and brand integration unless the PRD expands it. |

These are product-direction conflicts, not reasons to refuse the requested redesign. The main session should record the intended superseding contracts before implementation, then update matching tests rather than merely deleting assertions.

### 8. Brand and icon propagation

| File / consumer | Purpose / requirement |
| --- | --- |
| `desktop/src/assets/devsweep-icon-master.png` | Current webview brand master; import at `app-shell/AppShell.tsx:3`, rendered at `:381`. Adjacent DevSweep text supplies the name, so the image is decorative. |
| `desktop/src/styles.css:43` | Capsule brand layout; test new silhouette at the actual small icon size, not only a large concept image. |
| `desktop/src-tauri/tauri.conf.json:37` | Packaged 32px, 128px, 256px PNG plus ICO/ICNS bundle entries. |
| `desktop/src-tauri/icons/` | Includes icon.png, Windows StoreLogo and Square*Logo sizes in addition to configured bundle files; audit generated outputs before replacing. |
| `desktop/src-tauri/src/tray.rs:224` | Tray reuses `app.default_window_icon()`, so native/taskbar/tray appearance changes through the bundled icon set. |
| `desktop/src/app-shell/glyphs.tsx:3` | Closed semantic names for modes, support, target kinds, ecosystems, and risk; inline SVG with `currentColor`, aria-hidden, and fixed 16px viewbox. Keep coherent small-control marks. |
| `desktop/src/App.test.tsx:361` | Existing assertion hard-codes the master source path; update only if the intentional asset contract changes. |
| `docs/validation/five-mode-native.md:47` | Historical icon acceptance evidence. It does not validate a newly generated mascot. |

Recommended deliverables for the brand child: original animal concept sheet and chosen master, compact silhouette variant, transparent asset, light/dark/monochrome treatment where appropriate, measured crop/safe-area rules, deterministic native icon conversion, asset/provenance manifest, and native taskbar/tray plus webview screenshots. Image generation and native integration are distinct acceptance steps. No new mascot was generated in this research-only assignment.

### 9. Minimal implementation boundaries for parent/subtask planning

1. **Contract and design:** task artifacts, selected design-md reference, Clean-specific overview/flow, capacity-warning semantics, explicit spec amendments, and canonical bilingual catalogue copy (`resources/i18n/en.json`, `zh-CN.json`; imported by `desktop/src/i18n/index.ts:2`).
2. **Clean frontend:** `modes/clean/CleanWorkbench.tsx`, `useScanScope.ts`, `ScanRoots.tsx`, Clean-owned controller/reducer/selectors as necessary, `pages/ScanPage.tsx`, `ScanPreviewPage.tsx`, `ReviewPage.tsx`, `modes/clean/impact.ts`, local styles, and focused tests. Reuse TargetTable and confirmation components.
3. **Shared shell touch only where necessary:** brand asset consumer and a deliberately designed route-state owner/lifecycle seam if retention is repaired. Avoid a blanket Stage/Planet rewrite across Software/Optimize/Analyze/Status.
4. **Backend/API:** only the fields and operations required by the scope/ownership/warning policy. Existing ScanOptions can express separate scans already. If a wire field changes, update Rust models, sample fixtures, generated types, closed decoder, Rust wire parity, and CLI parity as one contract change; do not hand-edit `types.gen.ts`.
5. **Brand package:** master/derived assets, native icon set, provenance, and actual consumers. Add any build-time conversion tool only if current packaging cannot reproduce the set; no production dependency is implied by asset generation.

### 10. Proposed validation matrix

- **Fresh first launch:** no selected roots; global action enabled; project action explains setup; global request has exactly global=true/projects=false/roots=[]; no project traversal. Root setup/validation/save failure never blocks the independent global lane indefinitely.
- **Project flow:** only validated explicit roots; add/remove/nested/duplicate/nonexistent/reparse/corrupt-store cases; saved root reload; optional cleanup; inspect-only excluded; large lower-bound/verified/unknown values produce truthful warning text and never automatic execution.
- **Two scopes:** global and project reports retain distinct source/selection/digest identity; starting, replacing, canceling, or failing one cannot promote the other's observations. Project-root edits invalidate only matching authority. Explicit busy behavior prevents surprise replacement scans.
- **Lifecycle:** late progress and terminal events, rapidly requested scans, cancel while awaiting coordinator join, route switch/back, Settings/theme/font/locale changes, StrictMode replay, close/unmount. Reproduce and resolve route-state loss before promising retained results.
- **Authority:** inspect-only/select-all/skip/protect, changing scope/selection during preview, stale dry-run response, stale digest, second confirmation, and successful owner-command versus trash outcome semantics. No UI-created plan/action/digest.
- **Results:** scope then project-root grouping, exact provider labels, full-report search and selection, bounded pagination, lazy evidence, incomplete zero-target coverage, no unsupported “space freed” copy. Preserve existing 10k report/render budgets; do not substitute small fixtures for them.
- **Visual/accessibility:** 390/800/1024/1440 CSS widths, EN/zh-CN, all resolved palettes, 100/110/125% text, default plus committed installed font, keyboard/focus, long copyable paths, forced colors, reduced motion, all empty/error/loading/results/confirmation states. Check the formerly untyped input's computed styles.
- **Brand:** actual webview 16–32px visibility, native app/taskbar/tray, transparent edges and Windows icon sizes, high-DPI/forced-colors accessibility, no double spoken product name. Record native Windows evidence separately from web fixture screenshots and DPR emulation.
- **Gates:** Node >=22 <23 and npm >=10 <12 from `desktop/package.json:6`; generated-types check, lint, typecheck, Vitest/Node tests, build, and canonical `just ci` when shared/Tauri contracts change. Existing files to extend include `useScanScope.test.ts:15`, `coverage-results.test.tsx:19`, `ScanPage.test.tsx:61`, `CleanWorkbench.test.tsx:247`, `AppShell.test.tsx:405`, `large-results.test.ts:28`, and `styles.test.ts:101`.

## Related Specs

- `.trellis/workflow.md`: persist research and prepare reviewed planning artifacts before implementation.
- `.trellis/spec/desktop-frontend/index.md`: package gates, native evidence, scan-driver scope verification.
- `.trellis/spec/desktop-frontend/component-guidelines.md`: shell/stage/table/accessibility/visual rules and explicit conflicts above.
- `.trellis/spec/desktop-frontend/state-management.md`: lifecycle, operation coordinator, completed-report authority, digest invalidation, preference retention.
- `.trellis/spec/desktop-frontend/type-safety.md:38`: CLI and Tauri share typed core services; Tauri does not launch the CLI process. The user's CLI-cleanability requirement should preserve this architecture unless expressly redesigned.
- `.trellis/spec/backend/desktop-preferences.md`: committed system/installed font, themes, text scale, and preserve-corrupt-bytes contract.

## External References / Versions

No external documentation was needed for the source trace. Local declared versions: React ^19.1.1, Tauri API ^2.8.0, Tauri CLI 2.11.4, Base UI 1.8.0, TypeScript ~5.9.3, Vite ^7.1.7, Vitest ^3.2.4 (`desktop/package.json`). The parent reports selecting design-md `style:shadcn`; catalogue/reference analysis and original beaver generation are owned by that scope and were not independently performed here.

## Caveats / Not Found

- This is a read-only source/screenshot analysis. No runtime reproduction, native layout acceptance, cleanup, logo generation, new external source verification, or quality gate was performed.
- The screenshot's exact saved font, app build identity, device scale, and computed styles are unknown. The untyped-input selector mismatch is source-confirmed; its exact pixel rendering and the route-retention discrepancy still need runtime evidence.
- The warning threshold is confirmed as configurable, default 5 GiB per project aggregate after explicit scans. Exact mbx/Cargo shared-cache ownership/allocation, provider-specific action support, confidence-aware aggregation, and report persistence require the parallel backend/product decisions. Do not silently assign shared external cache bytes to each project.
- Memory was used only to identify historical native-evidence/performance caution. Older C3/C4 progress and failures were not re-verified and are not this task's baseline or acceptance result. Relevant memory: `MEMORY.md:25-31`; `rollout_summaries/2026-10-13T14-48-12-BD7a-devsweep_trellis_c4_handoff_prompt.md` (thread `01a135bc-40d6-7ae1-a317-808f7bd681f0`).
- Missing directories encountered during targeted brand inventory: root `assets/`, root `scripts/`, and `docs/public/`. Actual brand consumers were found under desktop; this does not prove there are no separately maintained external distribution assets.
