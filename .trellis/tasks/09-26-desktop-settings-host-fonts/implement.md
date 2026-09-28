# Implementation Plan: Host Fonts

Prerequisites: controls child complete, Q1 resolved, parent plan approved, child activated.

- [ ] Freeze V1-to-V2 mapping and first-write transaction tests before changing consumers.
- [ ] Implement strict V2 types, validation, V1 read conversion, fixed V2 path, shared-lock first creation, and atomic replacement. Keep V1 and language bytes untouched.
- [ ] Add the approved Windows-only DirectWrite dependency and native catalogue command. Register main-only permission and test HUD refusal.
- [ ] Add response/error and V2 fixtures; update command/patch wire parity; regenerate TypeScript; extend closed decoders and fixture bridges.
- [ ] Wire cached asynchronous font discovery, explicit refresh, localized alias search, and the existing Combobox interface.
- [ ] Implement the tested CSS font-string serializer and main/HUD application; preserve saved unavailable names and fallback notices.
- [ ] Update relevant preference/state/type specs and record the downgrade boundary.
- [ ] Run core store, native command, decoder, and component tests including the full failure matrix.
- [ ] Record native Windows catalogue and real main/HUD rendering using isolated preference storage.

Focused commands: cargo test --locked -p devsweep-core desktop_preferences; just desktop-test; from desktop npm run types:generate and npm test -- src/preferences/preferences.test.tsx plus added font/contract tests. Run just ci after the shared Rust changes. Parent final integration may reuse current results when no later code changes invalidate them.

Do not install/remove fonts, change OS display settings, write user preferences, or report a native font test from a fixture. Record F-AC1 through F-AC7 separately from the existing native-acceptance task.
