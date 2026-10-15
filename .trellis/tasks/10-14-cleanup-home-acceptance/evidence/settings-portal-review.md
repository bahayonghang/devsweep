# Final gate: Settings portal timing review

Date: 2026-10-14. Reviewer: `backend_contract_check` (`trellis-check`).

## Finding and evidence

The parent full gate in `dist/cleanup-home-final/gates/desktop-web-check-final.log` ended with 470 passed / 1 failed. The sole failure was `src/components/settings-choices.test.tsx:35`, immediately after `await user.click(trigger)`, at the synchronous accessible `getByRole("listbox")` lookup.

The captured failure DOM contains the expected portal, listbox and options, but its positioner still has `data-closed` and `hidden`. This is the Base UI asynchronous open-state transition, not evidence of a changed product selection contract. The same file's later Select typeahead test already waits for the portal option with `findByRole`. The parent reports that D's earlier full suite on the same product source passed 471 tests; this follow-up therefore records a test timing failure, not a demonstrated product regression.

## Fix

Only the first Select test's portal lookup changed from `screen.getByRole("listbox")` to `await screen.findByRole("listbox")`, with a short explanation. Kept the accessible role query, outside-render-container assertion, selected-option assertion, exact one pointer-choice callback, unchanged committed display and focus-restoration checks. No `hidden:true`, timeout increase, skipped assertion, product change or dependency change was introduced.

## Verification

From `desktop`, using exact Node `C:/Users/lyh/AppData/Local/mise/installs/node/22.23.2/node.exe`:

- `node_modules/vitest/vitest.mjs run src/components/settings-choices.test.tsx`: **PASS**, 15 tests; first Select case also passed.
- `node_modules/eslint/bin/eslint.js src/components/settings-choices.test.tsx`: **PASS**.
- `node_modules/typescript/bin/tsc --noEmit -p tsconfig.json`: **PASS**.

The parent will rerun the complete desktop gate after this narrow repair. This report does not assert full-gate or native/performance acceptance from the focused test result.
