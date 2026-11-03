# Source diagnosis

`ShellView::top_bar` in `desktop-gpui/src/view.rs` gives each mode tab a
`keyboard_focus` modifier (`tab_index(0)` in `ui.rs`) and marks the selected tab
with `aria_active_descendant()`. The pinned GPUI revision's `elements/div.rs`
prepaint first registers the focused node, then registers active-descendant
focus for the same node. `window/a11y.rs::set_active_descendant` rejects this
with the exact panic reported by the user.

These tabs use real per-control focus; selected state already has
`aria_selected`. Remove the active-descendant marker, retaining real focus.
The source sweep finds only this application-owned active-descendant call.

Expected source boundary: `desktop-gpui/src/view.rs`, plus focused regression
coverage if needed. No upstream dependency or Windows backend patch is needed.
Native verification must activate accessibility and exercise mouse and keyboard.

## Original native reproduction

On 2026-11-03, ran `RUST_BACKTRACE=1; just gpui-dev` on Windows using the
existing mbx-managed debug target. Initial UIA tree exposed the five tabs and
Note focus. A screenshot-grounded mouse click on Software reproduced the exact
panic, process exit 1. Backtrace: `A11y::set_active_descendant` at a11y.rs:263,
`Interactivity::prepaint` at div.rs:2351, native Windows paint dispatch.
UIA-index clicking initially reported missing coordinate geometry without
changing the app; the actual reproduction used a fresh screenshot coordinate.
