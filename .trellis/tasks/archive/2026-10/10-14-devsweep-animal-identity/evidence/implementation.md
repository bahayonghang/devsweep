# E asset implementation evidence

Status: asset implementation and deterministic packaging verified; native final
acceptance is NOT_RUN. No task completion, commit, archive or journal was done.

Changed product files: compact webview master, all sixteen existing native icon
files, `desktop/scripts/prepare-brand-icons.mjs`, and `desktop/branding/`
(full character, exact prompts, provenance README and generated manifest).
No AppShell, tray, stylesheet or Cargo.toml edit was required. D can retain the
existing compact master import. Semantic glyphs remain owned by D.

The parent generated full v1, compact v1 and compact v2 with built-in imagegen.
This implementation inspected full v1/compact v2 and the existing size review,
then packaged compact v2. It performed no artistic transformation. The full
1254px mark is retained outside the application bundle. The original sixteen
native files and webview master were copied before replacement to
`dist/brand-icons-e/prior/`; exact hashes are in `prior-manifest.json`.

Validation:

- Tauri CLI 2.11.4 export succeeded with Node 22.23.2.
- `inspection.json`: master plus sixteen final files decoded; alpha, sizes,
  ICO/ICNS frame inventories, meaningful bounds and edge alpha retained.
- `actual-frames-light-dark.png`: inspected actual small ICO frames and 20px
  resized master on white, dark green and navy. No visible forehead blemish,
  lost face/teeth or obvious halo was observed. Outer alpha 1 and almost-opaque
  interior alpha remain explicitly documented, not silently repaired.
- First double-export revealed ICNS record ordering differences only. Every
  encoded chunk hash matched. The packaging script now orders container records
  deterministically; `reproducibility.json` proves ten decoded frame sizes keep
  identical RGBA pixels and both generated manifests match.
- `dist/brand-icons-e/prepared-canonical` and `reproduced-canonical` matched all
  sixteen file hashes before promotion. No mobile or extra 64px output entered
  product sources.
- Existing decorative image accessibility is source-confirmed: empty alt and
  aria-hidden with adjacent DevSweep name. No duplicate name was introduced.
- Focused ESLint, Node syntax check and scoped `git diff --check` passed.
  Cargo.toml remained SHA-256
  `b7d6f6a30067850f5c4b1f3afde6450667262eb57309d230ca05fe7a0dd84915`.

Reproduction:

```powershell
& 'C:\Users\lyh\AppData\Local\mise\installs\node\22.23.2\node.exe' desktop/scripts/prepare-brand-icons.mjs dist/brand-icons-new
python .trellis/tasks/10-14-devsweep-animal-identity/evidence/inspect-assets.py desktop/src/assets/devsweep-icon-master.png desktop/src-tauri/icons dist/brand-inspection-new
```

Native final app/taskbar/tray screenshots, installer verification and independent
review remain open for F. No new native build was run after product edits.
