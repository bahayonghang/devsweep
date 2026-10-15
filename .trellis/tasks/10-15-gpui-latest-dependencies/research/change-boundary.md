# C16 change boundary and technical freeze

Approved parent scope: Windows x64 first, existing cross-platform CLI retained.
Checkpoint: 2026-10-15. Baseline commit: `25437d102ab7355198b3ef2647c2cbced740b3b2`.

## Gap and owner

The core/CLI currently lock older SHA-2, Windows bindings, trash, Clap, JSON and
libc releases. Official crates.io metadata has been fetched and preserved in
`registry/`; `direct-dependency-inventory.json` includes every Rust direct edge.
The smallest change is to update those manifests and the lockfile, adapting only
compiler-identified API changes in their existing owning modules. Existing
authorization, digest bytes, cancellation, output schemas and native-handle
ownership must remain unchanged.

## Frozen choices

- Core: windows 0.62.2, windows-sys 0.61.2, sha2 0.11.1, trash 5.2.9,
  serde_json 1.0.154 and libc 0.2.190. CLI: Clap 4.6.7 and serde_json 1.0.154.
- Existing latest core/CLI direct versions remain current; record their explicit
  minimum versions instead of broad major-only requirements. Cargo.lock records
  exact evaluated versions and checksums. No wildcard or floating Git source.
- Update the core/CLI dependency closure without deliberately updating retiring
  Tauri-only packages. Shared package upgrades also affect Tauri and therefore
  require its Rust and frontend compatibility gates.
- Preserve Rust 1.88 unless actual dependency constraints justify a documented
  increase. Missing local 1.88 is an evidence limitation, not a reason to raise
  the declared MSRV. Test the exact declared compiler when available.
- Scope MSRV/non-Windows CLI CI explicitly to core and CLI so future GPUI members
  cannot accidentally enter the CLI compiler matrix. Preserve all four root gates.
- GPUI registry/upstream candidate audit is recorded separately; final evaluated
  pin, native binary and native matrix are C09's responsibility.

## Expected files and verification

Manifests and Cargo.lock own version selection. Windows/WinRT call sites and SHA-2
consumers change only if required for API compatibility. CI and user build docs
change only for explicit package/compiler policy. C16 research/evidence records
full source/lock/toolchain baseline, latest/selected/license/advisory decisions,
commands and truthful results. Baseline source is recoverable from the clean Git
commit; all build inputs and hashes are copied under `baseline/` before edits.

Run `just ci`, exact-MSRV core/CLI check and tests, legacy frontend checks and
native compilation, dependency audit/license checks, and existing semantic tests.
Record missing Unix execution/native hardware evidence as NOT_RUN. This task adds
no cleanup capability, GUI, installer, provider authority or performance claim.
No commit, install, publish, unrelated refactor or real cleanup is authorized here.

## Reviewed PDH ownership adaptation

The Windows bindings update changes PDH handles from integers to raw pointers.
Retaining `Arc<PdhProbe>` both fails Clippy and removes `Send + Sync` from the
public `StatusSampler`. The main session approved this narrow correction:
`StatusSampler` keeps its private construction, `Default`, and `snapshot` API,
but no longer stores an always-default cache that `snapshot` only cloned.
Each snapshot creates its own local cache; the live worker already does so.
That cache uses `Rc<PdhProbe>`, preserving one local query and exactly-once drop
without claiming native thread safety through unsafe trait implementations.
An external-crate compile-time assertion preserves the public sampler's
`Default + Send + Sync` contract. No sampling cadence or counter math changes.

## Reviewed compatibility-gate repairs

Final compatibility checks exposed an older CLI test-decoder mismatch: optional
GPU/thermal failures were counted as overall partial despite the existing core
and PDH contract excluding them. The main session approved synchronizing that
test and adding deterministic positive/negative envelope cases; production
status behavior and strict data checks remain unchanged.

Repeated full Linux test runs exposed Optimize audit locks retained through a
concurrent fork's open file description. Independent barrier-based research
confirmed the mechanism. The main session approved only the Optimize Unix guard's
explicit LOCK_UN on drop, plus a deterministic shared-description regression.
Acquisition, true-owner conflict refusal and Windows behavior remain unchanged.
This bounded gate repair does not authorize changes to other audit owners.
