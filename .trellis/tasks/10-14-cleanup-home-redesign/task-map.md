# Task map

Implementation authorized on 2026-10-14. The source-dependent baseline is frozen. A/C/D and E's assets, plus the safe subset of B, are implemented and independently checked. F's final Rust/desktop gates, real CLI previews and existing production review/preview benchmark pass. The new home benchmark is measured with retention/DOM bounds verified, without a new latency budget. Native/manual gaps and mbx qualification remain explicit. Parent owns requirements, cross-child decisions and final acceptance. See execution-status.md and acceptance-matrix.md for current evidence and open gates.

| ID | Child | Deliverable | Depends on |
| --- | --- | --- | --- |
| A | [cleanup-scope-contracts](../10-14-cleanup-scope-contracts/prd.md) | Independent scope contracts, global capability fidelity, project opt-in defaults | None |
| B | [project-artifacts-mbx](../10-14-project-artifacts-mbx/prd.md) | Project ownership/overlap, mbx qualification and supported workspace cleanup | A |
| C | [project-capacity-warnings](../10-14-project-capacity-warnings/prd.md) | Shared pure aggregation, configurable 5 GiB advisory and persistence | A; B's ownership contract before mbx integration |
| D | [desktop-cleanup-workbench](../10-14-desktop-cleanup-workbench/prd.md) | Design system, independent workflows, retained state, warning and results UI | A; B/C DTOs before integration; E's reviewed mark before final visuals |
| E | [devsweep-animal-identity](../10-14-devsweep-animal-identity/prd.md) | Original mascot, compact/native assets, provenance | Parent design; independent of A/B/C |
| F | [cleanup-home-acceptance](../10-14-cleanup-home-acceptance/prd.md) | Current parity, safety, native, visual and performance evidence | A/B/C/D/E |

Order: after approval, the parent first captures the read-only baseline before any product edits, handing the baseline manifest to F. A and E can then proceed independently. B follows A. C can implement non-mbx fixture logic once A is fixed, but final aggregation waits for B. D can prepare tokens/components against the reviewed contract, then integrates B/C/E. F is the final gate and consumes the earlier baseline. Tree order does not schedule dependencies.

Ownership: A core scope/selection/provider contracts and necessary CLI/TUI parity; B project/mbx discovery + authority; C advisory model/store/DTO; D Clean state/components/styles/i18n/spec amendments; E brand master/derived/native assets; F evidence/drivers/tests. Coordinate shared model/API edits explicitly before parallel implementation.

