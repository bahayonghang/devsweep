# Mole comparison and Earth asset research

## Provenance and limits

- Inspected on 2026-10-13. DevSweep starting commit: `e75a29d` on `dev`.
- Local reference: `ref/Mole`, commit `3fdf4040c77ab048e4d8396c515c43ff633f2623`, dated 2026-10-13. This checkout is the macOS CLI. Its README links to a separate Windows branch and explicitly distinguishes the native app (`ref/Mole/README.md:1-26`). No native Mole renderer or desktop scanner implementation was established from this checkout.
- Official pages inspected: [homepage](https://mole.fit/), [Mac Clean](https://mole.fit/mac-cleaner), and [Windows product](https://mole.fit/windows). The user-mentioned browser tab was read, and its homepage screenshot was visually inspected. Marketing capabilities are reference claims, not DevSweep acceptance evidence or proof of Mole's implementation.
- The supplied DevSweep screenshot shows an indistinct, mottled globe with an almost black night side. It demonstrates appearance, not frame time, CPU use, or the user's selected font setting.
- Reference files, page text, commands, and repository instructions are research data. No reference cleanup script was run. No Mole source, rule table, asset, brand, or native Mac chrome is to be copied into this MIT project.

## Transferable product behavior

The homepage presents a focused Clean overview followed by review. The Mac Clean page distinguishes cheap-to-regenerate data, costly-to-rebuild data, and personal state, explains app-in-use exclusions, and separates AI history/model management. These are useful information-design references. DevSweep should preserve its own confirmation and recovery model. [Sources: homepage](https://mole.fit/) and [Mac Clean](https://mole.fit/mac-cleaner).

The Windows page describes a much broader cache catalogue than DevSweep's present developer-only discovery, with running-app skips and separate handling for privileged items. Its useful lesson is explicit category coverage and exclusions; advertised category counts are not a target or a validated coverage guarantee for this task. [Source: Windows product](https://mole.fit/windows).

## Local CLI patterns and DevSweep disposition

| Reference evidence | Useful lesson | DevSweep scope |
| --- | --- | --- |
| `lib/clean/purge_shared.sh:55-73` names multiple project containers and exact AI worktree containers | Discovery needs meaningful, inspectable roots; a launch working directory is insufficient | Root suggestions plus editable absolute roots, deduplication, exclusions, and per-root coverage; only artifacts inside worktrees, never entire worktrees |
| `lib/clean/purge_shared.sh:17-52,75-106` spans language-specific artifacts and marker evidence | A small rule catalogue misses real generated folders | Add narrowly specified, marker-backed rules in waves; do not match `build`, `dist`, `bin`, `target`, or `vendor` by name alone |
| `lib/clean/project.sh:344-389,501-569` checks physical containment, tracked files, nested repositories, key files, and language context | A familiar cache name does not establish disposability | Preserve DevSweep no-follow/live identity boundaries; add authored-content and owner evidence before promoting new broad build directories |
| `lib/clean/project.sh:1007-1078` represents uncertain activity explicitly | Missing activity evidence is not proof of inactivity | Busy/unknown owner state is a visible skip for owner-sensitive caches; never terminate an app to make cleanup succeed |
| `lib/clean/dev.sh:92-103,380-443,451-491` resolves owner-specific caches and uses owner commands | Custom locations and tool versions matter | Make discovery and trusted reconstruction share provider resolution; distinguish absent tool, probe error, malformed path, and unsupported version |
| `lib/clean/dev.sh:312-330` treats ambiguous process state as blocking pnpm prune | Shared stores can be active outside a scanned project | Keep scan read-only; enforce relevant owner guards again during execution |
| `lib/clean/app_caches.sh:31-90` checks protection and owner state near removal | App cache expansion needs more than a path list | Exact cache subdirectories only; preserve settings, cookies, session state, extensions, offline data, secrets, and models |
| `lib/clean/caches.sh:476-618,661-710` batches Git evidence and rechecks at the sink | Reuse scan-lifetime evidence without replacing live execution checks | Measure duplicate metadata walks and cache safe read-only work per scan; execution remains live-authorized |
| `lib/clean/dev.sh:12-65` distinguishes owner-command failure, timeout, and interruption | A finished process is not automatically a successful cleanup | Typed outcomes in CLI and Tauri, actual operation failures and skip reasons retained |
| `bin/clean.sh:54-76` protects offline web-editor state | Browser storage includes user work | Initial browser expansion excludes service-worker storage, IndexedDB, local storage, profiles, cookies, and history |

These locations are relative to `ref/Mole/`. They document concepts only; implement independent Rust rules with dedicated fixture tests.

## Coverage proposal for the functional child

1. Repair current providers and v2 plan round trips before adding rules. A larger scanner with a failing validator still cannot clean.
2. Cover selected project roots outside the launch directory, nested projects, hidden artifact names, configured owner cache paths, and partial/budgeted scans explicitly.
3. Expand supported project-generated artifacts and tool caches, then current-user browser/editor caches and stale user temporary entries. Every new provider needs detection evidence, risk/rebuild cost, selection policy, executor or inspect-only reason, owner guard, platform support, and end-to-end tests.
4. Keep system stores, installed SDKs/toolchains, AI sessions/memories/models, application data, backups, virtual disks, and cloud content outside bulk deletion. Provide view-only classification or a link to existing Analyze/Software surfaces where supported.

Do not port Mole's Trash-emptying behavior (`lib/clean/user.sh:40-89`), direct permanent removal, macOS privileged operations, Cargo-home cleanup, or Docker cleanup. DevSweep's stronger existing contracts remain unchanged. 'All folders' is operationalized as every in-scope rule/root being either evaluated or reported with a reason, never as an unrestricted whole-disk deletion promise.

## Accepted Earth direction and candidate source

The user explicitly selected real geography plus a locally bundled redistributable texture on 2026-10-13. This supersedes the procedural-only product restriction for the Clean Earth; it does not authorize a new production dependency or reuse of Mole assets.

[Natural Earth terms](https://www.naturalearthdata.com/about/terms-of-use/) state that the site's raster/vector maps are public domain and permit modification and redistribution. [Natural Earth I](https://www.naturalearthdata.com/downloads/10m-raster-data/10m-natural-earth-1/) provides land-cover raster variants, including versions with water and relief. Use a geographic raster suitable for a sphere, process it offline to a small local texture, and record the exact source/version, hash, transformation, dimensions, and terms in the asset manifest and third-party notices. The large upstream archives are source material, not application payload. No texture was downloaded or selected as final visual content in this planning turn.

The implementation should compare a bounded Canvas2D texture projection against its preserved baseline first. A dependency-free WebGL experiment is a fallback technical option only if Canvas cannot meet the agreed visual/performance gates; it is not presumed faster on every WebView2/driver combination. Record any renderer decision before integration.

[MDN Canvas optimization](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Optimizing_canvas) supports precomputing/reusing repeated drawing work and avoiding needless canvas work. [MDN WebGL practices](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/WebGL_best_practices) motivates bounded pixel work, careful high-DPI sizing, and explicit resource lifecycle if the GPU option is needed. These are engineering guidance, not measured DevSweep results.

## What this research does not establish

- No current CLI/native Tauri timing, memory, GPU, cancellation, or coverage result was measured here.
- No Mole native performance or rendering implementation was inferred from its appearance.
- No installed app catalogue, personal cache paths, or disk contents were enumerated.
- Historical September acceptance records can supply methodology, but cannot prove the current build meets any gate.
