# Evidence and design-atlas audit

## Evidence boundary

Baseline: dev at 63a4245, initially clean. The supplied screenshots show only
three current first screens; no later state or runtime action was inferred from
them. Screenshot sources are the three codex-clipboard PNG attachments ending
76692e6c, 07e99438 and f7eb2140. No new application run or screenshot capture was
performed in this planning turn.

## Findings

| Priority | Finding and owning source | Consequence and planned correction |
| --- | --- | --- |
| P1 | view.rs:188 owns Note; :705 computes footer labels; :741 focuses Note after a blank click; :793 onward builds status-bar; :2505 and :2535 restore/start Note focus | Deleting a div alone breaks the input/focus contract. Remove the entity and Note-only actions/accessibility labels, migrate focus to the shell/active control and preserve search |
| P1 | view/page_clean.rs:147 clean_landing; :156 Earth; :201 introductory action; :261 scope cards; shell.rs:2468 opens scope, :2473 choosing Project can request a scan | The apparent Start action only opens another choice, while scope selection can itself scan. Show scope/configuration first and make a labelled Scan action the dispatch boundary |
| P1 | view/page_workbench.rs:10 renders tabs; :106 explicitly keeps landing controls mounted for UIA; :118 renders duplicate cards | Two controls perform each navigation and the test retains obsolete product UI. Keep semantic tabs, migrate automation to reacquire current elements after transitions |
| P1 | preferences.rs:169 DesktopFont; :208 persisted font; :278 Font patch; shell.rs:2421-2442 draft setters/dirty; :3218 onward save; view/page_support.rs:52 Settings | Store supports fonts but the session draft/UI/render connection is missing. Extend existing model and save flow; do not add another store |
| P1 | view.rs:658 render uses scale/theme without selected font; input.rs:458 uses window.text_style and :461/:500 style.font() | Root font inheritance can cover input shaping but must be verified for actual text, placeholders, composition and fallback, not only preference JSON |
| P2 | GPUI Cargo.toml:5 version 0.1.0 versus root Cargo.toml:6 version 0.3.0; root :9 canonical repository | About must label the GPUI binary version truthfully, not hard-code the CLI release |
| P2 | shell.rs:1556 software_enter, :1584 queue_software_load, :1334 generation-guarded results | Direct-entry Software must enqueue once on transition, never every render, and preserve stale-result guards |
| P2 | software_review.rs:147 onwards host loaders; workbench.rs Leftovers refusal; page_workbench.rs introductory copy | Capability varies by segment/source. Updates reads do not install, and startup/uninstall claims must match the active host/fixture adapter; remove the irrelevant Leftovers disclaimer from ordinary landing UI while retaining refusal |
| P2 | view.rs:759 onwards page-scroll owns remaining height, mode-panel is content-height; Clean intro and Software cards dominate their short panels | The unused area is structural: the useful workspace is behind navigation, and data panes lack a viewport allocation. Fix information architecture before spacing |

## Font implementation evidence

The pinned vendored Windows text system implements all_font_names at
vendor/gpui_windows/src/direct_write.rs:241 and :1285. It enumerates system and
custom collections. Prefer the existing public GPUI text-system facade if it
provides a safe, read-only route; deduplicate/filter display names and keep
System default available. Inspect its thread rules before dispatching it.

The existing Tauri fonts.rs (desktop/src-tauri/src/fonts.rs:64 onward) provides a
reference for native-only DirectWrite enumeration, localized names, canonical
identity, no downloadable fonts and owned strings crossing workers. Do not
import the Tauri binary crate or copy its command wrapper into GPUI. If the GPUI
facade cannot satisfy the contract, implement a small GPUI-local native adapter
following this established policy; no speculative cross-frontend refactor.

The V2 compatibility contract is .trellis/spec/backend/desktop-preferences.md.
Its Tauri event/commit timing is not the current GPUI draft behavior. Keep GPUI's
explicit Save semantics; font fallback must not silently write a replacement.

## Design language and source hierarchy

Keep style:airy-glass (轻盈毛玻璃), already applied to this GPUI app. Sources:
- ../11-04-gpui-airy-glass is archived under
  .trellis/tasks/archive/2026-11/11-04-gpui-airy-glass.
- That task's research/DESIGN.md is the generated design-atlas entry.
- Its design.md describes native dark/light adaptations.
- desktop-gpui/src/ui.rs owns actual production tokens.
- Root DESIGN.md is historical architecture. Replacing it would destroy unrelated
  context and is outside this task.

The entry explicitly says "There is no marketing hero." Current Clean has an
introductory hero with a decorative globe; current Software is an equal-card
feature overview. The issue is a mismatch between material and page purpose,
not a reason to replace the theme.

## Mechanical audit

Command executed:
node C:/Users/lyh/.skillsmanage/skills/design-atlas/scripts/slop_check.mjs
desktop-gpui/src --scope app --design
.trellis/tasks/archive/2026-11/11-04-gpui-airy-glass/research/DESIGN.md --json

Result: Floor 0 FAIL; House 0 FAIL; Reflex 0; files 0.
This tool does not cover these Rust sources. Result is NOT a passing native UI
audit. No new waivers were written and no design files were replaced.

## Judged audit (self-review, not independent review)

| Gate | Assessment |
| --- | --- |
| 3 / 8 | Fail: Software's three equal icon cards duplicate tabs; Clean starts with introductory content instead of scope/task |
| 9 | Needs improvement: nested large bordered surfaces dominate; use headings, alignment and separators inside one main sheet |
| 46a | No invented numbers in the proposed design. Empty or unscanned data remains unknown, not zero |
| 54 / 58 | Introductory prose competes with controls. Shorten to one useful sentence and put capability explanations at the relevant action. Dense real tables are not subject to a marketing-copy word cap |
| 63 / 66 | Remove the generic Ready dot, long shortcut sentence and nonfunctional Note affordance. Preserve meaningful per-job status |
| 69 | Duplicate segment destinations are a confirmed interaction echo even though this gate's literal metric-count condition is not reached |
| 70 | Note has no meaningful commit/output destination. Remove it; never imply that text was saved |
| 40 / 41 | Not verified: screenshots alone do not establish all composite color pairs. Native contrast checks remain mandatory |
| 44b | Current landing actions fit, but task configuration/results are behind them. New first-screen criteria cover actual task controls |
| 49a / 73 / F26 | Not verified at other sizes/scales; native layout matrix required |
| 84 | Supplied real screenshots are evidence; proposed textual wireframes are labelled design, not rendered UI |
| F4 | One primary action per active scope/review stage; secondary navigation is not a competing primary |
| 29 / 65 / 80 / 83 | Keep established airy-glass field; do not add halos, photography or ornamental assets. Remove Clean Earth only |
| F23 | DOM document metadata is not applicable to GPUI. Native window naming and accessibility are the relevant checks |
| Other checklist items | No new evidence of the corresponding defect in these screenshots; not a claim of runtime verification |

## Sharper problem statement

Blank space alone is not a defect. A scan tool with no data should not fabricate
a dashboard. The defect is that first-screen space fails to answer "what will be
scanned?", "what can I do next?", and "what is the current state?". Supply those
answers directly. Keep purposeful whitespace after genuine empty states.
