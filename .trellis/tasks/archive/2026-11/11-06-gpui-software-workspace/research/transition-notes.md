# Software transition traps

- Both Session::show_mode and Session::apply_key mode changes call
  ensure_workbench_rows. Default Uninstall initialization should occur in the
  transition layer once, not on every render or every workbench Enter.
- Session::close_nested_review currently closes any open Software review even
  when another mode owns Escape. A persistent Software segment must not be
  closed as a side effect of Escape in Optimize/Analyze/Settings. Gate actions
  by the active mode/surface and clear only contextual pending review as needed.
- Current software_open_segment closes and reopens even the same segment.
  Make repeated activation a no-op; explicit Refresh owns reloading.
- Host/fixture paths differ. Keep Session::unbacked tests isolated and preserve
  injected fixture priority; don't add incidental host inventory reads to tests.
- A global job generation exists; switching segment must invalidate the old
  pending result even when the destination resolves from a synchronous fixture.
- Initial inventory must stay read-only, and merely opening Software must not
  probe winget. Startup/Updates get appropriate filters rather than inheriting
  uninstall-only Removable state.
