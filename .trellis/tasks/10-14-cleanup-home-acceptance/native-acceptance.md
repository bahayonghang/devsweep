# Native scope acceptance evidence

Status: bounded native work complete; final native previews UNVERIFIED and tree
CPU anomaly OPEN. No cleanup execution is authorized or performed.

## Isolation and tools

`scripts/isolated-native.py` creates an allowlisted environment rather than
inheriting the user's environment. HOME, USERPROFILE, application data,
PROGRAMDATA, XDG locations, provider caches and empty npm configuration files
are fixture-owned. PATH contains only qualified Node 22.23.2 and System32.
The installed npm 10.9.8 entrypoint is used directly by preflight. Python, pip,
uv, Cargo, mbx, pnpm, yarn, Go and Bun are absent from child PATH. No provider
installation, cache cleanup or installer replacement occurs.

Preflight: `dist/cleanup-home-final/native-fixtures-01/manifest.json` and its
`preflight-*.{stdout,stderr}.txt` and command records. The npm cache/userconfig/
globalconfig probes resolve inside the fixture. Existing sampler HOME restore
is contained by the launcher's already-isolated outer HOME/USERPROFILE.

`scripts/native-scopes.mjs` uses real UI input and observes actual Tauri IPC.
It does not replace the bridge or issue execute. PID, creation FILETIME and
debug listener ownership are required before connecting to the packaged
`tauri.localhost` page. Every run closes its owned window normally, then verifies
no forced teardown or owned survivors. Sentinel hashes guard fixture content.

Machine JSON CLI commands are language-neutral. Separate human preview commands
exercise `--language en` and `--language zh-CN`. Native locale changes use the
real Settings UI. Current target checkbox aria-labels remain English in both
locales (`TargetTable.tsx`); this existing accessibility-localization gap is
explicitly not evidence of complete Chinese localization.

## Preserved attempts

All paths below are under `dist/cleanup-home-final/`; outputs are never reused.

| Attempt | Observed result |
| --- | --- |
| `native-cli-01` | npm config probe fails; CLI returns partial with exit 5. |
| `native-cli-02` | Same frozen binary/environment outside sandbox also fails. Direct Node controls identify canonical script argv as the trigger; canonical executable with ordinary script path succeeds. SystemDrive, real ProgramFiles and inherited-environment controls (all fixture cache/config overrides retained) do not fix canonical script argv. |
| `native-functional-01` | Window starts; CIM listener query denied. Normal close, no survivors. |
| `native-functional-02`–`07` | Driver diagnostics for missing listener. Port 9517 bind returns Windows 10013; sandbox loopback requests return EACCES and netstat does not expose TCP entries. These are environment/driver failures, not product acceptance failures. All recorded runs close normally with no survivors. |
| `native-functional-08` | Normal host process, baseline port 19314, original allowlist unchanged. Exact listener ownership verified; packaged UI and three actual window sizes captured. Real global scan sends global-only/no-root options. Stops honestly because failed npm probe provides no supported npm target. No preview claimed yet. |
| `native-cli-03` | Fixed frozen CLI produces a valid npm preview. Driver incorrectly expected executed-success count 1; preserved as a driver assertion failure. Successful dry-run is skipped with explicit no-execution reason. |
| `native-cli-04` | PASS: fresh fixture 02, fixed frozen CLI, real global npm and project Next cache scan/plan/preview in both locales. JSON is neutral; localized human output is saved separately. Four valid sha256 digests; same-scope digest matches across locales. Sentinels unchanged. |
| `native-functional-09`–`11` | Final pure-production binary starts and closes normally, but no debug listener is available. Runs explicitly requested ordinary-host execution. Attempt 11 sees 837 TCP rows, saves the owned WebView command line with the correct remote-debugging flag and fixture user-data directory, and observes ECONNREFUSED after 30 seconds. No UI/IPC success is claimed for this binary. |
| `native-functional-12-control` | Previous working binary 43c273... with the same fixture 02 also has no debug listener. IPv4 and IPv6 TCP entries are checked. Correct debug flag reaches the owned WebView child. Normal close, no survivors. This control does not establish a new-binary regression. |

