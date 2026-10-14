# Baseline A preservation and safe harness checkpoint

Date: 2026-10-13. Source: `e75a29d675987429473ed37e80a253c356b3fc4d`.
This checkpoint releases the original-source freeze. It does **not** complete
the runtime child or its full acceptance matrix.

## Preserved inputs

Immutable copies are in ignored generated storage:
`target/performance/cleanup-earth/A/`. The repository's `target` is redirected
to the MBX Cargo cache; `preservation.json` records its resolved local location.
Keep these copies until A/B/C comparisons are complete.

| Artifact | SHA-256 |
| --- | --- |
| Original release CLI `devsweep.exe` | `b7280f91304ec965c025e9d110967896d8b4f1d068603fff792d2ed2eb0c8fd6` |
| Original packaged Tauri `devsweep-desktop.exe` | `28167d0788adc2d9c18e2b256bcb7ac661f66972896acfc8f79a19cf33ff5f47` |
| Product source archive `product-source-e75a29d.tar` | `df09effc267384b66c08e7dafe8676dbae267a57ee4e7534e2ded31942e0fead` |
| Deterministic fixture contents (7,205 files) | `4067adeef4c76e4d6abc7bf8b3c09142b7219f315d8d9c199a9b393836898212` |

The source archive includes Cargo files, crates, desktop sources, resources,
tools and justfile. Original built frontend files are separately preserved in
`A/frontend-dist/`; every file's hash and length is in `A/preservation.json`.
Fixtures in `target/performance/cleanup-earth/fixtures` cover 100/1,000 small
Node targets, a 5,000-file target, a deep Python project, Unicode/spaces and
overlapping roots. Contents and timestamps are deterministic. No fixture is
deleted or moved by this harness.

## Builds and host

- `cargo build --locked --release -p devsweep-cli --bin devsweep` passed.
- `mise exec node@22 -- npm run tauri -- build --no-bundle` passed after scoped
  sandbox escalation. The sandbox repeatedly denied access to the redirected
  Cargo target lock, including with a different target directory. No lock or
  unrelated process was removed.
- Rust/Cargo 1.99.0, MSVC x86_64 host; Node 22.23.2. The global npm shim was
  unexpectedly 12.2.0, outside package engines. Rebuilding with the Node
  installation's bundled npm 10.9.8 produced byte-identical frontend assets;
  use that explicit installation/PATH for subsequent checks.
- Windows 11 build 26300, 24 logical cores, observed Balanced power plan;
  security and power settings unchanged. OS filesystem cache is uncontrolled.
- Native page evidence: `A/native-cdp-runtime-profile/page.json` and `.png`.
  It loaded `http://tauri.localhost/#/clean` with embedded assets, no dev server,
  actual DPR 1.25, 1080x720 CSS viewport, English, WebView/Edge 155. Screenshot
  inspection confirms the original procedural Earth and functioning Clean UI.

## Harness and reproducible commands

`tools/measure-readonly.py` is separate from the historical side-effectful
maintenance script. It exposes only an explicit read-only workload allowlist,
creates fresh profiles/output, samples owned PID/creation identities and their
descendants through Windows APIs every 200ms, and records CPU, private/working
bytes, threads, handles and I/O. Missing/too-short samples are unavailable.
CPU normalization is 100% per logical core. The original harness revision used
for these samples never force-killed; the later [deadline revision](harness-deadlines.md)
adds separately labeled failed-fixture teardown without treating it as lifecycle
success.

```powershell
python tools/measure-readonly.py prepare target/performance/cleanup-earth/fixtures
python tools/measure-readonly.py run --exe target/performance/cleanup-earth/A/devsweep.exe --fixtures target/performance/cleanup-earth/fixtures --out target/performance/cleanup-earth/A/many100-cold --workload many-100
python tools/measure-readonly.py run --exe target/performance/cleanup-earth/A/devsweep-desktop.exe --fixtures target/performance/cleanup-earth/fixtures --out target/performance/cleanup-earth/A/native-idle-valid --workload desktop-idle --duration 35
python -m unittest discover -s tools/tests -p test_measure_readonly.py -v
```

