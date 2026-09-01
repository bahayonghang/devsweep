# Windows Status Boundary

## Decision Summary

Status is a bounded, read-only, on-demand collector. It uses documented Win32
APIs through the existing `windows-sys` dependency, with added feature flags but
no new monitoring crate in the initial design.

## Supported Metrics

- system CPU from two `GetSystemTimes` samples;
- physical memory from `GlobalMemoryStatusEx`;
- fixed-volume capacity from logical-drive/type/free-space APIs;
- per-interface network rates from `GetIfTable2` counter deltas;
- battery/power status from `GetSystemPowerStatus`;
- process identity, CPU, working set/private bytes, and I/O counters from
  ToolHelp and documented process APIs.

GPU utilization, GPU memory as a whole-system value, thermal sensors, fan RPM,
SMART status, and physical disk activity are unsupported/unavailable in v1.
PowerShell, WMI, WMIC, typeperf, nvidia-smi, and vendor tools are not default
collectors.

## Bounds

- one snapshot: 500 ms rate window;
- live: default 2 s, minimum 1 s, maximum 60 s;
- processes refreshed every 4 s; volumes/battery every 30 s;
- 15 processes by default, 100 maximum returned, 4096 maximum enumerated;
- 150 ms cooperative process-detail budget;
- one sample in flight; skipped ticks are not backfilled;
- leave/cancel/channel-close joins the collector;
- live is mutually exclusive with other heavy foreground modes;
- no service, tray, scheduler, notification, persistent history, or health score.

## Output

Every metric is tagged `available`, `partial`, `unavailable`,
`permission_denied`, or `unsupported`. JSON is one snapshot; NDJSON is explicit
watch output. Broken pipe stops collection quietly. The UI retains at most 60
in-memory points and drops them on navigation.

## Primary References

- [GetSystemTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getsystemtimes)
- [GlobalMemoryStatusEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex)
- [GetIfTable2](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getiftable2)
- [GetSystemPowerStatus](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getsystempowerstatus)
- [Process enumeration](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-createtoolhelp32snapshot)
- [Process memory](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-getprocessmemoryinfo)
