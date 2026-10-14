# Windows cache production pipeline regression

## Result

The Windows test
`execution::tests::windows_cache_pipeline::real_scan_saved_plan_preview_and_recorded_execution`
passed: one parent test, seven isolated child scenarios, zero failures/ignored,
18.58 seconds. Raw Cargo output is retained in
`dist/native-test-isolation/windows-pipeline-current.txt`.

The new test source SHA-256 was
`49639169d83fdfabab30c1771a43146a56a8aa00b628ecb75a2c9bae3763af53`.
The retained compiled test executable observed after the run is
`dist/debug/deps/devsweep_core-4dcf4b75f763db1a.exe`, SHA-256
`846058105e852803c85099c953e190c239d961575853f16a8195931c7c0c0ffa`.
These paths are in the repository's independent ignored `dist/`, not the
redirected shared `target/` whose earlier executable retention failed.

## Trust boundaries exercised

Each child owns private HOME/USERPROFILE/APPDATA/LOCALAPPDATA/TEMP/PATH. It calls
the real `Sweeper`, serializes/deserializes the scan report and v2 plan, calls
`validate_plan`, obtains a live executor preview and passes its digest into
execution. Only `RecordingTrashRunner` can receive dispatch; no files are moved
or deleted and no installed application is stopped.

The only test substitution is a scoped, cfg(test), thread-local owner-state
observation. Independent helper tests cover actual native process enumeration,
current-user SID and exclusive no-sharing opens. Root resolution, profile and
installation metadata, path validation, tree checks, plan reconstruction,
preview, execution authorization and audit remain production code. There is no
test-only validated-plan constructor or environment-enabled production bypass.

- Positive: 22 exact cache paths across Chrome, Edge, Firefox, VS Code, Cursor
  and current-user stale Temp reach the fake runner. All remain unselected by
  default; recent/secret-containing Temp entries remain unavailable.
- Busy and unknown: application caches remain inspect-only with matching typed
  coverage; the complete mixed report still validates.
- Owner, profile-registration and secret-content drift after preview refuse
  execution before any fake runner action.
- A tampered profile/state root cannot inherit cache authority.
- Cookies, credentials, browser history, offline storage, unregistered profiles,
  extensions, editor settings/state, backups and agent-session sentinels retain
  exact bytes. A recent child prevents eligibility even with an old parent.

This test exposed a real integration defect: distinct inspect-only Windows
cache/Temp observations shared one logical fingerprint, causing whole-plan
validation to reject otherwise valid mixed reports. The functional implementer
changed these inspect footprints to exact paths. The unmodified pipeline test
then passed. Generated-output inspect observations received the same exact-path
identity treatment.

## Separate permissions fixture finding

The existing scanner test used `USERNAME` to identify the denied principal.
On this host its `icacls` call succeeded while directory enumeration remained
allowed. In a new disposable fixture, an explicit numeric Everyone SID with
the `RD` (list-directory) right reliably denied enumeration; removing that deny
restored access. Both setup and restore exited zero. The experiment is retained
at `dist/acl-proof-066bafd43fe34db088a48381c8e47159`.

The [official icacls documentation](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/icacls)
defines numeric SID syntax and the list-directory right. Functional test helpers
now apply this fixture-only rule with restoration on unwind. No user-directory
permissions or machine security settings were changed.

## Limits

This is focused safety/integration evidence, not a complete `just ci`, native
Tauri smoke, cross-platform, cancellation or performance result. General user
protection, reparse and lock guards also retain their separate existing/helper
tests. Final integrated gates remain required.
