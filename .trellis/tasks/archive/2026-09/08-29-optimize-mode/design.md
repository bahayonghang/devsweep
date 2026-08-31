# Design - Optimize Delivery Gate

`optimize-catalog-execution` owns the only operation registry, authorization,
adapters, and audit schema. Presentation consumes that registry through typed
DTOs and cannot manufacture operation ids or argv. The umbrella fixture ledger
covers every allowed operation/action class plus rejected categories, supported
OS capability, `RtlGetVersion` query failure, Settings launch, DNS process
outcomes, stale preview, cancellation, and audit. The catalogue child alone owns
the required `Wdk_System_SystemServices` feature and manifest-independent build
query; presentation may consume only the resulting typed capability/refusal
evidence. Registration is atomic; rollback hides Optimize while preserving
versioned audit records.
