# Research: Native and fixture verification paths

- Query: Find existing desktop launch, CDP, screenshot, accessibility, isolated-profile, and window-interaction harnesses that the main session can use for the approved window/settings implementation.
- Scope: internal; read-only source and file-availability inspection.
- Date: 2026-09-25 (host local date).
- Task: .trellis/tasks/09-25-desktop-window-settings.

## Findings

### Available paths and current prerequisites

Native computer APIs in cua_repl are disabled. Repository scripts can still launch a dedicated desktop process, connect to its loopback WebView2 CDP endpoint, and use scoped Win32 helpers through PowerShell. Browser fixture review is also available. These paths do not require the disabled native CUA surface.

The inspected workspace contains desktop/node_modules, Chrome at C:/Program Files/Google/Chrome/Application/chrome.exe, and Edge at C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe. PATH resolves node.exe to D:/GreenSoftware/node/node.exe, pwsh.exe to C:/Users/lyh/.cache/codex-runtimes/codex-primary-runtime/dependencies/native/powershell/pwsh.exe, and mise.exe/just.exe to Scoop shims. Versions were not queried.

At inspection time, target/release/devsweep-desktop.exe was absent. The generated resource desktop-driver.mjs was also absent from the active native acceptance evidence directory. Main must produce and freeze a current release artifact before native validation. Existing dependencies and archived evidence do not prove the new binary passes.

Required repository toolchain: Node >=22 <23 and npm >=10 <12 (desktop/package.json:6-8). Use mise exec node@22 for the Node scripts. PowerShell 7 is required for the resource driver argument path; the existing acceptance record documents an empty-argument failure under Windows PowerShell 5.1 (.trellis/tasks/09-20-desktop-native-acceptance/evidence/record.md:19-24). Do not reproduce that failure.

The accepted native artifact is produced by just desktop-build, which runs npm run tauri -- build under desktop (justfile:199-200). A plain cargo build --release desktop binary is not the accepted embedded-asset artifact (.trellis/tasks/09-20-desktop-native-acceptance/design.md, section Rows handed off by the performance child). Do not run just tinstall: that target installs the application (justfile:202-204). No build or installer was run during this research.

### Maintained native matrix driver

Source: tools/native-acceptance.mjs.

Exact existing entry points, run from the repository root after a current release build:

    mise exec node@22 -- node tools/native-acceptance.mjs target/release/devsweep-desktop.exe .trellis/tasks/09-25-desktop-window-settings/evidence/native-matrix-NEW matrix

    mise exec node@22 -- node tools/native-acceptance.mjs target/release/devsweep-desktop.exe .trellis/tasks/09-25-desktop-window-settings/evidence/native-cancel-NEW cancel

Replace NEW with a unique run identifier. These are documented commands for the main session, not commands executed here. The cancel run is outside the minimum appearance/window scope; it starts read-only Clean scan, Software inventory, and Optimize catalogue operations and should not run merely to repeat existing acceptance evidence.

Inputs and outputs:

- Arguments: executable path, output directory, optional matrix or cancel (tools/native-acceptance.mjs:19-27, :340-355).
- CDP port: fixed 9333. Run one instance at a time. The target selector finds a tauri.localhost page and explicitly excludes hud.html (:34, :88-107).
- Per-launch profile: OUT/profiles/LABEL. The driver recursively removes that particular profile directory before recreating it, then seeds DevSweep/settings/presentation-v1.json (:213-220). Always choose a fresh task-owned output directory; never use user LOCALAPPDATA or an archived evidence directory.
- Per-process environment: LOCALAPPDATA and WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS containing the CDP port and optional --force-device-scale-factor (:222-232). The driver does not change the Windows display configuration.
- Matrix covers five primary modes, en/zh-CN, current OS scale plus WebView factors 1, 1.25, 1.5, and 2. CSS widths are 390, 800, 1024, 1440 at scale 1 (:29-34, :355-397).
- Outputs: OUT/matrix.json and OUT/screenshots/*.png, or OUT/cancel.json. Profiles remain under OUT/profiles (:149-152, :351, :402-406).
- Existing keyboard checks use CDP Input.dispatchKeyEvent for arrows, Enter, and End (:186-210). Screenshots use Page.captureScreenshot (:149-152).

Important source/document mismatch: the file header and old acceptance record describe WM_CLOSE, but current closeAndCheck invokes taskkill.exe /PID without /F, then polls processes (:237-251). That path does not exercise the custom Close button and must not be credited as its acceptance test. Use a task-scoped driver that activates the actual custom Close control and observes the owned process exiting. An observed failed Close action remains failed even if later cleanup closes the process.

The existing matrix does not visit Settings or the HUD, select themes/fonts, inspect native minimized/maximized state, capture an accessibility tree, or validate titlebar drag/resize. Reuse its launch/CDP/screenshot/process helpers for a task-specific extension. A blanket rerun adds ten launches but does not prove the new feature contracts.

### Minimal isolated launch pattern

The maintained driver owns an entire matrix. A smaller task-owned harness can reuse its launch function at tools/native-acceptance.mjs:213-234 and connection function at :88-147. The profile and port must be explicit per run. The following PowerShell preparation and launch uses the same isolation contract and requires PowerShell 7:

    $runRoot = Join-Path (Get-Location) '.trellis/tasks/09-25-desktop-window-settings/evidence/native-settings-NEW'
    $profile = Join-Path $runRoot 'localappdata'
    $settingsDir = Join-Path $profile 'DevSweep/settings'
    New-Item -ItemType Directory -Path $settingsDir -Force | Out-Null
    '{"schema_version":1,"language":"en"}' | Set-Content -LiteralPath (Join-Path $settingsDir 'presentation-v1.json') -Encoding utf8NoBOM
    $exe = (Resolve-Path -LiteralPath 'target/release/devsweep-desktop.exe').Path
    Get-FileHash -Algorithm SHA256 -LiteralPath $exe | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runRoot 'artifact.json') -Encoding utf8NoBOM
    $appProcess = Start-Process -FilePath $exe -PassThru -Environment @{ LOCALAPPDATA = $profile; WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9333 --remote-allow-origins=*' }
    $appProcess.Id | Set-Content -LiteralPath (Join-Path $runRoot 'app-pid.txt') -Encoding utf8NoBOM

This launch intentionally displays the test window. Main must first verify that the selected port is free and that NEW resolves inside the current task's new evidence directory. Do not overwrite an existing profile to create a fresh-default case. Use a second new profile instead. Do not seed a guessed desktop-preference JSON shape; use the implementation's final DTO and UI for settings persistence.

Record executable SHA-256, main PID, profile path, relevant preferences, host/scale, and target identity before actions. The existing process-tree helper is tools/native-acceptance.mjs:39-85. Its leftover matcher also matches every process with the same executable path, so a pre-existing user app can contaminate results. Do not close such processes; bind test actions to the launched PID and HWND.

### Native window operations and evidence limits

| Operation | Existing supported source | Recommended verification boundary |
| --- | --- | --- |
| Custom Minimize button | CDP button activation can use the maintained evaluate/input helpers. Archived release-direct-cdp.ps1 uses ShowWindow(handle, 6), IsIconic, foreground HWND, and PID checks at :381-395. | Activate the new UI control, then inspect IsIconic on the exact launched HWND. Calling ShowWindow directly proves the helper works, not the button. |
| Restore from taskbar | The same archived script identifies one taskbar UI Automation button, uses SetCursorPos + SendInput, and checks exact HWND, PID, executable path, and hash at :370-440. | Reuse only with refreshed labels and identity checks. Foreground global input needs an unobstructed interactive desktop. The old hardcoded taskbar label is unsuitable for blind replay. |
| Maximize / Restore button | No complete reusable maximize/restore driver was found in the targeted maintained or archived scripts. | CDP can activate the actual new controls and collect UI state. Add a scoped native-state assertion in the task harness, or obtain an operator result. A changed icon alone does not prove native window state. |
| Graceful close / Alt+F4 path | Archived native/capture-native.ps1:118-121 calls CloseMainWindow and waits. Archived release-direct-cdp.ps1:261-280 closes CDP, requests CloseMainWindow, and records exit/residue. | Test the actual custom Close control separately. CloseMainWindow is a useful separate native close-path check and a scoped fallback for the owned test window; taskkill is not custom-close evidence. |
| Native size / minimum size | Archived native/capture-native.ps1:47-74 uses GetWindowRect, GetClientRect, and SetWindowPos to request and verify exact client dimensions. | Reuse the geometry helper with current minimum dimensions. The old script loops 390/800 widths below the current 900 px minimum (:143-153); do not replay those native size expectations. |
| Titlebar drag / border resize / Snap Layouts | No maintained driver in the inspected sources exercises those native pointer paths. | Use an operator result unless the main session adds a controlled pointer harness bound to the exact test HWND. SetWindowPos does not prove drag or edge hit testing. CDP viewport emulation does not prove window resizing. |

Archived paths referenced in the table:

- .trellis/tasks/archive/2026-08/08-29-desktop-shell-navigation-brand/evidence/native/capture-native.ps1.
- .trellis/tasks/archive/2026-08/08-29-desktop-shell-navigation-brand/evidence/focus-failure-repair-20260830/release-direct-cdp.ps1.

Treat both as source patterns. Their full workflows include old labels, old geometry expectations, or evidence-directory assumptions. Do not execute them unchanged. The current native acceptance task already classifies real window minimum/maximum inspection as operator row OP-12 and taskbar restore as OP-5 (.trellis/tasks/09-20-desktop-native-acceptance/evidence/operator-checklist.md:15, :22).

### Screenshots, accessibility, scale, and theme checks

WebView screenshots: use Page.captureScreenshot from the maintained driver. The result captures page content. Record separately any claim about native borders, shadows, screen edges, or Snap UI.

Full-window screenshots: archived native/capture-native.ps1:77-92 uses GetWindowRect, foreground activation, and System.Drawing CopyFromScreen. The helper requires the window to be visible and unobscured. Limit capture to the test window; the script's whole-desktop screenshot helper at :95-105 is unnecessary for this task and can include user content.

Accessibility: .trellis/tasks/archive/2026-08/08-29-status-tui-desktop-native/evidence/independent-native/capture-native-status.mjs:282-297 calls Accessibility.enable and Accessibility.getFullAXTree and saves JSON under axtree/. The launch and generic CDP code is at :33-100. Reuse those helpers with the new task's paths and page targets. The archived file derives its repository root from a fixed ancestor count (:7), which is unsafe after archival; do not run it unchanged.

Media emulation: the same script uses Emulation.setEmulatedMedia for reduced motion and forced colors at :270-280. The settings implementation can be reviewed with browser emulation for light/dark system preference without changing the user's OS theme. Label that row as emulated media. Verify the actual current OS preference separately without changing it. A full accessibility tree proves exposed roles/names/states, not successful spoken output in a screen reader.

Width and scale: retain the existing evidence labels. 390/800 viewport emulation is not a native 900-minimum window. --force-device-scale-factor creates a WebView device-scale row; it does not change Windows display settings. Source: tools/native-acceptance.mjs:222-227, :379-397; .trellis/tasks/09-20-desktop-native-acceptance/design.md, Scale and width evidence. Avoid the old repeated resource failures; record only the new feature-specific rows needed for this implementation.

The maintained target selector excludes the HUD. A new harness must choose the HUD page explicitly from the CDP target list after the HUD is shown. Existing tray/HUD show/hide/menu checks remain operator rows OP-1 to OP-6 (.trellis/tasks/09-20-desktop-native-acceptance/evidence/operator-checklist.md:11-16). The taskbar input pattern does not itself prove a reusable tray-icon activation path.

### Frontend fixture visual review

Exact existing command from the repository root:

    mise exec node@22 -- npm --prefix desktop run dev:fixture -- --host 127.0.0.1 --port 4180

Review URL: http://127.0.0.1:4180/. Settings route: http://127.0.0.1:4180/#/settings. Stop the server by its owned shell session when review ends; do not terminate unrelated Node processes.

Sources: desktop/package.json:10-13; desktop/.env.fixture:1; desktop/vite.config.ts:21-25; desktop/scripts/dev-server-port.mjs:3-5. If port 4180 is occupied, choose a free explicit loopback port and use the reported URL. The server is strict-port; do not silently connect to an unrelated server.

The fixture is enabled only in development by VITE_FIXTURE_BRIDGE=1. Main receives a controlled domain bridge and an in-memory language adapter (desktop/src/main.tsx:7-25). The domain fixture bridge returns archived typed snapshots and controlled results instead of native cleanup actions (desktop/src/api/fixture-bridge.ts:1-33, :133-169). New desktop preferences and window controls need compatible fixture adapters before browser fixture review can exercise the implementation. Check the final code, since another agent is implementing those adapters now.

The current HUD entry loads the production presentation-settings bridge, independently of main fixture selection (desktop/src/hud/main.tsx:12-24). Opening /hud.html in fixture Vite does not by itself supply a deterministic HUD bridge or native sampler. Use a dedicated fixture adapter added by the implementation or validate the real isolated HUD through CDP/operator input.

Browser controls in the current tool catalog can inspect a loopback fixture tab even though native CUA is disabled. Alternatively, reuse the built-in Node WebSocket CDP helpers. No Playwright dependency is declared by desktop/package.json:22-46, and no separate maintained screenshot runner for all fixture routes was found in the targeted scripts.

tools/run-analyze-react-commit.mjs is a narrow Analyze performance driver. Its exact interface is node tools/run-analyze-react-commit.mjs PAGE_URL OUT_JSON OUT_CONSOLE_LOG (:1-12). It launches headless Chrome/Edge with a dedicated profile (:14-23, :49-63), then waits for an Analyze-only benchmark result (:137). It is not a general Settings screenshot driver. Do not run the Analyze benchmark to validate appearance/window controls.

### Scripts excluded from this validation scope

Do not run tools/measure-resources.ps1 wholesale for this task. It creates profiles/artifacts and includes confirmed Optimize execution: dns.flush at :1205-1213 and Windows Settings operations at :1279-1299. The native acceptance record reports those real side effects. Its generated desktop-driver.mjs source at :756-828 can be read for launch/CDP patterns without invoking the resource protocol. Do not create the absent generated driver by running the full resource script.

Do not rerun the recorded idle-thread or Clean-baseline failures as part of appearance verification. Their owning performance/native tasks retain those rows. No installer, actual cleanup target, startup item, Recycle Bin move, or OS display/theme change is needed for this task's new UI verification.

### Minimum evidence plan for the main session

1. Use fixture review for Settings layouts, save/error states, both locales, all themes, typography, reduced motion, forced colors, and 390/800/1024/1440 viewports. Record screenshots and an accessibility tree. Label evidence as fixture/browser.
2. After the correct release build, launch one isolated profile and bind the app PID, HWND, binary hash, and CDP target. Validate preferences through actual UI save/restart, and both main/HUD consumers. Use separate new profiles for missing/corrupt store cases.
3. Activate custom controls through the product UI and inspect native state where a scoped helper exists. Preserve the process when testing minimize/maximize; do not substitute a new process for restore. Test graceful close as the final action and record owned descendants after exit.
4. Obtain operator results for drag, border hit testing, Snap Layouts, tray activation, unobscured native chrome review, and any native state check not covered by the completed task harness. Record actual OS scale as found. Do not change host scale/theme.

Suggested task-owned output layout: evidence/native-NEW/{artifact.json,app-pid.txt,host.json,window-events.jsonl,preferences-before.json,preferences-after.json,screenshots/,axtree/,localappdata/}. These names are recommendations; the implementation/main session owns the actual harness and evidence writes.

### Related specs and external references

- .trellis/spec/desktop-frontend/index.md, Quality Check and Render-budget and native evidence protocol: separates fixture/browser evidence from native Windows evidence and requires artifact/method attribution.
- .trellis/spec/desktop-frontend/component-guidelines.md:115, :185-186: reduced motion, both locales, keyboard/high-contrast review, and no automatic display-scaling changes.
- .trellis/tasks/09-20-desktop-native-acceptance/design.md and evidence/operator-checklist.md: current evidence categories, isolation, and operator scope.
- No external API documentation was needed for this report. Named automation methods come from checked-in harness source; no unverified new Win32 driver is presented as existing functionality.

## Caveats / Not Found

- Only this report was written. No app/browser/server was launched; no build, tests, cleanup, native input, OS setting change, or git operation was performed.
- The release executable was missing at the time of the read. Main may create the executable later; use the new build and hash at validation time.
- No current maintained all-in-one harness proves the new Settings/HUD preferences and custom window controls. Reuse bounded helpers and add task-specific coverage, or request operator evidence for the uncovered native interactions.
- Existing comments claim WM_CLOSE while the maintained driver invokes taskkill. Do not carry that claim into new evidence.
- Existing historical scripts have stale labels, path assumptions, and minimum-width expectations. Read and adapt selected helpers; preserve archived evidence.