Use new output directories for re-runs; existing ones deliberately fail rather
than overwrite evidence. Add `--debug-port PORT` only for an instrumented native
run, then `node tools/inspect-native-readonly.mjs PORT OUTPUT_DIRECTORY` to save
page metadata and a screenshot. Keep instrumented and uninstrumented metrics
separate. Native launch keeps real `USERPROFILE`/`HOME` for WebView2 runtime
discovery; app settings, browser data and temporary storage remain isolated.
Passive native runs never start a project/global scan.

Four harness tests pass: closed read-only workloads, reproducible fixture and
overwrite refusal, child-local environment, and PID reuse/unrelated-process
exclusion. `git diff --check` passes. Product test suites are not a claim of this
harness-only checkpoint.

## Fresh measurements

Raw summaries, stdout/stderr and resource samples remain in ignored `A/`.
Software observations contain local installed-application inventory and must
not be committed with this aggregate record.

| Workload | Result | Qualification |
| --- | --- | --- |
| Projects-only 100 targets | 48,734.40ms; complete, 100 targets, exit 0, zero owned leftovers | Initial sandbox run with simultaneous native build; preparation evidence, not paired regression acceptance |
| Optimize catalogue, one warmup + five normal-user runs | p95 66.88ms; median 64.48ms; 8 entries each; exit 0 | Passes inherited 1,000ms timing bound; <200ms calls have insufficient resource samples |
| Software inventory, one warmup + five normal-user runs | p95 8,595.93ms; median 5,456.35ms; 1,100 entries each; all 8 sources available; exit 0 | **Fails** inherited 5,000ms timing bound; must profile and fix/retest, no waiver |
| Uninstrumented native preparation | Host median CPU 0%, max private 7,843,840 bytes, max threads 36; tree max private 211,648,512 bytes, 263 threads, 7 processes | One 35s run, analysis window 5–35s; not the inherited five repetitions with 30s settle + 60s retention |
| Native preparation close | One main-window WM_CLOSE; exit 0; zero owned processes remain | Valid passive close evidence only, not active-operation cancel/join acceptance |

Five-run p95 is nearest-rank (the largest sample). Software measured values:
5111.93, 5208.36, 5456.35, 5838.08, 8595.93ms. Optimize measured values:
66.33, 63.90, 64.33, 64.48, 66.88ms. Calls are fresh processes with warm OS caches,
not same-process warm operations. Results include harness spawn/poll overhead.
Paired A/C and B/C comparisons must reuse this method; missing phase telemetry
and broader native/interaction coverage remain work for later steps.

## Invalid setup attempts and teardown

Three earlier native attempts are explicitly excluded from idle/close gates.
The first sandbox/profile attempt did not load WebView2; the initial shutdown
helper also posted WM_CLOSE to private top-level event windows. The corrected
helper matches only the recorded PID/creation identity and main Tauri window
class/title. Two old processes exited after the invalid attempts; one hung
sandbox process was terminated only after checking the exact saved PID and
creation time. This is failed-fixture teardown, **not** successful cancellation
or normal lifecycle evidence. `A/invalid-attempt-cleanup.json` records it. No
user-owned process was stopped, no process-name termination was used, and no
cleanup, maintenance or OS setting change ran.

## Follow-up before runtime completion

2026-10-13 preservation recheck: original A CLI, native executable, source tar,
all nine frontend files and the 7,205-file fixture identity still matched their
recorded hashes. A verified second copy now lives in the nonredirected ignored
`dist/performance/cleanup-earth/{A,fixtures}`; subsequent B/C evidence must also
use this directory family. `A/relocation-verification.json` records verification.
An initial full-directory copy could not copy one INetCache child in a failed
setup WebView profile; executable/source/frontend/fixture reproduction does not
depend on that profile. No shared target files were deleted.

- Preserve corrected reference B at functional handoff before shared runtime
  edits; repaired/new coverage compares B/C, unchanged healthy workloads A/C.
- Diagnose the Software timing failure and the repeated-work 100-target scan.
- Add phase/publication counters, equal-coverage comparison, paired repetitions,
  100/1k/10k frontend interaction, slow sink/backpressure, and full lifecycle
  acceptance. Baseline A is reusable for those comparisons after source edits.
- Complete inherited idle, Analyze and operation-specific join matrices. Do not
  promote this preparation run into full acceptance.
