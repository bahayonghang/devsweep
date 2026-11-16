# Software native acceptance

Date: 2026-11-15. Platform: Windows. Tested the release produced by canonical
`just gpui`, SHA-256
`d824781cc18d387dad9a191916b211d63e7f6b77db2f24c59910efaa9be58e37`.

## Method and screenshots

Used `@oai/sky` to operate only the owned DevSweep window. The launcher uses
isolated preferences/APPDATA and the normal native window size; all captured
images are 1242 by 821 pixels. No global appearance settings were changed.
The four list cases use the real read-only inventory: 1,220 entries, 89 eligible
for review. Related-directory cases use explicitly named isolated fixtures.

| Evidence | Source and observation |
| --- | --- |
| [English dark list](software-en-dark-list.jpg) | Real inventory; NVIDIA and A HUB resource icons, publisher/source, qualified size and servicing date |
| [English dark sort](software-en-dark-sort.jpg) | Name/size/installed-or-serviced selector, with Calculator search retained |
| [English dark filter](software-en-dark-filter.jpg) | Calculator search + MSIX source + eligible-for-review filter + date sort |
| [Chinese light list](software-zh-light-list.jpg) | Real inventory; localized row evidence and summary |
| [Chinese light sort](software-zh-light-sort.jpg) | Three localized sort choices |
| [Chinese light filter](software-zh-light-filter.jpg) | A HUB search + registry source + date sort; unknown dates remain unknown |
| [English light list](software-en-light-list.jpg) | Real inventory with the same evidence in light theme |
| [Chinese dark list](software-zh-dark-list.jpg) | Real inventory in Chinese dark theme |
| [English related directories](software-en-dark-related.jpg) | Fixture: missing/UNC icon avatars, invalid date unknown and last, one read-only directory candidate |
| [Chinese related directories](software-zh-light-related.jpg) | The same isolated candidate and warning in Chinese |

Only referenced screenshots are retained by `evidence/.gitignore`. Raw inventory,
preferences, probe/build logs and disposable fixture data remain under ignored
`runtime/`. `native-launch.ps1` and `native-fixture.py` locate the repository
independently of the task's active/archive directory depth.

## Observed behavior

- ARP resource icons rendered, including NVIDIA, A HUB, ASUS Mouse HAL and
  Floorp. Unknown dates/sizes remained explicit; estimates kept their qualifier.
  Source names and spoken row labels used localized presentation, with internal
  identity strings confined to automation IDs.
- Sorting Calculator results by date placed Windows Calculator (2026-08-29)
  before Occupancy Calculator (2026-05-29). Search plus ARP/MSI/MSIX filters
  yielded 1/0/1 results respectively; adding eligible-for-review retained the
  correct MSIX entry. Chinese search/source/date combinations also worked.
- Both missing-local and UNC fixture icons stayed as letter avatars. The
  malformed saved date displayed unknown, and date sorting kept it after the
  valid leap-day entry. Native observations complement the lexical rejection
  tests; no network image reader exists in this route.
- Focusing the fixture row exposed Related directories and a separate Review
  button. Only Related directories was activated. The panel showed one 36 B
  candidate, warned that it may still be in use, and stated nothing was selected
  for removal. Switching focus cleared the old panel. English and Chinese
  cases each retained exactly the same three isolated files and SHA-256 values
  (two preference documents and a sentinel); no audit directory appeared.
- No uninstall, update, startup change, process termination, cleanup or registry
  backup was invoked. Owned windows were closed normally with Alt+F4; their
  process IDs were absent afterwards.

## Responsiveness and timing limits

English dark showed the loading view in the first capture at 244 ms after the
Software click. The first follow-up capture with rows was at 8,825 ms. Chinese
light showed loading at 531 ms and rows in the follow-up at 9,193 ms. These are
sampled observations, not measurements of the exact inventory-completion time.

Calculator search plus capture took 333 ms. Two wheel actions on the real
Chinese list visibly moved and returned its contents; action plus capture took
282 ms and 266 ms. No input stall or hang was observed. These times include
automation/capture overhead and do not measure frame rate or prove a prolonged
scrolling stress test.

The captures did not isolate a populated text-only frame before the first
successful icon. Text-first scheduling is verified by the Session/cache tests
and source review: rows install before icon jobs and rendering performs no icon
I/O. Native captures establish real icon output and usable interactions, not
frame-accurate first-icon latency.

## MSIX icon limitation

Calculator and 1Remote remained avatars despite valid Logo location evidence.
A read-only probe using the loader's flags failed at the WindowsApps parent
directory with ERROR_ACCESS_DENIED (5), before opening or decoding the PNG.
Original FILE_READ_ATTRIBUTES and metadata-only zero desired access both failed;
Calculator scale-100/125 paths were checked.

The approved policy requires no-reparse checks and retained parent handles.
That refusal therefore keeps the avatar. No parent check, ACL, sharing policy
or elevation was changed. This is explicitly **not** evidence of successful
MSIX bitmap rendering. Core/CLI still expose the optional MSIX Logo/date
evidence, and ARP bitmap rendering succeeded. The independent reviewer confirmed
this unavailable-path fallback is consistent with the agreed contract.

## Coverage boundary

Software English/Chinese, dark/light, sorting/filtering, native fallback and
read-only fixture preview were exercised. This is not the parent task's final
five-mode/tray screenshot matrix. No OS/DPI/hotplug matrix or frame trace was
performed. Core/CLI fixtures and 204 GPUI tests cover the associated malformed,
generation, cache, no-write and cross-segment regressions.
