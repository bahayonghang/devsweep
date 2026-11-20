# MSI core/CLI independent review

Date: 2026-11-15. Scope: approved steps 3–4 and shared Software audit changes.
The approved step-2 decisions govern; the original absolute no-UAC proposal does
not. No GPUI removal/process gate was opened. Shared Clean, Updates, Status,
History, localization and public spec work remains with its respective owner.

## Findings (fixed)

- `software/file_authority.rs`: the no-reparse probe preceded retaining each
  Windows name, leaving a probe/open substitution window. Repeat the existing
  no-follow inspection after all names are retained, before canonicalization and
  hashing. An injected late leaf/ancestor reparse regression also checks that an
  ordinary file still binds.
- `software/msi_authority.rs`: SID evidence accepted `+` syntax and numbers beyond
  the Windows SID representation. Require decimal digits, a 48-bit identifier
  authority, 1–15 subauthorities and 32-bit subauthority values. Regression covers
  valid boundary values and rejected malformed/out-of-range inputs. Local SDK
  `winnt.h` confirms the six-byte authority and `SID_MAX_SUB_AUTHORITIES=15`.
- `software/execution/audit.rs`: applying the new MSI leftover gate while reading
  historical `LeftoverMoved` records made an otherwise valid legacy MSI journal
  unreadable. Historical records retain their previous Removed/identity lineage
  validation; new appends independently require the stricter current MSI proof.
  Regression reads a legacy uninstall **and existing leftover record**, while
  refusing a new leftover append from that legacy operation.
- Updated MSI-related comments and the invalid-identity message which still
  described the execution boundary as exclusively MSIX.
- Added a native Windows SummaryInformation fixture regression: create a temporary
  MSI database with package-code and Word Count summary properties, close it,
  bind its physical file and read through the production summary API. It tests
  Unicode/spaced names and the canonical bound path without installing,
  registering or executing the package.

## Reviewed contracts

- Native qualification requires complete current-user/machine context enumeration,
  exactly one current-user unmanaged registration, installed State `5`, package
  code/version, matching cached summary code and Word Count bit 3, typed exact
  GUID registration restrictions and disabled/observably absent elevation policy.
  API/type/access failure is not successfully absent optional evidence.
- The SDK/product-info property is `State`; only unknown-product (`1605`) proves
  absence, while unknown-property (`1608`) remains unavailable. Checked against
  [Microsoft ProductInfoEx](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msigetproductinfoexw).
  [Microsoft enumeration](https://learn.microsoft.com/en-us/windows/win32/api/msi/nf-msi-msienumproductsexw)
  confirms exact SID/context selection. [Word Count documentation](https://learn.microsoft.com/en-us/windows/win32/msi/word-count-summary)
  supplies the package-authoring bit; it does not sandbox custom actions.
- Private live inventory reconstructs the opaque action. New optional serialized
  evidence or fixture eligibility alone cannot authorize production removal.
  Preview tokens include registration/tool proof; immediate registration/tool
  rebinding catches late changes. The absolute system-directory physical tool
  stays held across dispatch. The cached package binding is released immediately
  before dispatch so Installer can maintain it.
- Separate fixed arguments remain `/x`, strict braced GUID, `/qn`, `/norestart`.
  MSI result classification and replay require process `0` plus exact absence
  for Removed; `3010` and `1641` preserve distinct numeric evidence without
  immediate leftovers. Failure or interrupted monitoring never becomes MSI
  success solely because a later query is absent. Recovery does not redispatch.
- New leftover authority uses the same operation/identity, Removed, valid SID,
  exit zero without interruption and accumulated absence. Legacy MSIX semantics
  and absent optional-field wire shapes remain intact. Support Update events are
  separate from uninstall receipts.
- Scoped `rg` of core Software and CLI Software finds vendor uninstall field
  names only inside test bodies. No production registry read or execution of
  those fields exists; the fixture-only generic text recognizer remains under
  `cfg(test)`.

## Findings (not fixed) / evidence limits

- No further concrete code defect was identified in this slice after the fixes.
- The raw native qualification branches are inspected from source; the full
  executor matrix injects `RegistrationProvider`, not raw Installer/registry API
  returns. The new native summary fixture narrows that gap but does not establish
  acceptance against a real installed current-user MSI. A future injectable
  native-evidence boundary would permit dedicated qualification-failure tests;
  that module-boundary change is not silently added by this reviewer.
- No native uninstall, third-party custom-action/UAC/reboot behavior, WOW64-native
  acceptance, screenshots or Computer Use is claimed. No host inventory was used
  to manufacture fixtures. No software registration, update, startup/process
  action, commit, push, archive or task-status change was performed.
- Shared specs already describe the contracts reviewed here; the main session
  owns their final sync and canonical gates. This slice is not the completed
  host-actions task, and steps 9/10 gates remain pending.

## Verification

All Cargo runs used `RUST_TEST_THREADS=4` in the coordinated shared root target
slot. The slot was explicitly released to the main session afterwards.

| Check | Actual result |
| --- | --- |
| `cargo test --locked -p devsweep-core software` | **107 passed, 2 ignored, 536 filtered**, 3 suites; test time 22.75s. Includes native summary, cache maintenance/tool holding, late reparse, SID bounds, MSI result matrix/recovery/audit and full legacy-journal regression. |
| `cargo test --locked -p devsweep-cli --lib software` | **24 passed**, 189 filtered; includes MSI forwarding, no-confirm refusal, bilingual 1641 and closed numeric JSON. |
| `cargo test --locked -p devsweep-cli --test cli_contract` | **17 passed**; includes saved MSI plan/strict GUID/closed optional evidence and mandatory digest/confirm grammar. |
| Targeted rustfmt | **Pass** for review-owned changes. |
| `git diff --check` | **Pass**. |
| Canonical lint / type-check / all tests / GPUI gate | **Pending main-session integration gates**; deliberately not duplicated per its Cargo-slot schedule. |

The 2 ignored tests are the existing opt-in read-only MSIX host benchmark and
metadata-count diagnostic, not newly ignored regressions. This review does not
reinterpret them as passing acceptance.
