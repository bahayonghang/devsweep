# Proposed GPUI UX and technical design

## Boundaries

Changes stay in desktop-gpui plus directly affected tests and task/spec docs.
Existing Session/domain services retain execution authority. No new framework,
frontend or production dependency is planned. Reuse ui.rs and current airy-glass
tokens; retain Windows high-contrast and transparency fallback.

## Shared shell

Remove status-bar and Note as a vertical layout unit. Retain top navigation and
brand-menu access to existing support pages. Move only useful Working/error
feedback to the active page, with a compact busy indication in existing header
chrome when background work belongs to another page. No always-on replacement
bottom banner. Keyboard help moves to existing accessible labels/tooltips and
menu copy, without a new help subsystem.

Initial focus belongs to the shell navigation/root; Tab reaches real controls.
On route changes restore a still-mounted control, otherwise the active tab/header.
Blank clicks cannot steal input focus. Focus-lost observers must not repeatedly
refocus a removed entity or recurse. Preserve bubbling from editable controls,
IME and character-edit keys. Retain TextInput/buffer for search and font filtering.

## Settings: fonts and About

Layout: existing header and Save; Appearance rows (language, theme, font, scale,
motion); Project folders; About. No new top-level mode.

Font selector: System default first, searchable installed names in a scrollable
bounded list with selected/check state and visible focus. A preview line contains
Latin, Chinese and digits. Filtering never changes the selected family by itself.
Loading, failure/retry, empty catalogue and missing saved family have clear copy.
Cache per session, load lazily on Settings entry and allow explicit Refresh.
Refresh is coalesced; stale completions cannot overwrite newer selection.

Data flow:
local text-system catalogue -> owned family names -> selector
-> Session draft DesktopFont -> inherited root text style -> live preview
-> explicit Save -> existing DesktopPatch::Font -> locked V2 store
-> next startup -> saved family resolved or non-destructive system fallback.

Extend settings dirty checks and save handling, preserving concurrent unrelated
fields and corrupt/future-file refusal. Back behaves like other existing GPUI
draft settings; it must not silently save. A failed Save retains the draft and
communicates that persistence failed. Do not introduce auto-save for fonts alone.

Prefer public GPUI font enumeration after verifying the pinned API/thread model.
Never run enumeration in every render. If a native worker is needed, only owned
strings cross it. Family names are opaque data, never a path or shell argument.
CJK/emoji fallback remains available. Inputs currently shape using inherited
window text style, so verify shaping rather than inventing separate font logic.

About shows DevSweep, "GPUI build" plus the actual GPUI package version, MIT,
and https://github.com/bahayonghang/devsweep. A fixed Issues link may use the
repository /issues endpoint. Use build/package metadata for version/license;
centralize the project URL consistent with root Cargo.toml. No invented commit,
release date, updater or connectivity probe. Link opening uses GPUI/platform URL
handling; if unavailable, allow copy and report failure. About needs no network
request just to render.

## Clean: proposed first screen

```text
Clean                                  [AI tools]
Choose a scope to inspect; review before cleanup.
[Project folders] [Global caches]
Project folders / configured roots     [Choose folders]
Explicit scope summary                [Scan projects]
------------------------------------------------------
Current scan progress OR current results OR compact empty guidance
When results exist: search/filter | targets | evidence/preview
```

Remove the generic Start gateway and decorative Earth from Clean only. A single
scope selector reveals the associated configuration; use stacked controls on
narrow windows. Project is the initial selected scope, but selecting a scope or
opening a page never scans. Explicit Scan projects validates selected roots and
uses the existing scan request. Without roots show an actionable folder picker.
Global scan retains the existing explicit scope-confirmation step; show the
current-user/global scope explanation before its confirmed dispatch.

Existing clean_choose_scope(Project) can start work; do not reuse it as a passive
tab-change handler. Separate pending scope UI selection from the explicit scan
action while retaining authority in Session. CleanPhase remains the lifecycle
owner. Scope/result changes invalidate old preview and selection authority.
During a job disable conflicting start controls and expose cancellation.

Progress replaces the workspace body rather than pushing another introduction
above it. Review keeps scope context, search/filters and a target list with
selected-item details. Existing preview/digest/second confirmation remains.
Results describe moved-to-trash or observed outcomes accurately, not "freed"
capacity. No rescan on render or tab restoration. AI tools stays a secondary
destination with consistent return focus.

## Software: proposed first screen

```text
Software                                  [Refresh]
[Uninstall] [Updates] [Startup]
[Search current segment...] [Sort] [applicable Filter]
---------------------------------------------------
Application / version / source / qualified size / capability
... remaining viewport used by rows ...
Selection -> contextual review -> explicit supported action
```

Select Uninstall and queue read-only inventory once on first entry. Reentry
retains current segment and loaded view; explicit Refresh reloads. Switching
segments clears pending authority and performs that segment's read-only load.
Opening Settings and returning does not reset to an introduction. Updates may
run the existing bounded winget probe only when explicitly opened/refreshed.

Remove software_landing cards and Back-to-introduction. Keep a single semantic
TabList with stable accessible IDs, selected state and keyboard activation.
Move any capability explanation beside its corresponding action. Do not present
Leftovers as an ordinary unavailable feature; retain its backend refusal.

Same row click convention: select and show context; an explicit review/action
control prepares a preview; a second explicit confirmation dispatches only where
the existing adapter permits it. Updates is not an installer, and host/fixture
startup/uninstall support must be truthful. Unsupported rows expose a reason.
Search placeholder, filters and empty-state copy reflect the active segment;
do not show uninstall-only "Removable" filters for Startup/Updates.

Keep a bounded row window and a selected-item detail area. Use a side detail
pane only when wide enough; stack it below selected context at narrower widths.
Do not render the full system inventory just to occupy space. Empty, loading and
errors use the same workspace region with one relevant recovery action.

## Layout and accessibility contract

One main glass sheet; dense data lives on solid surfaces with separators. Reuse
8/12/16/24 spacing. Compact header replaces large intro cards. Shell provides the
remaining viewport; operational list containers consume it, with one deliberate
scroll owner per list and no hidden clipped confirmation buttons.

1280x800 and 1536x1024: real scope/tab controls and action above the fold; 960x720:
stack controls/details. Check app scale 100/125 and existing 110 path. Keep titlebar
drag/system controls functional. Dynamic native 100/150% DPI evidence is separate
from app text scale; never change user OS settings automatically.

Mouse, keyboard and UIA invoke the same Session action. Update UIA clients to
reacquire current elements after page changes rather than preserving duplicate
cards for obsolete handles. No self active-descendant or stale focus entity.

## Dependencies, compatibility and rollback

Implement footer/focus first, then settings and Clean/Software serially because
view.rs/shell.rs are shared. Final integration uses the selected fonts and no
footer on every mode. No schema bump, no new cleanup authority, no root design
replacement. Keep each child independently revertible; revert UI/state together,
preserve preference bytes and backend protections. A missing native-font API
requires an adapter spike, not a promise that the unverified API works.
