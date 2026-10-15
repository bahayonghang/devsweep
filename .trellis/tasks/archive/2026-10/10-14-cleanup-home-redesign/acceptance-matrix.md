# Requirement acceptance and remaining work

This matrix distinguishes implemented behavior from complete native acceptance.
The parent remains in progress while AC5 is open. Detailed current integration
evidence belongs to child F; archived task results are not current PASS results.

| Criterion | Evidence / boundary | Status |
| --- | --- | --- |
| AC1 independent empty-root global entry | Core/CLI/Tauri scope tests, D session tests and isolated native IPC on the unchanged frontend. No implicit project root is accepted. | PASS at tested scope-contract levels |
| AC2 explicit projects and independent retained results | Scope/session/revision tests cover both reports, failure, cancel, route return, root/protection edits and stale async work. | PASS at tested core/API/UI levels |
| AC3 supported provider authority and unsupported refusal | Existing provider authority suites plus actual Node 22/npm 10.9.8 CLI preview pass. Native coverage is intentionally isolated, not every installed provider. Installed uv 0.13 is outside the existing reviewed range and remains unsupported. | Partial native coverage |
| AC4 project identity and deduplication | Marker/Cargo/shared-target/footprint tests pass; known shared targets become inspect-only. Lexical grouping does not establish unknown physical aliases. mbx workspace attribution is not available. | Partial; mbx gap |
| AC5 mbx workspace cleanup | Pinned 1.24.0 audit cannot bind complete ownership, link/metadata/lease changes and partial outcomes through the public contract. No executable mbx cleanup was added. | OPEN; blocks full parent completion |
| AC6 warning boundaries and preferences | Inclusive 5 GiB, T-1/T/T+1, accumulated outputs, overlap, unknown/lower-bound totals, atomic/corrupt/future preferences and UI state-preservation tests. Dependencies remain separate and warnings never select targets. | PASS at tested core/API/UI levels |
| AC7 cleanup authority retained | Production registry/digest/selection tests, late-preview/confirmation invalidation tests, actual CLI previews and fixture second-confirmation/outcome captures. Fixture execution is simulated. Current native debug listener is unavailable, also on the old control binary. | Implemented; current native preview UNVERIFIED |
| AC8 visual/accessibility matrix | D has 23 fixture captures, saved themes/125% scale, both locales, reduced motion/forced colors; F adds flow captures and native geometry. Native screen-reader, additional installed-font matrix and complete state-by-locale screenshots are not established. | Partial native/manual coverage |
| AC9 identity and consumers | Generated original beaver, compact v2 review, alpha/edge inspection, reproducible 16-asset exports and source-consumer inventory. Actual taskbar/tray/installer appearance remains NOT_RUN. | Assets complete; native consumer acceptance partial |
| AC10 current gates and resources | Final Rust/desktop gates and existing review/preview budget pass; new home workload is measured without an invented latency budget. The final 548 s uninstrumented native idle protocol and host-PID limits pass, but owned-tree CPU is 70.08–82.49% of one logical core and remains unexplained. | Partial; whole-application idle concern OPEN |

## Follow-up boundaries

- mbx needs a public versioned ownership/expected-state contract or equivalent
  complete authority design before a workspace-clean action can be added. A path
  string or successful `mbx clean` process exit is insufficient proof.
- Existing uv support was not broadened to the installed 0.13 merely to make a
  scan appear successful; that version needs its own qualification.
- Target checkbox accessible names are still English in Chinese views, an
  inherited localization gap recorded in F's native evidence.
- No real user cleanup, installer replacement, commit, archive or publication
  has been performed. Missing evidence is not converted into PASS.
