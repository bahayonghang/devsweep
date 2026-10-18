# C10 GPUI shell assets preferences and lifecycle

Status: **approved for implementation**; Windows x64-first scope confirmed. Execution follows dependency acceptance.
Parent: [CLI-first GPUI rewrite](../10-15-gpui-desktop-rewrite/prd.md).
Dependencies: [C01](../10-15-gpui-cli-contracts/prd.md), [C09](../10-15-gpui-native-feasibility/prd.md).

## Goal

Create the native five-mode shell with reusable assets, settings and bounded operation ownership.

## Requirements

C10-R1. Implement the centered five-mode capsule, beaver menu, local history/protection/rules/settings surfaces, EN/zh-CN copy and keyboard focus. The capsule order is Clean, Software, Optimize, Analyze, Status.

C10-R2. Reuse original DevSweep beaver master/ICO/PNG, licensed Earth still/texture and existing semantic icons/colors where suitable; document any asset conversion.

C10-R3. Preserve preference formats and separate scan roots from protections. Read existing stores without writes and migrate only on explicit save.

C10-R4. Run blocking domain work off the render thread with bounded mailboxes, operation IDs, stale-result fences and cancel/join or wait-only close policies.

## Acceptance criteria

- [x] C10-AC1: Route/settings/back preserves reports, selection and appropriate invalidation; no old completion revives canceled/replaced state.
- [x] C10-AC2: Old Tauri and new GPUI can read compatible preferences without corruption; concurrent writers and downgrade cases are tested.
- [ ] C10-AC3: Native titlebar/tray/icon, screen reader, text scaling, high contrast, reduced motion and Chinese text pass actual UI checks. The light palette is unit-tested (`light_theme_canvas_is_not_dark`). The high-contrast palette is unit-tested (`high_contrast_palette_is_distinct`). A later release SHA-256 `f35fabec4498b8c52e123c80939bca6d0ee7107d6ead7c49aec1b515758ec06d` launched with an isolated `theme: light` store, showed a light canvas, and exited 0. Screenshot: `evidence/c10-light-window.png` at text scale 125. The same binary at text scale 100 is `evidence/c10-scale100-window.png`. Both windows are 1318 by 947. The dark text block is 464 pixels wide at 100 percent and 579 pixels wide at 125 percent, a ratio of 1.25. Reduced motion was stored but a still frame does not measure it. The tray guard is unit-tested (`tray_add_is_removed_on_drop`). Release SHA-256 `79F58E4BA32F266151263A99B0A83FA226430E37B58F20EAFD76B5B9EBCE285E`, 13442048 bytes, build exit 0, and the build log has no `incremental`. PID 63532 showed a DevSweep window. The visible taskbar had no DevSweep notification element. After one click on Show Hidden Icons, `TopLevelWindowForOverflowXamlIsland` contained a `NotifyItemIcon` named DevSweep. That element's UI Automation process was explorer PID 15260, the shell host for notification icons. After `WM_CLOSE` the process exited 0 and the overflow no longer contained DevSweep. Sample: `evidence/c10-tray-sample.txt`. UI Automation names inside the DevSweep window were observed for Clean, Software, Optimize, Analyze, Status, DevSweep menu, Modes, and Note (PID 95220, class `Zed::Window`, `WM_CLOSE` exit 0). A Narrator session was not run. With presentation language `zh-CN`, PID 90644 painted and exposed 清理, 软件, 优化, 分析, and 状态. Screenshot: `evidence/c10-zh-window.png`. A later visual sample, SHA-256 `9EEB657B731C147473FA7853304ED59C7B663DE0F7952C025ECBA302409249C6`, 13351936 bytes, PID 76924, painted `在此输入` and did not paint `Type here`. The build log for that binary contains `1 incremental`, so it is not a reproducibility pair. The process left the window after `WM_CLOSE` within 20 seconds; the numeric exit code was `not_exposed`. Screenshot: `evidence/c10-zh-placeholder.png`. PID 35008 exited 0 after `WM_CLOSE`, but the menu click left the Software tab selected and Settings was not shown. Screenshot: `evidence/c10-reduced-motion-miss.png`. PID 4936 had no AutomationId `beaver-menu`, `menu-settings`, `shell-root`, or `mode-tabs`, so nothing was invoked. Log: `evidence/c10-reduced-invoke.txt`. A later binary SHA-256 `793245A0E369011E362BB8641FE1A6D42CB8DC12B3B9A25267B6540A1B4A88B5`, 13355008 bytes, build exit 0, contains the standalone word `incremental` and is not a reproducibility pair. PID 92404 was the foreground DevSweep window. F10 then Enter opened Settings and painted the word `reduced`. Screenshot: `evidence/c10-f10-settings.png`. Exit code after `WM_CLOSE` was 0. That word is the stored preference. No frame interval was measured, and the shell still schedules no decorative frames in any motion mode. `SPI_GETHIGHCONTRAST` succeeded with `dwFlags` `0x0000007E` and `HCF_HIGHCONTRASTON` off. The app was not launched. Log: `evidence/c10-hc-get.txt`. An actual Windows high-contrast window sample remains NOT_RUN.
- [x] C10-AC4: Decorative visuals are optional and idle/minimized/hidden states have no recurring decorative work; icons are offline and correctly attributed.

## Scope and decisions

Parent safety exclusions, platform decision, latest-dependency policy and evidence
rules apply. Detailed design and execution are in [design.md](design.md) and
[implement.md](implement.md). These are proposals, not implementation evidence.
Obtain approval of the parent final summary and resolve applicable technical decisions before starting.
Do not recreate completed upstream work or promote fixture/native gaps to PASS.
