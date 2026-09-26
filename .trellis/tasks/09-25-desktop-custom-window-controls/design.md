# Design: Custom Main Window Controls

## Ownership

| Component | Responsibility |
| --- | --- |
| Tauri main window | Authoritative minimize/maximize state, drag, resize, and close events |
| desktop/src/lifecycle.ts | The existing permitted native adapter; extend with a typed, injectable window-control interface |
| desktop/src/app-shell/ | Titlebar composition and accessible controls; no direct Tauri imports |
| DesktopLifecycleController | One shared operation drain for native close and unmount |
| Main capability file | Exact permissions for the implemented window methods |

## Interaction Flow

1. The titlebar reads native maximize state through the adapter on mount.
2. A resize listener refreshes state. Each state request carries a local generation so a late response cannot restore stale state after a newer request or disposal.
3. Minimize calls the native minimize method. Maximize/restore calls toggleMaximize, then reads native state. Pending actions prevent duplicate button requests.
4. Drag starts only from an explicit blank/title region. Double-click on the same region toggles maximize. Interactive descendants never receive drag semantics.
5. Close calls the native close request method. The existing App listener awaits DesktopLifecycleController.requestClose. The installed SDK destroys the window after that listener resolves. Do not call destroy from the button or add a second drain owner.
6. Main destruction retains the existing tray exit and HUD shutdown behavior.

The titlebar is a compact row above the existing capsule. Keep controls at the upper right. Use original inline SVG symbols and semantic CSS variables. Preserve a centered capsule without absolute-position overlap. Keep the titlebar visible when a mode scrolls. Apply the settings child's typography and theme variables without introducing another theme state.

## Capability Boundary

Keep existing main event and destroy permissions. Add only allow-minimize, allow-toggle-maximize, allow-is-maximized, allow-start-dragging, and allow-close under the core:window namespace if the corresponding method is used. Register and remove resize subscriptions through the current event permission path. Recheck the installed SDK and local generated schema during implementation.

HUD capabilities do not receive these permissions. Keep application invoke gating explicit. Update the capability test's expected set with the concrete methods; do not replace the test with a broad core-default grant.

## Compatibility And Failure Behavior

Preserve the current 1080 by 720 initial size and 900 by 600 native minimum. The 390 and 800 CSS-pixel checks are viewport emulation and do not imply a new native minimum. Command failures show localized, non-destructive feedback. A failed state read retains the last confirmed state and offers retry.

Preserve React StrictMode listener and controller ownership. Browser fixtures use a fake bridge. Fixture success does not establish native drag, snap, work-area, or DPI behavior.

## Risks And Rollback

Custom controls need native Windows verification for edge resize, keyboard snapping, taskbar interactions, monitor work areas, and scaling. Maximize-hover Snap Layouts remain explicitly deferred; no unsupported native hit-test dependency is included.

Ship the decoration toggle, usable controls, bridge, and permissions as one change unit. If native controls fail acceptance, revert that unit and restore native decorations. Do not leave an undecorated window without a working close path.
