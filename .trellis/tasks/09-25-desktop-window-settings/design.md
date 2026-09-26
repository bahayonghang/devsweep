# Design: Desktop Window Controls And Settings

## Architecture And Sources Of Truth

| Concern | Authority | Projection / adapter | Change class |
| --- | --- | --- | --- |
| Window state | Tauri main window | Existing lifecycle adapter and new titlebar | UI implementation |
| Close and active work | Existing operation coordinator and lifecycle controller | Custom close request through the same listener | Preserve lifecycle while changing UI |
| Shared language | Core presentation-v1.json | Existing i18n bridge and TUI adapter | Preserve contract |
| Desktop preferences | New core-owned desktop-preferences-v1.json | Tauri get/update commands and closed frontend decoder | Add bounded preference contract |
| Theme/font/motion | Committed preferences plus OS accessibility signals | Shared main/HUD appearance application | Extend the visual system |
| Status live work | Existing Status service/coordinator | Committed interval and returned-row inputs | Connect supported parameters |
| HUD work | HudState and HudSampler | Committed interval applied at next show | Parameterize existing sampler |
| Cleanup authority | Existing validated plans, live digest, and executor | Existing mode workflows | No change |

The frontend owns controls and presentation. Rust owns preference validation and durable bytes. Native window state owns maximize/restore. No setting may produce a Cleanup Plan or bypass authorization.

## Child Interfaces And Sequence

The window child extends lifecycle.ts and AppShell composition. The titlebar uses semantic variables and accepts the current locale through typed inputs. The settings child supplies the shared appearance contract and the committed preference state. Titlebar code must not acquire its own preference store.

The settings child extends the existing route, bridge, and Rust adapters. It preserves the exact language store. A separate versioned file isolates desktop-only preferences from existing TUI readers. The store has no CLI command surface and no database.

Implement the window child first to avoid overlapping AppShell/spec changes. Then implement settings, including complete titlebar/HUD appearance. The parent reviews combined behavior after each child passes its owned checks. Parent documents do not authorize direct parent implementation.

## Configuration Contract

The authoritative field/default/apply-time matrix is ../09-25-desktop-settings-preferences/design.md. Included controls are theme, local font preset, text scale, motion, Planet frame cap, Status interval, Status returned rows, and HUD interval. Language remains an existing separate preference.

Keep current runtime defaults for the new controls. Theme is the user's confirmed dark default. Application-drawn controls use real window operations. Reduced motion and longer intervals affect actual consumers. Do not expose traversal budgets, worker ceilings, or artificial CPU/memory targets.

## Compatibility And State Transitions

The existing language document remains unchanged. Desktop preferences use a separate core-owned V1 format, closed validation, typed patches, a transaction lock, and atomic replacement. A missing file selects defaults. Corrupt or newer bytes remain intact. A failed save retains the last committed view.

The Tauri preference coordinator emits ordered committed snapshots for main and HUD. Both entries subscribe before loading, apply only current snapshots, and remove listeners on disposal. OS theme changes affect system mode only; OS reduced motion and forced colors remain accessibility overrides.

A theme/font edit does not recreate domain reducers. Opening Settings uses the existing route coordinator. Custom close uses the existing operation drain. Status parameter replacement preserves cancel/join; HUD cadence changes apply on next show without starting a hidden sampler.

## User Flow

Brand menu -> Settings -> General / Appearance / Performance groups -> validated field save -> visible committed value and apply timing.

Appearance changes apply to main/HUD after commit. Status defaults affect the next explicit run; an interval edit inside active Status keeps its existing restart path after a successful save. HUD interval changes apply on the next show. Back returns to the last mode with the existing focus behavior. Group reset affects only the named preference group.

## Spec Update Ownership

The window child changes the native-chrome and lifecycle-adapter clauses. The settings child changes dark-only/font clauses and documents the separate store, HUD read scope, and runtime settings. Preserve the managed Trellis block and all unrelated guidance. Existing component rules for the capsule, original graphics, domain terminology, accessibility, and safety still apply unless the confirmed requirement directly changes a clause.

## Rollout, Rollback, And Evidence

Ship the decoration flag with usable controls and exact permissions in one window change unit. Revert the entire unit if native window behavior fails. Ship preference schema, bridge/decoder, UI, and real consumers as one settings feature; do not leave enabled placeholders.

An older app can still read the unchanged language store and ignore the new desktop file. Rollback retains user preference bytes. No installer, release, new production dependency, OS display change, or real cleanup is needed for feature validation.

Browser fixtures establish frontend behavior. Native Windows evidence establishes window operations, system-theme response, HUD lifecycle, and scale behavior. Record unavailable native rows honestly. New results cannot silently replace the older performance task's thresholds or the native task's binary-specific evidence.
