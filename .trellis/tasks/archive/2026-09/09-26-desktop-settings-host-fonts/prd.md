# Installed Host Fonts and Preference V2

## Goal and scope

Let each Windows DevSweep installation select installed local font families while preserving saved settings. Own parent R3, V2 migration in R5, and the font part of main/HUD consistency.

Approved for sequential implementation on 2026-09-26. Parent: ../09-26-desktop-settings-experience/. Read the parent PRD, design, research, and approval record before this child. The user approved @base-ui/react and the Windows-only windows crate DirectWrite binding, and authorized the documented task order.

Current evidence: parent research/repository-findings.md E5-E8, E11, and E13. The current font family is a three-value enum. The Windows desktop MVP defines the supported host scope.

## Requirements

- F1: Read the current host's installed font-family catalogue, including user-installed families exposed by the OS. Do not impose a hardcoded preset or result-count ceiling.
- F2: Provide search by family/localized alias, explicit selection, preview, refresh, and system fallback. Distinguish empty, loading, failed, and missing-saved-font states.
- F3: Apply the committed selected family to actual main/HUD UI text and controls with Chinese/English fallbacks. Preserve technical monospace and text-scale behavior.
- F4: Introduce a typed V2 font preference and migrate valid V1 values without writes on read or loss of unrelated preferences.
- F5: Keep native font discovery read-only, main-window-scoped, and independent from cleanup authority and sampling.

## Acceptance criteria

- [ ] F-AC1 (F1): Native evidence records catalogue count and selects families beyond the old three presets. First/middle/last and available per-user families can be found. Face variants do not create duplicate family options.
- [ ] F-AC2 (F2): A 2,000-family fixture remains searchable and fully reachable; typing does not invoke native enumeration. Empty results, refresh failure, and late/disposed responses preserve a usable picker and committed value.
- [ ] F-AC3 (F2, F3): Selection survives restart and updates main/HUD after commit. A removed or unavailable saved family retains its stored name, shows a notice, and renders through the system fallback. A family with Chinese text, quotes, or a backslash cannot inject CSS.
- [ ] F-AC4 (F4): Each V1 font preset migrates correctly, and all other V1 fields remain equal. Reading alone creates no V2 file. First successful save creates V2 and leaves V1/language bytes unchanged.
- [ ] F-AC5 (F4): Valid V2 takes precedence. Invalid/future/unreadable V1 or V2 follows the parent failure matrix without overwriting files. Concurrent first saves retain both patches. Replacement failure preserves old bytes and emits no commit.
- [ ] F-AC6 (F3, F4): Out-of-order events, pending reads, reload, and HUD show follow the existing sequence rules. Resets affect only their documented group.
- [ ] F-AC7 (F5): HUD cannot invoke font discovery or preference update. The command returns no paths/bytes and downloads/installs nothing. Unsupported platforms return an explicit unavailable state while shared Rust code remains buildable.

## Exclusions and dependencies

Controls child first; The target-specific Windows binding was approved on 2026-09-26. No remote hosts, font installation/import/download, cloud fonts, style/weight editor, or new OS support. Font catalogue access never starts Status/HUD sampling. Palette child follows and extends the final V2 theme catalogue. Do not release the intermediate schema separately.
