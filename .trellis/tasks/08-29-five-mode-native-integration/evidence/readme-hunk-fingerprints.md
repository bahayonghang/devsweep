# Pre-start README hunk fingerprints

Recorded before any README edit for `08-29-five-mode-native-integration`.
User-owned dirt must not be rewritten or absorbed.

## `README.md`

`git diff -- README.md` at start contained **one** hunk in Desktop Development:

```
@@ -58,13 +58,14 @@ npm ci
 npm run tauri -- dev
 ```
 
-From the repository root, run the desktop frontend and Rust checks or build an
-unsigned NSIS installer:
+From the repository root, run the desktop frontend and Rust checks, build an
+unsigned NSIS installer, or silently install that installer:
 
 ```powershell
 just desktop-web-check
 just desktop-test
 just desktop-build
+just tinstall
 ```
 
 The desktop app uses the same core validation, confirmation digest, and
```

- HEAD blob: `README.md` at `fbb080b` (3572 bytes, 114 lines).
- Working-tree context: after the `npm run tauri -- dev` fence, before
  "The desktop app uses the same core validation...".
- Working-tree lines: 61–69 (the `just tinstall` addition).

This task added a **separate** `## Five-mode product` section between
Documentation and Validation. It did not edit the tinstall hunk.

## `justfile`

Pre-start dirt. This task did **not** edit `justfile` or `.trellis/.gitignore`.
