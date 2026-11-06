# Technical design

## Boundaries and approach

Use Session's current settings draft plus DesktopFont and DesktopPatch::Font.
Extend draft initialization, getter/setter, dirty comparison, save and restored
root typography. Existing search uses inherited window.text_style: verify this
path rather than duplicating font shaping.

First validate the pinned public GPUI facade to all_font_names and its thread
rules. Cache/filter/deduplicate owned family names. Do not block every render,
scan font folders or enable downloadable fonts. If the facade is insufficient,
use a small native adapter following desktop/src-tauri/src/fonts.rs; do not
import Tauri into the standalone workspace. No new dependency is planned.

Provide a compact searchable picker and multilingual preview row, bounded list
height and a clear System default reset. Maintain selected value through search,
refresh errors and missing-family fallback. Selection alone has no disk writes.
Use current Back/draft semantics; font setting must not introduce auto-save.

About is a section after existing settings groups. Version/license come from
actual build/package metadata; fixed repository URL matches root Cargo.toml.
No automatic networking, updater or made-up git revision.

## Shared design contract

Follow ../11-06-gpui-usability-workbenches/design.md and the archived GPUI
airy-glass adaptation. ui.rs remains the token owner. Preserve high contrast,
localization, text scaling, stable accessibility roles and current domain safety.

## Compatibility and rollback

After footer/focus child. Does not depend on Clean or Software redesign.
Keep this child's UI and Session changes together when reverting. Preserve
persisted user data. Do not delete unrelated pre-existing code or shared controls.
