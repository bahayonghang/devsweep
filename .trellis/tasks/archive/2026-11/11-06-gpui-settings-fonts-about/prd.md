# Installed fonts and About in GPUI Settings

## Goal and parent mapping

Deliver R2, R3 and R6 from ../11-06-gpui-usability-workbenches/prd.md.
This child is planning only; implementation awaits approval of the parent summary.

## Confirmed evidence

preferences.rs:169-208, :278; shell.rs:2421-2442 and :3218 onward; page_support.rs:52; input.rs:458-508; vendor/gpui_windows/src/direct_write.rs:241/:1285; GPUI Cargo.toml:5 and root Cargo.toml:6/:9.
Full causal analysis: ../11-06-gpui-usability-workbenches/research/audit.md.

## Requirements

Add a searchable local-font selector, real session preview, explicit saved
preference and missing-family fallback. Add About inside Settings with accurate
GPUI build version, MIT license and canonical repository. Keep the V2 schema
and other Settings controls.

## Acceptance criteria

- Real installed names plus System default; bounded scrolling/filtering,
  selected state and keyboard/UIA selection; local-only enumeration.
- Enumeration runs once per Settings session/cache plus explicit refresh;
  loading/error/empty states and retry are honest, no per-render enumeration.
- Chosen font changes root text, rows, search text/placeholder and composition;
  Latin/CJK/emoji fallback remains legible at 100/110/125 app scale.
- Selection previews; explicit Save persists and restart restores. Failed Save
  retains draft and error. Missing saved family falls back without erasing it.
- Invalid/future files remain byte-for-byte unchanged; concurrent unrelated
  fields survive; catalogue refresh never saves a font implicitly.
- About labels the actual GPUI package version, not the CLI release. Fixed
  repository/Issues links work with keyboard and mouse or provide copy/error.

## Dependencies

After footer/focus child. Does not depend on Clean or Software redesign.

## Out of scope

No new cleanup authority, new providers, permanent delete, live cleanup tests,
Tauri redesign or unrelated refactoring. Parent exclusions and safety acceptance
apply. No code change is part of this planning delivery.
