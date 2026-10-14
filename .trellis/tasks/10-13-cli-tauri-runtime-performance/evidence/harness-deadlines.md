# Owned-process harness deadline revision

`tools/measure-readonly.py` now writes version 2 summary metadata. CLI workload
deadline defaults to 900 seconds (the old fixed 180 seconds could not cover A's
1,000-target fixture); `--deadline` overrides it explicitly. Values must be
finite, positive and at most one day. Desktop defaults to duration + 30 seconds
and requires time after its normal close request. Exact limits appear in launch
and summary files.

At the deadline the harness records elapsed time and owned PID/creation values,
then uses held native process handles to recheck each creation identity before
failure teardown. A changed identity is never terminated. It waits a bounded
five seconds and reports remaining processes. Timeout always means failed
measurement and failed lifecycle; teardown is not evidence of cooperative
cancel/join. The harness exits nonzero on timeout, unsuccessful process exit or
leftover owned processes. No process-name termination, cleanup or maintenance
workload exists in its closed workload list.

Desktop runs now explicitly choose visible (default) or hidden startup with
`--window-state`. Each native sample records the owned main window's visibility
and minimized status. Earlier SW_HIDE preparation samples remain preparation
only and are not mixed into visible-idle acceptance.

Added closed `software-arp`, `software-msi` and `software-msix` read-only
workloads for preserved A source profiling. The CLI's real `--help` confirmed
these source enum names. No product code or inventory semantics changed.

Validation: `python -m unittest discover -s tools/tests -p
test_measure_readonly.py -v` passes 8 tests on Windows. Native fixture tests
prove wrong creation identity leaves the owned sleeping child alive, exact
identity permits failed-fixture teardown, and a real 100 ms deadline is recorded
as failure with no remaining owned process. These fixture terminations are
deliberate harness tests, not application lifecycle acceptance.

Native post-exit settlement now observes recorded owned helpers for at most
five seconds after the host exits, sampling their normal exit without sending
termination. Summary records the limit, actual elapsed time and remaining
identities at each observation. This replaces a fixed 300 ms observation that
reported two WebView helpers still alive in `dist/native-clean-precheck-01`;
the coordinator's later identity checks proved both naturally exited. The
original failed summary is retained, not rewritten as a pass.

Samples include a decimal-string Unix nanosecond timestamp for correlation
with native UI request/ack/join logs. Main-window minimize/restore requires an
exact recorded HWND once a same-title HUD window exists; a regression test
rejects ambiguous or unrelated handles. HUD toggling addresses only the owned
`tray_icon_app` window using the callback defined by locked tray-icon 0.24.2;
actual visibility and sample observations remain required for lifecycle proof.
