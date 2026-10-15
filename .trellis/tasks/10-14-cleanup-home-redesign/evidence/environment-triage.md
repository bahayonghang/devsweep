# Existing test failure triage

Date: 2026-10-14. These checks reuse A's already compiled test binaries and do not rebuild or edit source. Each exact test failed in the default restricted sandbox and passed using the same binary outside that sandbox after automatic approval. No source workaround was needed. This establishes an environment difference, not a baseline-code comparison.

| Binary and exact test | Restricted sandbox | Normal Windows context |
| --- | --- | --- |
| `dist/debug/deps/devsweep-645e9f937dd4d885.exe application::commands::protect::tests::protect_add_remove_round_trip_and_redacts_audit --exact --nocapture` | FAIL: `protection_store_unavailable`, 0.04 s | PASS: isolated temporary store add/remove, 0.03 s |
| `dist/debug/deps/cli_contract-9ac61bcf6359e6c9.exe json_and_ndjson_are_locale_neutral_and_keep_diagnostics_off_stderr --exact --nocapture` | FAIL: `status snapshot outcome must reflect supported-group availability`, 1.15 s | PASS: read-only Status snapshot/live, 2.85 s |

The protection test only writes its own temporary fixture. The Status test reads system availability and closes its owned live stream. Neither performs user cache cleanup or changes user settings. F must still run the complete canonical gate against the final source; two passing exact tests are not a full CI pass.

Binary SHA-256 identities at this check: CLI library test `78525c7a1b975745ef996ec90abbc737cafd9b10c61f7fcbd896c6895ce9113f`; CLI contract test `6c816cd1ba9587abf09751bed3b8379a14b1a374f8a3b43264161fd03c7f9fed`.
