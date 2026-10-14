# Packaged reference precheck failure

The first backend reference attempt is explicitly superseded and is not the
corrected runtime baseline. Its CLI and backend source archive remain useful
precheck artifacts; independent functional review found additional repairs.

The native EXE built successfully, but the attempted absolute Windows
`frontendDist` was parsed as a URL. The actual owned WebView target was
`file:///D:/Documents/Code/Rust/Exp/devsweep/dist/performance/cleanup-earth/B/frontend-native/`,
with a directory-index title. It did not load the packaged application. Build
success and a frozen frontend file manifest were insufficient proof.

- EXE SHA-256: `3ba59fa9f55d660dbc2766e48e0c5165782403ba49aae3acebe702ade8d6f354`.
- Owned native PID: 138680, FILETIME creation identity 134363751977612442.
- Isolated profile/output: `dist/native-clean-precheck-01/`.
- No scan or cleanup ran. The smoke rejected the target before UI interaction.
- Normal verified-window WM_CLOSE returned one window and native exit code 0;
  no forced termination occurred. The old harness observed two WebView children
  at only 300 ms after exit, so its lifecycle result remains failed. A later
  identity-checked read found both had exited naturally; that later read does
  not retrospectively supply a precise join time or turn the first record into
  a passing lifecycle measurement.

The runtime implementer is correcting the evidence-only build override to a
path relative to the Tauri project directory, and adding bounded cooperative
post-exit process-tree sampling. The refreshed B2 must load `tauri.localhost`
with the preserved frontend asset identities before backend optimization starts.
The standard production Tauri configuration was not changed by this mistake.

The new `tools/native-clean-smoke.mjs` asserts debugger listener ownership using
the root PID plus exact FILETIME and owned-descendant tree, then checks the
packaged origin. Its eventual successful run must prove offline Earth and
local still fallback, empty-scope validation, duplicate-root deduplication,
keyboard Scan, two actual fixture drives, and full CLI/Tauri report equality.
This file records the failed precheck, not those pending passes.
