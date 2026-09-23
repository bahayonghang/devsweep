# Safety and capability matrix

This matrix describes the shipped five-mode Windows product. It is not a
roadmap. Rejected actions stay rejected.

| Capability | Surface | Authority | Result | Notes |
| --- | --- | --- | --- | --- |
| Clean scan | CLI / TUI / Desktop | Observation only | Plan candidates + health | Dry-run by default. JSON is not executable. |
| Clean plan / preview | CLI / TUI / Desktop | Saved v2 plan + live digest | Digest, no side effects | Inspect-only rows cannot be selected. |
| Clean execute | CLI / TUI / Desktop | Plan + digest + `--confirm` | Trash or trusted command | Permanent delete is disabled. Recycle Bin emptying is outside DevSweep. |
| Software inventory | CLI / TUI / Desktop | Read-only tagged inventory | Partial/available sources | ARP/MSI remain manual. Last-used is unknown. |
| Software uninstall | CLI / TUI / Desktop | Exact current-user MSIX + digest + confirm | Closed five terminals | No MSI execute, no vendor strings, no elevation. |
| Software update check | Desktop | Read-only `winget upgrade --disable-interactivity` through `ProcessRunner` | Rows or `unavailable` + stable reason | No agreement flag, no upgrade run. Unknown output is never an empty success. |
| Software startup toggle | Desktop | Current-user entry + explicit switch | Re-read state + Software audit record | Writes only the current-user `Explorer\StartupApproved` value. HKLM rows are view-only. No entry is deleted. |
| Software leftover move | Desktop | Succeeded uninstall audit + leftover plan digest + confirm | Moved to the Recycle Bin + audit record | Name matches start unselected. Protected paths and system roots are never candidates. No permanent delete. |
| Optimize list / preview | CLI / TUI / Desktop | Closed eight-id catalogue | Digest for executable ids | Guidance cannot be planned. |
| Optimize DNS flush | CLI / TUI / Desktop | `dns.flush` digest + confirm | Succeeded / failed / unknown | Native System32 `ipconfig /flushdns` only. |
| Optimize Settings launch | CLI / TUI / Desktop | Frozen URI + digest + confirm | **Launched**, not completed | Storage / Search / Energy only. OS Settings lifetime is not a DevSweep total. |
| Analyze scan | CLI / TUI / Desktop | Read-only walker | Snapshot, never a cleanup plan | 250,000-node / 256 MiB accounted cap, two workers. |
| Status snapshot / live | CLI / TUI / Desktop | Read-only collectors | Available / partial / unsupported | GPU, VRAM, thermal, fan, SMART, physical-disk activity are unsupported. |
| History / Protection / Rules | CLI / TUI / Desktop | Inspect or explicit protect mutation | Redacted history | Unknown audit versions are preserved, not replayed. |
| Docker cleanup | None | Rejected | Absent | Not an MVP command. |
| Cargo home cleanup | Clean inspect-only | No cleanup action | Inspect | Credentials, `bin`, and registry internals are never targets. |
| Elevation / UAC | None | `asInvoker` | No consent UI | Machine MSI and protected products stay inventory/manual. |
| Compatibility aliases | None | Rejected | Exit 2 | `tui`, `scan`, `inventory`, `protect`, `rules` are unknown. |

Provenance relative to other public cleanup tools is recorded in
[`docs/provenance.md`](provenance.md). Rollback is the last accepted local
release; this matrix is not a publishing or signing checklist.
