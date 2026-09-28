# IME Independent Repair Review

Review date: 2026-09-28 UTC.
Reviewer: dispatched trellis-check, palettes_review_final.
Scope: Settings font Combobox IME keyboard boundary and affected selection behavior.
Path abbreviations: controls = .trellis/tasks/09-26-desktop-settings-controls;
parent = .trellis/tasks/09-26-desktop-settings-experience. Source paths start
at the repository root.

## Decision

**Source review, focused checks and native renderer composition: PASS.**
The repair resolves the confirmed composition-key selection defect in the
reviewed code. No additional code defect was found. Complete C-AC2/C-AC5 or
parent acceptance is not established by the focused checks.

The reviewer read the controls PRD, design, implementation plan and check
manifest, and reused the previously loaded parent approval, design and specs.
The reviewer inspected the installed Base UI implementation and the raw native
failure evidence. Production code and tests remained read-only. Only authorized
review reports were written.

## Findings (fixed)

### IME-1: composing keys could persist a highlighted font

- File: desktop/src/components/SettingsCombobox.tsx:47-51.
- Requirements: C3/C4, C-AC2 and C-AC5. Composition, query and highlight must
  preserve the committed value. Explicit choices must still save once.
- Current regression evidence: controls/evidence/native-ime-before-fix.json
  contains the enhanced old-binary reproduction from 03:19:46.962 through
  03:19:53.519 UTC. It records composition start/update, composing input,
  ArrowDown and Enter with native isComposing=true, then composition end.
  Its before/during preference objects directly show Segoe UI changing to
  System. The preference-file hash changed from
  71e8ce10892c421bb1639f477948b3b677af9f0cf0ee0fa5daa63aedb0bf4470
  to 15057b1ec98371a20bbb24b4718a3c98f52fbccb25a5665b101a3d197689990a.
  Its final error is Composing Enter persisted a font choice.
- Earlier probe: at 03:10:51.960-03:10:58.309 UTC, the initially inspected
  artifact recorded a hash change from 854b6fd10b9003c21e28f74ef35dc0401ae339395ecd816b85cbf528403e6537
  to cd688a44654bbdd0307cfd556ad8536b5d2187a6f34dc86741b2767f5175578d,
  the same composing-key trace, and a later popup-close timeout. That version
  did not contain before/during preference objects. The parent replaced the
  file with the enhanced reproduction before the release rebuild. The current
  file must not be cited as the earlier timeout record. No retained copy of
  that first JSON was found; this paragraph records the previously inspected
  output only.
- Cause: Base UI 1.8.0 ComboboxInput.mjs:342-359 checks event.which===229,
  then can call clickHighlightedItem for Enter. The ordinary composing key
  codes reach the item-press path accepted by the wrapper at lines 41-42.
- Fix: the input handler checks nativeEvent.isComposing or nativeEvent.keyCode
  229 and calls preventBaseUIHandler. The implementer made the serialized
  repair; the reviewer independently checked the resulting source.

## Event and state audit

| Boundary | Independent finding |
| --- | --- |
| Handler order | Base UI mergeProps.mjs:167-205 invokes the supplied handler first, then skips prior Base UI handlers when baseUIHandlerPrevented is set. ComboboxInput uses validation/element props last in the merge. The wrapper therefore guards navigation and Enter before selection. |
| Native text commit | preventBaseUIHandler sets an internal flag only. The wrapper does not call DOM preventDefault or stopPropagation. Both key-event tests assert defaultPrevented=false. |
| Composition lifecycle | Base UI ComboboxInput.mjs:204-215 retains its own composition state and calls setInputValue at composition end. Lines 236-254 defer controlled filtering during composition. The repair does not replace these handlers or change query/filter timing. |
| Already highlighted candidate | The native-isComposing and legacy-229 cases at settings-choices.test.tsx:184-205 assert no save and an open, focused input, then verify ordinary Enter still saves the expected option exactly once. |
| ArrowDown and Enter | The full sequence at settings-choices.test.tsx:154-181 uses ordinary key codes 40/13 with isComposing=true and a committed value different from the first option. Both events preserve the value and browser default. |
| Dismissal | After composition end, the test verifies the Chinese alias is filtered, Escape closes the list, focus returns to the input, the committed label is restored, and no save occurs. Existing outside-click and Tab tests also pass. |
| Pointer choice | The handler affects keydown only. The explicit pointer test at settings-choices.test.tsx:207-217 still sends one choice during composition. Existing ordinary pointer and keyboard cases pass. |
| Save ownership | onValueChange continues to accept only changed, non-null item-press values. No callback, query, persistence, preference type, dependency or catalogue ownership changed. |
| Specs | Existing component/type-safety rules already require query/highlight to remain transient and explicit selection to send one typed patch. The repair restores that contract without changing a public interface. No spec update is required for the limited fix. |

