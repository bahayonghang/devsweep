# Current state and Mole reference audit

## Reference provenance

- Local reference checkout: `ref/Mole`, commit
  `014a25f88db3fd2e5ef65011c38954b827c9a8b1`, clean at inspection time.
  The directory is ignored by the DevSweep repository (`.gitignore:35`) and
  must remain research-only.
- Requested product surface: <https://mole.fit/zh/>. The page describes one
  entry point for Clean, Software, Optimize, Analyze, and Status, with
  review/selection, software leftovers, bounded maintenance, treemap disk
  navigation, and read-only system/process metrics.
- `ref/Mole/README.md` identifies the checkout as a macOS 12+ CLI and says the
  experimental Windows implementation is a separate upstream branch. It is
  not a Windows/Tauri implementation contract.
- `ref/Mole/TRADEMARK.md` and the repository licence require an independent
  DevSweep identity. No Mole source, assets, screenshots, strings, geometry,
  rule tables, or GPL code may enter production or tests.

## Existing DevSweep evidence

- The five-mode contract and Windows safety boundary already exist in the
  archived 08-29 redesign and five-mode integration tasks. The later 09-01
  and 09-13 tasks already delivered a sidebar shell, visible page headers,
  dark mode canvases, and typed supporting destinations. The new task must
  improve the remaining workbench quality and orchestration instead of
  reopening those archived deliverables.
- `desktop/src/api/bridge.ts:20-156` is the typed five-mode/supporting-domain boundary. It calls
  Tauri commands through `invoke`, decodes closed wire contracts, and uses
  `Channel` for scan/analyze/status streams.
  It is not the only React IPC adapter: presentation settings use
  `desktop/src/i18n/index.ts:165-167`, and window lifecycle/DEV-only fault
  injection use `desktop/src/lifecycle.ts:21-35`. TPR-04 retains these explicit
  typed non-domain boundaries; components still cannot call IPC directly.
- `desktop/src/state/operation-coordinator.ts:27-107` serializes heavy work,
  requests cancellation, waits for join, and rejects stale completion by
  operation identity. It must remain the lifecycle seam.
- `desktop/src-tauri/src/lib.rs:48-90` registers the five-mode commands.
  `desktop/src-tauri/src/analyze.rs:4-141`, `software.rs:8-290`,
  `optimize.rs:8-279`, `status.rs:5-225`, `commands.rs:92-135`, and
  `clean.rs:1-82` call typed `devsweep-core` services from `spawn_blocking`.
  There is no external CLI process or shell command in the Tauri path.
- `desktop/src/styles.css:1-84,99-163,221-275` contains the current dark
  sidebar shell and mode tokens, but also generic empty/table surfaces and
  light fallback button/risk tokens. These are concrete polish targets for a
  Mole-inspired, original workbench pass.
- `.trellis/spec/backend/directory-structure.md:131-150` currently requires
  `desktop/src-tauri` to remain a thin adapter over `devsweep-core`, prohibits
  importing the CLI crate, and centralizes external process spawning in the
  core process runner. Any deliberate change needs a spec update first.
- Archived resource evidence is a baseline only. It was collected on earlier
  commits and must not be presented as proof for this task; the performance
  child must recapture current release binaries, fixtures, host metadata, and
  cancellation/backpressure traces.

## Accept / adapt / reject

| Mole idea | DevSweep disposition |
| --- | --- |
| Five tools in one entry point | Accept as the existing five typed modes. |
| Review first, explicit selection, stable action summary | Adapt to DevSweep's plan, digest, and second-confirmation funnel. |
| Dense grouped rows and progressive detail | Adapt with existing target, evidence, inventory, and audit DTOs. |
| Analyze treemap and breadcrumb drill-down | Adapt as read-only Analyze; no treemap deletion path. |
| Status dashboard with live metrics | Adapt only for supported Windows metrics and explicit availability states. |
| Dark immersive visual hierarchy | Adapt with original mineral/forest tokens and Windows-native chrome. |
| Mole planets, hamster, traffic lights, marketing copy, or pixel geometry | Reject for provenance, accessibility, and product identity. |
| macOS menu-bar, Touch ID, fan control, privileged maintenance, or broad Trash semantics | Reject/defer until a separate Windows contract exists. |

## Resolved architecture decision

The user selected the shared typed service option: Tauri and the CLI reuse the
same in-process `devsweep-core` service path, with no external CLI child
process. This preserves the current thin-adapter specification, avoids a JSON
round-trip and duplicate scan, and keeps cancellation/join under the existing
operation coordinator. External executable discovery, stdout/NDJSON loopback,
process-tree cancellation, and child-process audit are therefore explicitly
out of scope for this task.
