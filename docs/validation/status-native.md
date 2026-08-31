# Status native validation

Native Status presentation evidence is captured against the release
`devsweep-desktop.exe` binary with an isolated `LOCALAPPDATA` profile. Device
scale uses WebView2 `--force-device-scale-factor`; the host OS global scale is
not changed.

## Matrix

| Check | Method | Artifact |
| --- | --- | --- |
| English / Chinese | Settings language then `#/status` | `evidence/native-desktop/screenshots/status-*-en-*.png`, `status-*-zh-*.png` |
| Keyboard / screen reader | Tab to Status controls; AX tree | `evidence/native-desktop/axtree/` |
| Reduced motion / high contrast | CDP `prefers-reduced-motion` and `forced-colors` | screenshots `status-reduced-motion-*`, `status-high-contrast-*` |
| Widths 390 / 800 / 1024 / 1440 | `Emulation.setDeviceMetricsOverride` | `status-*-{width}.png` |
| Device scale 100 / 125 / 150 / 200 | `--force-device-scale-factor` | `status-dpi-{percent}-*.png` |
| Explicit live start | Click Start live | `status-live-started-en.png` |
| Interval change | Change Live interval while live | `status-interval-change-en.png` |
| Process churn / truncation | Snapshot process table | `status-snapshot-en-1440.png` |
| Leave / close / restart | Route away, close, reopen Status | capture log events `leave`, `close`, `restart` |

## Resource gate (release binary)

Raw samples: `evidence/native-desktop/resource/`.

| Phase | Threshold |
| --- | --- |
| Snapshot p95 | <= 2 s |
| Peak private bytes | idle + 64 MiB |
| Peak threads | idle + 4 |
| 60 s live median CPU | <= 5% of one logical core |
| 60 s live p95 CPU | <= 15% of one logical core |
| Post-exit | 25 samples at 200 ms within 5 s; every final-five sample must meet both private and thread caps; missing sample fails |

Do not change the user-global display scale. Do not request UAC. Live starts only from an explicit control.
