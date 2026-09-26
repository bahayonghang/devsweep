# Custom Main Window Controls

## Goal

Provide application-drawn window controls that match DevSweep and preserve the current Windows operation lifecycle.

## Ownership And Dependencies

- Parent: `.trellis/tasks/09-25-desktop-window-settings`. Parent requirements R1 and R7.
- Own the main titlebar, window adapter, main-window capability changes, and focused window tests.
- No product dependency on the settings child. Integrate semantic colors and typography with that child after both complete.
- This child can be checked with the current dark theme. Parent acceptance also checks light and system themes.

## Requirements

- W1: Replace the main window's native caption controls with one application-drawn minimize, maximize/restore, and close control set. Keep the DevSweep title and existing initial/minimum size.
- W2: Support blank-region dragging, titlebar double-click maximize/restore, edge resizing, taskbar restore, keyboard snapping, and Alt+F4. Keep navigation and buttons outside the drag region.
- W3: Reflect actual native maximize state, including changes initiated outside the application. A window operation failure must remain visible and allow retry.
- W4: Close follows the existing cancel/join or wait-for-completion path. Repeated close requests share the same drain. Minimize does not cancel active work.
- W5: Provide localized accessible names, tooltips, keyboard focus, high-contrast states, and scale-aware hit targets. Controls must remain distinct from the centered capsule.

## Acceptance Criteria

- [ ] W-AC1 (W1): The main window shows exactly one custom control set. All three actions operate on the native window; maximize changes to restore after native state confirms the change.
- [ ] W-AC2 (W2, W3): Drag, double-click, edge resize, taskbar restore, Win+Arrow, and Alt+F4 pass a native Windows check. Buttons and navigation never initiate dragging.
- [ ] W-AC3 (W3): Initial state, external resize, rapid toggle, failed command, and unmount cannot leave a stale maximize icon or active listener.
- [ ] W-AC4 (W4): Active cooperative operations join and wait-only cleanup completes before destruction. Repeated close and React effect replay leave no orphan operation. Minimize leaves operation ownership intact.
- [ ] W-AC5 (W5): Both locales, keyboard-only use, forced colors, and the existing viewport/scale matrix preserve readable names, focus, and reachable controls.
- [ ] W-AC6 (W1, W4): Only the main window receives new window-control permissions. HUD command restrictions and close-to-exit behavior remain intact.

## Out Of Scope

Close-to-tray, autostart, window-layout persistence, custom platform non-client hit testing, native Snap Layouts hover emulation, and a new navigation design. The existing performance and native acceptance tasks remain separate.

## Evidence

Parent research: ../09-25-desktop-window-settings/research/window-chrome.md. The native-chrome clause in the desktop component guide is intentionally superseded by the user's request. The implementation will update the affected clause.
