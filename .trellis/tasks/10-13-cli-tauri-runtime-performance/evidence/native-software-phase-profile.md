# Native C3 Software phase diagnostic

This instrumented, single-window diagnostic identifies work inside the native
Software request. It does **not** replace the failed formal C3 Software p95 of
5211.467 ms against 5000 ms. It has different instrumentation and no simultaneous
baseline control window. No extra formal samples were selected or discarded.

The ignored `C3-native-profile/workspace` was extracted from the immutable
`C3-final/source.tar`. Only three copied Software files received phase observers:
core inventory orchestration, MSIX enumeration/sizing and the Tauri adapter.
No production source, feature, dependency, worker count, metadata operation or
inventory field changed. The generator, exact diff, source manifest and archive
are retained under `dist/performance/cleanup-earth/C3-native-profile/`.

- Instrumented source archive SHA-256: `db5a8e44e47a052e3024c963cb411f25569f2ea4d192dd94fd71ddca94eb23f7`.
- Instrumented native SHA-256: `b4d5f997df799392d252a5f99c7c4b582cdced5b23c3f751c57711a2ff59936c`.
- Build: locked release through Node 22.23.2 Tauri CLI, explicit Cargo
  `--target-dir`, with `CARGO_TARGET_DIR` unset; passed in 3 minutes 9 seconds.
- All 453 copied source hashes matched after build. The separate formal C3
  source and its 11 asset hashes also remained unchanged.

## Method and correlation

One fresh native process used the same launcher restoration of real
HOME/USERPROFILE and isolated app/preferences/WebView data as formal acceptance.
The effective final Popen environment is recorded and has no
`CARGO_TARGET_DIR`. It retained first use and three consecutive repeats, using
the existing actual UI Refresh inventory operation and real Network IPC response
observer. No cleanup, uninstall or maintenance operation ran.

Each adapter record has the actual operation ID. Its `core_result` record maps
that ID to the inventory's existing `observed_at_unix_ms`, which correlates the
core/source records without a new global context or production API. All four
requests have exactly 13 expected phase records; no absent phase was reported as
zero. The diagnostic driver additionally records the CDP request's wall-clock
anchor. Durations below use each observer's monotonic timer.

## Observed durations

All values are milliseconds. These four observations are not a p95 acceptance
series.

| Phase | First use | Repeat 1 | Repeat 2 | Repeat 3 |
| --- | ---: | ---: | ---: | ---: |
| Network request to response | 4427.423 | 4473.587 | 4735.579 | 4873.265 |
| Command entry to return-ready | 4421.603 | 4467.279 | 4729.330 | 4864.992 |
| Blocking-worker queue wait | 0.019 | 0.013 | 0.011 | 0.023 |
| Core total | 4420.408 | 4466.687 | 4728.807 | 4864.411 |
| ARP source | 57.128 | 54.469 | 58.462 | 52.864 |
| MSI source | 104.464 | 102.836 | 106.667 | 107.473 |
| MSIX enumeration, MTA creation/join included | 1033.549 | 994.598 | 948.851 | 940.299 |
| MSIX installed-path sizing | 3384.242 | 3469.438 | 3777.426 | 3921.616 |
| Assembly, fingerprint included | 1.998 | 1.956 | 1.937 | 1.790 |
| Fingerprint alone | 0.918 | 0.968 | 0.942 | 0.875 |

ARP, MSI and MSIX overlap and must not be summed as total wall time. Fingerprint
is nested inside assembly. MSIX enumeration and sizing dominate this observed
critical path. Queueing, assembly and the remaining request interval do not
support prioritizing an IPC rewrite.

Network duration minus command duration is 5.820, 6.309, 6.249 and 8.273 ms.
That remainder includes dispatch before command entry, serialization, delivery,
receiver activity and observer/clock effects. It is **not** a measurement of
pure IPC or of serialization alone.

All four inventories contained 1100 entries and matched every field/order of
the retained formal C3 inventory after excluding only inventory observation
time, size observation times and the fingerprint derived from those times.
Unmodified raw inventories and fingerprints remain preserved. The private
phase logs contain timings and correlation IDs, not installed directory paths.

## Lifecycle and limitations

The first launch request never reached CreateProcess because the automatic
approval service exceeded its deadline. Its tool response explicitly permitted
one retry; the identical retry produced the observations above. This is retained
as an approval-service failure, not an application failure or an unsafe action.

Owned PID 53500 closed normally once after all four operations and returned 0.
Helpers settled naturally in 1288.1 ms; no forced teardown or owned process
remained. `run-01/` preserves the launch, effective environment, all 200 ms
samples, stderr, four raw IPC responses, screenshots, normal-close record and
`phase-summary.json`. The measurement slot was then released.

These instrumented single-window results below 5000 ms do not close the formal
C3 failure. Further optimization must be justified within shared sizing or
MSIX enumeration while preserving current two-worker, no-follow and coverage
contracts; no additional production change is made by this diagnostic.
