# Clean implementation evidence

## Changes

- `clean_workbench.rs`: selected scope defaults to Project. Passive scope edits
  do not scan; changing scope resets old review authority. Removed the hidden
  ScopeChoice wizard state. A running scan/execution rejects scope replacement.
- `shell.rs`: split passive `clean_choose_scope` from explicit
  `clean_start_selected_scope`. Project dispatch uses explicit stored roots;
  Global only arms its existing confirmation. Picking a valid project folder
  persists/session-configures it and resets old results, without dispatch.
  Keyboard activation now matches the visible scan action instead of stepping
  through a hidden introductory screen.
- `view.rs`: folder-dialog completion no longer calls the scan follow-up path.
  Passes the current Window into page/Clean composition for row viewport sizing.
- `view/page_clean.rs`: compact shared header, one content-width scope TabList,
  selected roots/configuration and explicit Scan action. Removed Clean's Earth
  and introductory gateway. Completed Project states expose Change folder.
  Explicit scope/scan/folder/Global-confirm UIA Click adapters are present.
  Results show actionable rows before secondary statistics/map. The bounded row
  list has a viewport-relative maximum height and its own vertical scroll owner;
  short/empty lists are not artificially inflated.
- `copy.rs`: removed orphaned introductory scan copy and corrected scope label
  so it no longer implies that selecting a scope starts scanning.

## Regression coverage

Two new Session tests cover passive scope/folder edits, explicit Project scan,
no implicit Global confirmation, invalidation after scope switch, busy refusal,
same-scope no-op, stale-preview rejection and intact isolated fixture files.
The existing CleanWorkbench scope test now checks the direct scope selector.

## Commands and outcomes

- `just gpui-check`: final pass, including the Change folder UIA button.
- `just gpui-test`: 172 tests passed after the scope/root-picker/copy changes.
  The final Change folder header button was added afterwards and passed
  all-target clippy; final integration must rerun the test suite.
- `git diff --check`: passed.
- The initial sandbox check could not access the installed mbx lock; approved
  elevated checks succeeded. No repository build configuration was changed.
- No release build was run for this child, per coordinator instruction; the
  final build and native Windows UIA/visual checks belong to integration.

## Remaining evidence

Native screenshots must verify first-screen rows/actions at all planned sizes
and text scales, scope tab focus/selection, Choose/Change folder no-scan behavior,
Global cancel/confirmation, and input/IME navigation. Source checks do not prove
those runtime properties. No host cleanup, uninstall or Global scan was run.

Source ownership was released to the coordinator after final clippy success.
