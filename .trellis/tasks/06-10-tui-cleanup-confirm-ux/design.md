# Improve TUI cleanup confirmation UX - Design

## Design Lens

Use `huashu-design` as a product UI critique lens, adapted to a Rust `ratatui`
production TUI rather than an HTML artifact. The useful principles here are:

- Start from existing context instead of inventing a new visual system.
- Let each UI element earn its place; no decorative icons or filler.
- Make one important detail notably better instead of redesigning the whole
  app. For this task, that detail is the confirmation modal plus footer action
  bar.
- Avoid generic visual noise. The TUI should use existing semantic color
  roles, not add a new palette or marketing-like treatment.

## Why The TUI Still Asks For `confirm`

The current prompt is intentional for command-backed cleanup. The selected
targets include irreversible official cleanup commands such as `cargo clean`
and `npm cache clean --force`. Unlike trash-backed targets, these commands are
not reversible by moving a known directory back from Trash.

The user decision for this task is to lower confirmation friction. The strongest
confirmation should be a fixed, short word: `confirm`. This keeps an explicit
final user action before `Effect::StartClean`, but removes the size-dependent
`CLEAN 45.1 GiB` style phrase that feels like the app is making the user do
clerical work.

The UX defect is therefore twofold: the phrase is too much friction, and the UI
does not explain the remaining safety gate well enough. The improved modal
should make `confirm` feel like a deliberate final approval, not a mysterious
password.

## Boundary

Primary implementation should stay in `src/tui.rs` unless the file becomes hard
to review. The task may add small pure helpers for display formatting and footer
rendering.

Do not change:

- scanner target discovery
- provider cleanup command generation
- `CleanupPlan` serialization
- executor command/trash behavior
- audit JSONL format
- permanent-delete disabled behavior

## Path Display Contract

Add a display-only helper, for example `display_path(path: &Path) -> String`,
and use it anywhere the TUI renders a path-like value. This helper must not
canonicalize, touch the filesystem, mutate `PathBuf`, or alter executor inputs.

Expected display normalization:

- `\\?\D:\Documents\Code\Rust\Exp\devsweep\target`
  -> `D:\Documents\Code\Rust\Exp\devsweep\target`
- `\\?\UNC\server\share\cache`
  -> `\\server\share\cache`
- Normal Windows and Unix-style paths remain unchanged.

Use the helper in:

- `target_title`
- `compact_path`
- `scope_label`
- `path_label`
- `action_summary`
- `evidence_summary`
- `command_preview` for `cwd`
- any filter/display touchpoint changed by this task if needed for consistency

Do not use the helper for:

- `selected_cleanup_plan`
- executor `CommandRequest`
- audit records
- model IDs or serialized fields

## Footer Action Bar

Replace the flat shortcut sentence with a small action-rendering helper. A
simple internal struct is enough:

```rust
struct FooterAction {
    key: &'static str,
    label: &'static str,
    tone: FooterTone,
}
```

Render each action as a compact pair:

```text
 NORMAL  [s] Scan  [Space] Select  [a] All  [d] Dry-run  [c] Clean  [/] Filter  [?] Help  [q] Quit
```

The exact glyphs can stay ASCII. The "pill" is a styled key span, not a real
clickable button. This is the terminal equivalent of a button style.

Mode-specific content:

- Normal: scan, select, all, dry-run, clean, filter, risk, help, quit. Include
  cancel only when a job is active or keep it muted if the existing layout needs
  a stable slot.
- Filter: Enter apply, Esc close, Backspace delete, text input active.
- Confirm: Enter confirm after typing `confirm`, Esc cancel, Backspace edit,
  Ctrl-C quit.
- Cleanup running: x cancel, optionally l logs, Ctrl-C quit.
- Cleanup finished: Enter close, Esc close, l logs.

Tone mapping:

- mode badge: normal/accent, filter/amber, confirm/destructive
- key pills: dark foreground on semantic background for active actions
- labels: muted foreground on footer background
- disabled or not-applicable actions: muted only, or omitted for simplicity

Keep the footer one terminal row. Prefer fewer context-aware actions over a
long universal strip that clips on narrower terminals.

## Confirmation Modal

Keep the centered modal pattern, but make confirmation a specialized render
path rather than a plain generic body. Suggested hierarchy:

1. Title: `Confirm cleanup`
2. Warning header:
   `Irreversible command-backed cleanup` in destructive tone when applicable.
3. Explanation:
   `Type confirm to run these command-backed cleanups. They cannot be reversed by devsweep.`
4. Summary:
   `Targets: 2  Estimated: 45.1 GiB`
5. Command preview section:
   Use neutral labels and caution styling. Keep argv language visible.
6. Required phrase row:
   `Required: confirm` with the word highlighted.
7. Input row:
   `Input: <typed text>` with a visible placeholder when empty.
8. Feedback row:
   Error-styled text only when present.
9. Action hint row:
   `Enter runs after confirm matches. Esc cancels.`

For trash-backed cleanup, use weaker copy:

- `Trash-backed cleanup`
- `Selected targets will be moved to Trash.`
- Required phrase should also be `confirm` if the flow asks for typed input.
  There should be only one typed confirmation word across cleanup modes.

Avoid adding decorative icons. In a TUI, strong copy, spacing, and semantic
color are more reliable than symbol decoration.

## State And Render Flow

State stays the same unless the footer helper needs a small derived
classification. `ConfirmState` already contains the data needed for modal
rendering:

- target count
- estimated bytes
- irreversible flag
- required phrase
- input
- feedback
- message
- command previews

For every cleanup confirmation that requires typed input,
`ConfirmState.required_phrase` should be `confirm`. Estimated bytes remain
visible in the summary, but should not be embedded in the phrase the user must
type.

Render functions remain pure. The event/update layer remains the only place
that can emit `Effect::StartClean`.

## Compatibility And Risk

The riskiest part is path display normalization accidentally changing data used
for execution. Keep the helper return type as `String` and call it only from
render/display functions.

Footer rendering is low-risk but snapshot-like tests should assert mode-specific
text so future changes do not bring back the universal shortcut strip.

Modal copy is product-sensitive: clearer wording should not imply that the app
has already executed anything, that commands are shell strings, or that typing
`confirm` makes cleanup reversible.

## Rollback

Rollback is local to `src/tui.rs` and tests. Removing the display helper,
footer action helper, and specialized confirmation copy should restore the
previous UI without data migration.