Socket diagnostics: `native-fixtures-01/port-preflight.json` and
`native-functional-06/07/captures/debug-http-preflight.json`.
The Windows npm dispatch defect was fixed by the backend reviewer with
parent authorization. CLI acceptance uses `native-fixed/devsweep-cli.exe`, SHA256
`4636a525c0d89e6cbaa7707c08fa222501e69c004fdf4959cebe40abfd8ba304`, and
`native-fixtures-02/manifest.json`. Successful dry-run assertions require one
selected/attempted target, zero executed successes/failures, one skipped outcome
with the explicit dry-run reason, and a live sha256 digest.
The final desktop binary is `native-fixed/devsweep-desktop-production.exe`,
SHA256 `6904d6bb2d5204f76060226257957e7aa5ab9933c37bf5528efb5b7cd3e88feb`.
Its current native dual-scope/dual-locale successful-preview acceptance is
**UNVERIFIED** because the real debug channel is unavailable. The older binary's
UI captures and the new CLI's successful previews must not be combined into a
claim of current native preview parity. No bridge substitution or looser process
ownership guard was used to hide this limitation.
Old failed artifacts remain unchanged.

## Geometry observations so far

`native-functional-08/captures/en-clean-{900x600,1080x720,1350x900}.{png,json}`
uses actual native resize at existing Windows DPR 1.25, not DPR emulation.
Initial global scan is enabled without roots; project scan is disabled; Clean
has no Earth. Screenshots show a stacked narrow layout and two columns at the
wide size. Settings/Optimize capture at 1080x720 matches the baseline viewport
and DPR. `other-mode-comparison.json` stores baseline/current measured regions;
the Settings page's rectangle, font and colors match exactly.

Interaction sampling is diagnostic only. Formal idle, full dual-locale
successful previews, final visual review, taskbar/tray visual appearance and
installer acceptance are not established by these attempts. Taskbar/tray and
installer remain NOT_RUN unless separately observed. No installer was run.

## Declared formal idle protocol

`isolated-native.py idle` invokes the unchanged `tools/measure-readonly.py`
without a debug port. It writes the protocol before launch: 8 seconds startup,
90 seconds discarded warmup, five repetitions of 30 seconds settle plus 60
seconds retained, for 548 seconds total at 200 ms sampling. Window stays visible;
no interaction or reduced-motion override is made. Parent serializes the run
after compilation, tests and other performance measurements.

`scripts/idle-analysis.py` uses retained windows 128–188, 218–278, 308–368,
398–458 and 488–548 seconds. It applies the existing host-PID gates unchanged:
median of five CPU medians <= 1% (100% = one logical core), maximum private
bytes <= 192 MiB and maximum threads <= 40. Tree resources remain separate;
integrated tree CPU is unavailable if process identities change. Missing
counters do not become zero. Exact identity, scheduled normal close, no forced
termination and no survivors are required.

The new analyzer replayed the archived C2 raw dataset successfully in
`native-idle-analysis-validation/`. This validates the analysis method; it is
not current native performance evidence.

The final measurement completed in `native-idle-final-01/`, started at Unix
nanoseconds `1791999481472339600` with PID 79764 / FILETIME
`134364730814324698`. The protocol and launch record confirm final SHA 6904...
and `debug_port: null`. Host-PID gates pass: median of CPU medians 0.0%, peak
private bytes 5,234,688 (4.99 MiB), maximum 14 threads. Zero CPU is the observed
counter result at its resolution. Normal close returns exit 0; helpers settle
in 241.54 ms; no forced termination, owned survivors or changed sentinels.

| Window | Samples | First–last wall time (ms) | Observed span (ms) | Maximum sample gap (ms) |
| --- | ---: | --- | ---: | ---: |
| 128–188 s | 293 | 128063.88–187955.02 | 59891.14 | 210.26 |
| 218–278 s | 293 | 218104.53–277984.64 | 59880.11 | 210.24 |
| 308–368 s | 292 | 308153.27–367924.94 | 59771.67 | 210.40 |
| 398–458 s | 293 | 398048.13–457961.00 | 59912.87 | 210.67 |
| 488–548 s | 293 | 488095.54–547931.48 | 59835.94 | 210.18 |

The first/last edges are within one observed sampling interval of their planned
boundaries. Complete raw samples, analyzer and protocol are retained; root
reviews coverage independently of numeric resource gates.

**Separate tree anomaly remains open.** Stable owned process sets show tree
integrated CPU 75.63%, 74.00%, 70.08%, 81.32% and 82.49% of one logical core.
Tree peak private is 264,953,856 bytes, maximum 253 threads/8 processes. No
invented tree threshold replaces the host gates; host PASS does not establish
low whole-application idle load. `per-process-diagnostic.json` attributes almost
all CPU to child PID 73556, created about 120.27 seconds after the host. Its
command line was not collected in the uninstrumented run, so its role is not
inferred from its name. The authorized current/earlier-candidate 180-second
diagnostics completed without reproducing the extra child; exact roles and
limits are recorded in [matched native diagnostics](matched-native-diagnostics.md).
The original anomaly remains open and formal raw data is unchanged.
