# Pre-edit baseline, 2026-10-14

Source-dependent capture finished before A/E product edits. `manifest.json` is
the index. Raw artifacts and preserved production outputs are under
`dist/cleanup-home-baseline/20261014-preedit/`; these generated files stay ignored.
The initial source was `dev` at `e69a4ee47c4ebae960f41997354c28c0c6030537`.
The exact tracked product files, including the pre-existing Cargo.toml working
bytes, are preserved in `source-snapshot.zip` and `source-identity.json`.
`frozen-build-manifest.json` records zero source drift at the build freeze.
Git initially showed Cargo.toml modified, but its normalized patch was empty;
its exact bytes were neither normalized nor edited by this preflight.

## Builds and method

- Explicit Node 22.23.2 with its bundled npm 10.9.8. `mise exec node@22`
  attempted network resolution and fell back to PATH Node 26.7.0; that fallback
  was not used for these builds.
- `npm run build` passed, including TypeScript; production output is retained
  as `production-web`. Existing large-chunk warning is recorded.
- `node node_modules/vite/bin/vite.js build --config
  vite.clean-benchmark.config.ts --mode home-baseline` passed; retained as
  `clean-benchmark-web`.
- `cargo build --locked --release --offline -p devsweep-desktop --features
  tauri/custom-protocol --target-dir dist` passed. Packaged production executable
  is `native/devsweep-desktop-production.exe`, SHA-256
  `264eed5dd13fa790093c91c36c118e7dd3b261646a4a099187d00fced10ab291`.
  The earlier build without the dependency feature is retained separately and
  was not used for native acceptance.
- No product files were edited. No full CI was run by this baseline step.

## Clean render measurements

Use `clean-render-measure-final/record.json`, collected after release compilation
finished. The existing production harness uses five warmups, thirty measured
samples, nearest-rank p95, complete 100/1,000/10,000-target reports, and real
row/select-all controls. The initial sandboxed Chrome launch exited before CDP;
its failure is retained. `clean-render-measure-retry` succeeded but overlapped
compilation and is diagnostic only. The final run used an isolated owned Chrome
profile, exited normally, and left no forced teardown.

| 10,000-target measurement | Baseline p95 |
| --- | ---: |
| Review row visible | 21.0 ms |
| Review select/deselect all visible | 14.1 ms |
| Preview update visible | 13.6 ms |
| Report JSON parse | 12.6 ms |
| Report decode | 12.9 ms |

All 10,000 targets were retained. Review rendered 100 rows/1,962 DOM elements;
preview rendered 100 rows/1,729 DOM elements. This is a baseline, not a new or
relaxed acceptance threshold. Raw samples and all smaller fixture sizes remain
in the records. `clean-render-visual` contains sixteen English/Chinese cases at
390/800/1024/1440 CSS widths; the driver observed zero horizontal overflow cases.

## Native geometry and observations

Use **`native-captures-final`** only for named dimensions. It contains actual
native production WebView screenshots and geometry at 900x600, 1080x720 and
1350x900 CSS pixels, all verified against `innerWidth`/`innerHeight`, at the
existing Windows DPR 1.25. Settings and Optimize are also captured at 1080x720,
plus the initial page and Clean after route return. Browser/runtime identity,
exact PID/creation identity, resource URLs and accessibility tree are recorded.
These are actual window resizes, not DPR emulation or a Windows scaling change.

The first capture attempt was affected by DPI virtualization; the second was
one pixel wide at 1350 due to fractional-DPI rounding. Both remain diagnostics
in their original directories, with actual dimensions in the JSON. The final
driver opts into DPI-aware sizing and accounts for native nonclient borders.

The empty-root initial state confirms both Projects and Global caches checked,
the Scan action disabled, and the path input lacking an explicit `type`
attribute. Its computed native font is Segoe UI Variable/system/CJK fallback;
the user screenshot's selected font remains unverified. No scan was clicked.
Returning from Settings/Optimize was observed only for this empty-root state;
completed-report retention was not exercised.

The launch requested a hidden window; Tauri ultimately reported the main window
visible. This is recorded, not treated as hidden-render performance evidence.
The profile/settings/WebView data and cwd were isolated. The inherited HOME
behavior of the existing runner was retained, with no global discovery allowed.
The owned process closed normally after 150 seconds, exit 0, lifecycle pass,
no forced termination and no owned survivors. This instrumented UI run is not
an idle CPU acceptance measurement.

## Remaining F work

Full native locale/theme/font/accessibility/scoped-scan/preview matrices,
completed-report route retention, other-mode visual comparisons, installer/tray
evidence and final revised-code performance acceptance remain NOT_RUN here.
They belong to F and the implementing children. The preserved source/builds
support further baseline comparisons without rebuilding from changed source.
