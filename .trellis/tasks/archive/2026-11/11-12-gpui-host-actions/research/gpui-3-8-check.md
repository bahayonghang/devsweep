# Independent GPUI and Updates check, steps 3–8

Date: 2026-11-15. Reviewed the approved step-2 decision, task artifacts,
check.jsonl references, implementation receipts, and actual core/CLI/GPUI paths.
This check stops before step 9. Other owners' Clean, MSI, Status, History and
CLI Status changes were preserved. No Computer Use, native window, real host
mutation, commit, push or archive was performed.

## Findings (fixed)

- `desktop-gpui/src/view/page_workbench.rs`: a prepared shared host review could
  execute on the second row activation while the button still said Review.
  Shared Ready state now participates in the existing Confirm label and button
  state; the explicitly confirmed core plan/digest remains unchanged.
- `desktop-gpui/src/shell.rs`: a keep-awake poll sampled before confirmation
  could arrive after an action terminal and overwrite its newer state, including
  replacing unknown after a failed clear. Confirmation now invalidates that
  older poll delivery. A regression samples stopped, restarts the fake session,
  and proves that delayed stopped delivery cannot replace active.
- `crates/devsweep-core/src/software/updates.rs`: post-update observation used
  the first matching ID without rejecting duplicate IDs or a changed row source.
  Those observations now produce Unknown with ambiguous_package or
  unsupported_source, and the same reason is persisted in the support terminal.
- The update-table probe now rejects truncated stderr as well as stdout.
  Stable provider version components explicitly require ASCII digits, refusing
  malformed strings such as v1.+8.0. The Windows family-name comparison also
  removes an unnecessary owned String reported by Clippy.
- Cancellation previously stopped only the next ActionExecutor call. During
  the current update's read-only preflight, cancellation could still be followed
  by a targeted upgrade. The main session explicitly confirmed that the existing
  guarantee means cancellation until actual target commit, and authorized this
  minimal API addition. `execute_software_update_cancellable` is exported beside
  the unchanged legacy function, which delegates with a Noop observer. The GUI
  forwards its job observer through the private ActionExecutor cancellable path.
  All execution-preflight requests and late probes receive that same observer;
  the final commit check refuses cancellation. When Started is already durable,
  refusal appends Refused/canceled_before_start and makes zero target calls.
  The target process and post-dispatch observations deliberately use Noop so a
  later user cancel cannot terminate the installer or erase its real outcome.
  CLI grammar and disk DTOs did not change.
- `desktop-gpui/src/view.rs`: the new shared host-review view was absent from the
  existing token and tab-stop source guards. Both inventories now include it.
- Added meaningful regression coverage for cancellation after dispatch retaining
  the Session lock until an obsolete worker terminal, and for an explicit CLI
  Stop changing core generation and refusing the old GUI review. These use fake
  backends and the real bounded core transport, without native power calls.
- Update regressions cover every initial/late probe cancellation boundary,
  durable refusal after Started, preservation of a real post-commit result,
  unsuccessful or truncated version/help/source evidence, and observation
  timeout/unknown output/truncation/wrong source/duplicate rows. Assertions check
  actual probe/target call counts, forwarded cancellation, stable error codes,
  durable result rows and the 30-second read / five-minute target budgets.

## Findings (not fixed)

- No remaining code finding in the assigned paths after the fixes and checks.
- The old Software maintenance paragraph in quality-guidelines.md still used
  read-only-era wording when reviewed. This was reported to the main session,
  which owns spec synchronization, including the new cancellable API and its
  commit boundary. This agent did not edit shared spec documents.

## Verified contracts

- GUI retains typed core plans and live SHA-256 digests; the action button and
  keyboard confirmation consume them only after preparation. Selection,
  navigation, refresh, root and relevant state changes invalidate UI authority.
  Stale deliveries do not replace the newer selection, and outstanding action
  locks release only on their actual terminal delivery.
- Startup restore creates a fresh inverse plan and preview. A failed/unknown
  result does not preserve a stale enabled badge. Startup and updates use the
  actual Software support journal; GUI result lists are not the durable store.
- Keep awake uses an owning core Session and explicit reviewed execution, never
  convenience open/stop as a GUI shortcut. Restart retains the connection tuple.
  Session Drop explicitly shuts down/joins even when a poll retains an executor
  Arc; failed clear stays unknown and supports a newly reviewed Stop retry.
- Default WinGet resolution uses current-user Store-signed App Installer's
  physical executable. Bound file identity/content, stable 1.x capabilities,
  official source fields/trust, current-user scope and exact selected row are
  checked before targeted execution. Read-only queries remain target-free.
  Confirmation, durable Started and late binding precede the targeted command.
  No agreement acceptance, shell, elevation or source-changing fallback exists.
- Native close paths check the outstanding host job; an idle keep-awake lease
  does not block normal exit cleanup. Clean display fixtures remain unbacked and
  cannot schedule these host actions. UI uninstall/process/leftover execution
  remains refused at this gate. No GUI CLI child was introduced.
- Shared action/control text has EN/zh projections and uses existing tokens and
  keyboard builders; no self active-descendant pattern was added. Audit stores
  contain no endpoint/token/owner label, argv, raw source credentials or localized
  error text. The existing CLI report path preserves one output document.

## Verification

Executed independently after the relevant fixes:

- `cargo test --locked -p devsweep-core --lib software::updates`: 18 passed.
- `cargo test --locked -p devsweep-cli --test c05_software_maintenance`: 2 passed.
- `cargo test --locked --manifest-path desktop-gpui/Cargo.toml`: 233 passed.
- `cargo clippy --locked -p devsweep-core -p devsweep-cli --all-targets -- -D warnings`:
  passed after the respective owners fixed four Status and one MSI-test lint.
- `cargo clippy --locked --manifest-path desktop-gpui/Cargo.toml --all-targets -- -D warnings`:
  passed, including the final stale-poll fix.
- Root and GPUI format checks and `git diff --check`: passed. The last format
  application changed only test assertion layout after its passing test run.
- Type-check: passed through both all-targets Clippy builds and test compilation.

The main session owns canonical `just ci` / `just gpui` and release-build
receipts. This report does not claim those gates ran here or that the full task
is complete. Installed WindowsApps ACL/resolution, real third-party installer
UI/UAC/restart behavior, effective native power hold, and EN/zh native screenshots
remain unverified. Text projections, source guards and command doubles do not
constitute those native acceptance results.
