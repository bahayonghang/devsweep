# Final source and task integrity

Checked on 2026-10-14 after the final gates, builds and component measurements.
Only task evidence changed after the product-source freeze.

- All **603** frozen product source entries match their current SHA256 hashes.
  The current tracked/nonignored-untracked product file set has no missing or
  extra files compared with the frozen manifest. Trellis/agent scaffolding is
  excluded by the documented snapshot contract.
- Source manifest SHA256:
  `1620e0e012345be139374f75ab41859c0fb377b4d43d12c64a5b8f5aa1302ba8`.
- Source ZIP SHA256, independently rechecked:
  `20ba3ec1dd746a37fb37fdaa08ca8a21e50e02d5224c36f20e0109e97d21e510`.
- Existing user-edited `desktop/src-tauri/Cargo.toml` remains SHA256
  `b7d6f6a30067850f5c4b1f3afde6450667262eb57309d230ca05fe7a0dd84915`.
- Temporary `.cargo/config.toml` is absent. The existing external target link
  was not removed or migrated.
- Git HEAD remains `e69a4ee47c4ebae960f41997354c28c0c6030537`; work is uncommitted.
- `git diff --check` passes.
- Parent has six unique children and all six child parent links agree.
  All seven `task.py validate` checks pass. They retain an existing warning:
  backend `quality-guidelines.md` is 72,317 bytes, over the 32,768-byte context
  injection cap. Future agents must read relevant sections explicitly; this
  warning is not evidence that the entire guideline was injected.

This integrity check does not close the mbx, native preview, native consumer
visual or whole-application idle gaps documented in `../result.md`.