The old IME test covered key code 229 without a different highlighted candidate.
The repair report records two pre-fix failing cases: the composition sequence
saved System and composing Enter on a highlighted candidate saved YaHei.
The reviewer did not revert production to repeat those red tests.

## Verification

Independent commands ran from desktop/ on 2026-09-28 UTC:

| Check | Result |
| --- | --- |
| mise exec node@22 -- npx vitest run src/components/settings-choices.test.tsx src/preferences/fonts.test.tsx src/preferences/preferences.test.tsx --maxWorkers=1 | PASS, exit 0; 3 files and 41 tests: 15 controls, 11 fonts, 15 preferences. Started at 22:20:42 America/Chicago; duration 12.53 seconds. |
| mise exec node@22 -- npx eslint src/components/SettingsCombobox.tsx src/components/settings-choices.test.tsx | PASS, exit 0. |
| mise exec node@22 -- npm run typecheck | PASS, exit 0. |
| git diff --check on the two source files | PASS, exit 0. |
| Parent final desktop-web-check, independently inspected machine result and log | PASS, exit 0; 51 Vitest files / 387 tests and 5 Node tests. Types generation, full ESLint, TypeScript and web build pass. Ran 03:19:53.663-03:20:25.329 UTC. |

All three handoff hashes matched:

| File | SHA-256 |
| --- | --- |
| desktop/src/components/SettingsCombobox.tsx | 060c994390351faec055472cea68dde7a2f33a975c3edb1f047aa23fd6260a72 |
| desktop/src/components/settings-choices.test.tsx | 436f026b90dfb5c2fdf32ff367cec3305d69de13c8269b86fb9af267b720f746 |
| controls/evidence/ime-repair-report.md | e1df505cc45b7aeee9cf084b69868a8d5c8052bc9404c35900244fce8c5a92fb |

## Rebuilt native evidence

The reviewer independently inspected
parent/evidence/native-final/native-composition.json and the current
parent/evidence/verify/native-composition.mjs helper. The final run was
03:25:53.893-03:25:54.144 UTC and reports PASS. The session and actual executable
hash both match 03c5b34fa650696f026b1ec982fc70ab8d93e2a8f9665b5b4df4b36f2d2b567d.
The two repaired source hashes remained unchanged.

- ArrowDown and Enter were recorded with native isComposing=true.
- Before, during and after-dismissal preference hashes all equal
  cdcf19e2861e9284ce755ae40aeab288f45f2f27255458f6dfd858367346402e.
  The recorded font remains installed Segoe UI.
- After checking the composing Enter, the helper sends CDP Input.insertText
  with 宋. The trace then records compositionend. The helper requires the input
  value to remain 宋 and a nonempty list narrower than the 249 original options.
  The final record contains 12 matching fonts. English display labels are
  valid because the query also searches localized aliases.
- The helper clicks the input, dismisses the popup, and checks the persisted
  hash again. The renderer text commit and dismissal produce no font save.

Two earlier attempts on this same final binary are retained separately:

| Record in parent/evidence/native-final/ | Actual result and interpretation |
| --- | --- |
| native-composition-no-explicit-commit.json, 03:24:11.976-03:24:18.487 UTC | The helper waited for compositionend without an explicit CDP text commit. All three hashes remained equal and true composing Enter was recorded. The timeout does not establish another unintended save. |
| native-composition-english-label-assertion.json, 03:25:04.128-03:25:10.569 UTC | The helper required a Chinese character in an English display label. Compositionend was recorded and all hashes remained equal. The corrected assertion checks the Chinese query and narrowed nonempty results. |

The assertion corrections retain the persistence and real composing-event
checks. No product code changed between these attempts. The reviewer does not
claim that a CDP Enter alone drives the Windows IME candidate window.
The parent copied the raw records from target/settings-native-final-20260928
into the task evidence directory. The archived final record was also inspected.

## Findings (not fixed) and acceptance limits

No unresolved code defect was found in the reviewed repair.

The post-rebuild native renderer composition retest and final full web gate
pass. The parent executed the native probes; the reviewer independently
audited their recorded data and helper semantics.

The probe explicitly excludes Windows IME candidate-window interaction. CDP
renderer composition evidence must not be reported as physical candidate-window
acceptance. The archived final navigation and matrix records also pass: two
locales, 44 pages, 156 popups and 28 media cases. The reviewer found no recorded
page overflow/text failures or popup boundary/portal/font failures. Viewport
and media settings were emulated through CDP. Complete human/OS acceptance
remains an integration decision. Core CI also remains blocked under
the parent investigation; its cause is not established by this UI repair.

Do not change task status, archive, or claim full parent completion from this
report. Final integration results belong in parent/evidence/integration-check-report.md.
