# Release-log follow-up and AI cleanup scope

Research date: 2026-10-15. Added after the user explicitly requested release-log
coverage, Claude Code cleanup and current library versions.

## Source identities and limits

| Source | Observed release/channel | Relevant evidence |
| --- | --- | --- |
| [User-linked X post](https://x.com/HiTw93/status/2109632109136818245) | User describes it as today's AI-cleanup announcement | Direct fetch returned HTTP 403. Exact post contents/date/platform remain UNVERIFIED; no quoted post text. |
| [Mac desktop releases](https://mole.fit/releases) | v1.16.0 dated 2026-10-12; v1.17.0 Preview build 328 has no displayed release date | Dedicated AI review covers caches/versions/sessions/worktrees/tools, active-data protection and recovery; preview adds Claude scratch worktrees, idle temp files and lower session-scan memory. |
| [Windows desktop releases](https://mole.fit/windows/releases) | 0.12.0 dated 2026-10-15; AI cache entry at 0.4.0 dated 2026-10-06 | Windows feature history differs from the Mac AI page. Today's entries concern Analyze, app ownership, status, startup and update behavior. |
| [Windows CLI V1.34.0-windows](https://github.com/tw93/Mole/releases/tag/V1.34.0-windows) | Release page displays 14 Oct 07:47, commit 6f3fba4 | Large-folder progress, recoverable/rechecked leftovers, unreadable exclusions fail closed, preserved offline data, preview/execution parity and selection freshness. |
| [Mac CLI V1.59.1](https://github.com/tw93/Mole/releases/tag/V1.59.1) | Release page displays 10 Oct 08:23, commit fe32f99 | Continued progress while checking services and explicit skipped-permission reasons. |
| [CLI release index](https://github.com/tw93/Mole/releases) | Windows and Mac tags coexist | Read per-platform tags, not one globally sorted version number. |

The exact X text could not be verified. Official logs independently justify the
AI capability work requested by the user. Mac behavior is an inspiration for a
Windows-native design, never evidence that the same Windows paths/APIs exist.

## Additions to the implementation backlog

| Requirement | DevSweep response | Owner |
| --- | --- | --- |
| AI cleanup beyond generic editor caches | Dedicated provider inventory and CLI-first AI review | C15 |
| Old tools versus tool caches | Distinct classes; current tool version/credentials stay protected; uninstall uses Software provider | C15,C04,C05 |
| Sessions and worktrees | Explicit opt-in review with recovery/identity and active-use evidence, not general cache selection | C15 |
| Large AI histories | Bounded discovery, incremental progress, cancellation and memory tests | C15,C14 |
| Hidden/unreadable disk content and drive aliases | Preserve denied rows; enumerate real volumes and qualify subst/hidden aliases | C07 |
| Application ownership | Same vendor is not same owner; uninstall/recycle rechecks actual sharing | C04,C05 |
| Named telemetry | Identify measured drive/GPU/network adapter; qualified per-drive totals | C08 |
| Repair cost | Health check and long/network repair are separate reviewed operations | C06 |
| Useful concurrency | Permit nonconflicting read/toggle work; protect exact resources during update/uninstall | C02,C10 |
| Result honesty | Separate skipped, failed, pending, unknown and recovered outcomes | C01,C10–C13 |

These are proposed product contracts derived from multiple evidence streams.
No automatic update watcher, recurring task or live user cleanup is created.

## C15 data classes

1. **Caches/temp/diagnostic artifacts:** known provider paths, bounded age/use checks,
   explained regeneration cost. New or unknown layouts remain inspect-only.
2. **Superseded native/tool versions:** discover current and referenced versions;
   keep active/pinned/unknown versions and use the provider's supported mechanism.
3. **Conversation/session data:** never default-select. Explicit selected scope,
   content/metadata recovery and provider-index consistency precede mutation.
4. **Worktrees:** idle age does not prove dispensability. Dirty, untracked, ignored,
   unpushed, active, shared or unrecognized repositories remain protected.
5. **Models/settings/credentials/memory/skills/MCP config:** retained data; report
   occupancy where useful, never treat them as disposable cache.
6. **Unused tool removal:** separate Software review, installed identity, dependency
   checks and post-uninstall requery. No broad AI-home/bin-folder deletion.

C15 must verify current provider documentation/source, Windows install modes,
custom roots and supported format versions before executable rules. This planning
does not claim those paths or recovery mechanisms have already been proven.

