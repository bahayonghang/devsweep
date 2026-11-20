# Clean review and material integration

On 2026-11-20 the isolated integration worktree combined local `264e7ac`
with remote `324e72b`. Five GPUI files required manual conflict resolution.
The integrated Clean page, Clean workbench, disk-map projection and UI token
module matched the remote versions under Git comparison. The remote Clean
page already included the local grouped review, tri-state selection, footer,
completion and read-only fixture handling. It additionally retained focus
reveal through both scroll containers and the revised paper/overlay material.

The shell keeps the fixture navigation guard, cancels stale host review on mode
changes and closes Software's related-directory view when leaving that mode.
The view keeps both scroll handles, the Software sort popover, icon preparation,
worker terminal handling and the close guard for in-flight host actions.
Software retains loadable app icons with a themed initial as fallback.
The host review bar uses the fixed overlay material. The three core files
involved in the remote Cargo process fixture lookup matched the remote revision.

At this source checkpoint `git ls-files -u` was empty,
`cargo fmt --manifest-path desktop-gpui/Cargo.toml -- --check` passed and
both staged and unstaged `git diff --check` passed. These checks report
source integration only. Runtime, accessibility, screenshots and the complete
project gates have their own evidence.
