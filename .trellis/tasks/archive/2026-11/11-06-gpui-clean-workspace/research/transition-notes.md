# Transition and layout traps found before implementation

- `shell.rs::clean_choose_scope(Project)` currently calls request_project_scan.
  A passive scope tab must not reuse that dispatch behavior.
- `shell.rs::clean_pick_scan_root` currently persists the chosen root and then
  calls request_project_scan. The approved workspace separates choosing a folder
  from the explicit Scan action; selecting a folder must not auto-scan either.
- `shell.rs::clean_enter` currently walks None -> ScopeChoice -> scan prompts.
  Update keyboard behavior with the visible UI; do not leave a hidden wizard.
- `clean_review` currently stacks stat tiles and a 640x260 disk map before the
  target list. Merely improving landing does not satisfy populated-first-screen
  acceptance. Put task controls and real rows first and allocate remaining
  viewport to review; secondary disk-map context must not push them offscreen.
- Keep the existing global confirmation token, explicit validated project roots,
  operation single-flight, preview invalidation and cancellation semantics.
- Use constrained native layouts; a fixed 640px map must not overflow the narrow
  content region at 125% text scale. Do not remove shared assets only because
  Clean landing no longer shows Earth.
