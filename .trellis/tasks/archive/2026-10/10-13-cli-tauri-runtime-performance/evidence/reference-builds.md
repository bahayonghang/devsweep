# Corrected native reference preparation

The first corrected source capture is a **superseded B-precheck**, not the
reference for runtime acceptance. Independent review found further functional
issues; corrected B2 will be built after those fixes. No backend runtime
production optimization began against this incomplete reference.

Generated records are preserved under
`dist/performance/cleanup-earth/B/`. `backend-source.tar` contains 286 current
working-tree files including new untracked modules, Cargo manifests/lockfile,
resources/i18n, Tauri source/config/icons, and API fixtures. Archive SHA-256 is
`809f0f3d2f6fc0db9551a0728a877c058492d17639d39bb95e0c3d6aa30814e1`.
All source hashes were unchanged during the native build. The separately
frozen frontend's 11 file hashes also matched its manifest.

The preserved CLI is
`396ffe5178ff0609773c131bf8b3c802570e05409ca28edeed7e481a38e157db`.
Tauri's locked release build completed in 3m27s and produced the preserved
12,844,544-byte executable
`3ba59fa9f55d660dbc2766e48e0c5165782403ba49aae3acebe702ade8d6f354`.
Compilation success did **not** establish correct packaging: the coordinator's
owned native launch opened a `file:///D:/.../B/frontend-native/` directory
index instead of the app. This executable is not a valid packaged reference.

Root cause is the Windows absolute `frontendDist` override. Locked
tauri-utils 2.9.3 defines an untagged `FrontendDist` with URL attempted before
directory; tauri-codegen 2.6.3 embeds no assets for URL, while directory paths
are joined to the config parent. The next build uses the relative directory
`../../dist/performance/cleanup-earth/B/frontend-native` from
`desktop/src-tauri`. Before opening the runtime gate, verify the actual
`tauri.localhost` application, offline resources and frozen asset names.

The incorrect packaged run closed normally through its owned window. Its
original fixed-300ms helper-settle check failed; later identity-only checks
confirmed both WebView helpers exited naturally, with no termination. Keep
that failed record and the separate later-settlement observation. Future
measurements use the bounded cooperative settle trace documented in
[harness-deadlines.md](harness-deadlines.md).
